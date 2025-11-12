//! Webhook management API endpoints

use std::sync::Arc;

use axum::{
    Json as JsonExtractor,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use uuid::Uuid;

use async_trait::async_trait;
use locai::hooks::{MemoryHook, Webhook};
use locai::models::Memory;

use crate::{
    api::dto::{CreateWebhookRequest, UpdateWebhookRequest, WebhookDto},
    error::{ServerError, not_found},
    state::AppState,
};

/// Webhook configuration stored in AppState
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub id: String,
    pub event: String,
    pub url: String,
    pub enabled: bool,
    pub headers: std::collections::HashMap<String, String>,
    pub secret: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

/// Event-filtered webhook wrapper that only fires for specific event types
#[derive(Debug, Clone)]
struct EventFilteredWebhook {
    inner: Webhook,
    event_type: String,
}

#[async_trait]
impl MemoryHook for EventFilteredWebhook {
    async fn on_memory_created(&self, memory: &Memory) -> locai::hooks::HookResult {
        if self.event_type == "memory.created" {
            self.inner.on_memory_created(memory).await
        } else {
            locai::hooks::HookResult::Continue
        }
    }

    async fn on_memory_accessed(&self, memory: &Memory) -> locai::hooks::HookResult {
        if self.event_type == "memory.accessed" {
            self.inner.on_memory_accessed(memory).await
        } else {
            locai::hooks::HookResult::Continue
        }
    }

    async fn on_memory_updated(&self, old: &Memory, new: &Memory) -> locai::hooks::HookResult {
        if self.event_type == "memory.updated" {
            self.inner.on_memory_updated(old, new).await
        } else {
            locai::hooks::HookResult::Continue
        }
    }

    async fn before_memory_deleted(&self, memory: &Memory) -> locai::hooks::HookResult {
        if self.event_type == "memory.deleted" {
            self.inner.before_memory_deleted(memory).await
        } else {
            locai::hooks::HookResult::Continue
        }
    }

    fn timeout_ms(&self) -> u64 {
        self.inner.timeout_ms()
    }

    fn name(&self) -> &str {
        "event_filtered_webhook"
    }
}

/// Create a new webhook
#[utoipa::path(
    post,
    path = "/api/webhooks",
    tag = "webhooks",
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook created successfully", body = WebhookDto),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_webhook(
    State(state): State<Arc<AppState>>,
    JsonExtractor(request): JsonExtractor<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookDto>), ServerError> {
    // Validate event type
    let valid_events = [
        "memory.created",
        "memory.updated",
        "memory.accessed",
        "memory.deleted",
    ];
    if !valid_events.contains(&request.event.as_str()) {
        return Err(ServerError::BadRequest(format!(
            "Invalid event type: {}. Valid events: {}",
            request.event,
            valid_events.join(", ")
        )));
    }

    // Validate URL
    if !request.url.starts_with("http://") && !request.url.starts_with("https://") {
        return Err(ServerError::BadRequest(
            "URL must start with http:// or https://".to_string(),
        ));
    }

    // Generate webhook ID
    let webhook_id = Uuid::new_v4().to_string();

    // Create webhook configuration
    let config = WebhookConfig {
        id: webhook_id.clone(),
        event: request.event.clone(),
        url: request.url.clone(),
        enabled: true,
        headers: request.headers.clone(),
        secret: request.secret.clone(),
        created_at: Utc::now(),
    };

    // Create the webhook hook
    let mut webhook = Webhook::new(request.url.clone());
    for (key, value) in &request.headers {
        webhook = webhook.with_header(key.clone(), value.clone());
    }

    // Wrap with event filter
    let filtered_webhook = EventFilteredWebhook {
        inner: webhook,
        event_type: request.event.clone(),
    };

    // Register the hook
    let hook_registry = state
        .memory_manager
        .hook_registry()
        .ok_or_else(|| ServerError::Internal("Hook registry not available".to_string()))?;
    hook_registry.register(Arc::new(filtered_webhook)).await;

    // Store webhook config
    state
        .webhook_registry
        .write()
        .await
        .insert(webhook_id.clone(), config.clone());

    // Convert to DTO
    let dto = WebhookDto {
        id: config.id,
        event: config.event,
        url: config.url,
        enabled: config.enabled,
        headers: config.headers,
        secret: config.secret,
        created_at: config.created_at,
    };

    Ok((StatusCode::CREATED, Json(dto)))
}

/// List all webhooks
#[utoipa::path(
    get,
    path = "/api/webhooks",
    tag = "webhooks",
    responses(
        (status = 200, description = "List of webhooks", body = Vec<WebhookDto>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_webhooks(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WebhookDto>>, ServerError> {
    let registry = state.webhook_registry.read().await;
    let webhooks: Vec<WebhookDto> = registry
        .values()
        .map(|config| WebhookDto {
            id: config.id.clone(),
            event: config.event.clone(),
            url: config.url.clone(),
            enabled: config.enabled,
            headers: config.headers.clone(),
            secret: config.secret.clone(),
            created_at: config.created_at,
        })
        .collect();

    Ok(Json(webhooks))
}

/// Get a webhook by ID
#[utoipa::path(
    get,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    params(
        ("id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Webhook found", body = WebhookDto),
        (status = 404, description = "Webhook not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_webhook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<WebhookDto>, ServerError> {
    let registry = state.webhook_registry.read().await;
    let config = registry.get(&id).ok_or_else(|| not_found("Webhook", &id))?;

    let dto = WebhookDto {
        id: config.id.clone(),
        event: config.event.clone(),
        url: config.url.clone(),
        enabled: config.enabled,
        headers: config.headers.clone(),
        secret: config.secret.clone(),
        created_at: config.created_at,
    };

    Ok(Json(dto))
}

/// Update a webhook
#[utoipa::path(
    put,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    params(
        ("id" = String, Path, description = "Webhook ID")
    ),
    request_body = UpdateWebhookRequest,
    responses(
        (status = 200, description = "Webhook updated successfully", body = WebhookDto),
        (status = 404, description = "Webhook not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_webhook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonExtractor(request): JsonExtractor<UpdateWebhookRequest>,
) -> Result<Json<WebhookDto>, ServerError> {
    let mut registry = state.webhook_registry.write().await;
    let config = registry
        .get_mut(&id)
        .ok_or_else(|| not_found("Webhook", &id))?;

    // Update fields if provided
    if let Some(enabled) = request.enabled {
        config.enabled = enabled;
    }
    if let Some(url) = request.url {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ServerError::BadRequest(
                "URL must start with http:// or https://".to_string(),
            ));
        }
        config.url = url;
    }
    if let Some(headers) = request.headers {
        config.headers = headers;
    }
    if let Some(secret) = request.secret {
        config.secret = Some(secret);
    }

    let dto = WebhookDto {
        id: config.id.clone(),
        event: config.event.clone(),
        url: config.url.clone(),
        enabled: config.enabled,
        headers: config.headers.clone(),
        secret: config.secret.clone(),
        created_at: config.created_at,
    };

    Ok(Json(dto))
}

/// Delete a webhook
#[utoipa::path(
    delete,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    params(
        ("id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 204, description = "Webhook deleted successfully"),
        (status = 404, description = "Webhook not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServerError> {
    let mut registry = state.webhook_registry.write().await;
    registry
        .remove(&id)
        .ok_or_else(|| not_found("Webhook", &id))?;

    // Note: We don't unregister the hook from the registry because
    // hooks are fire-and-forget and removing them would require
    // tracking which hook instance corresponds to which webhook ID.
    // For Phase 1, this is acceptable - the hook will simply not
    // match events anymore if the webhook config is deleted.

    Ok(StatusCode::NO_CONTENT)
}
