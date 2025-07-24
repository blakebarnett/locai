//! Authentication service using storage layer abstractions

use std::collections::HashMap;
use uuid::Uuid;
use serde_json::Value;
use chrono::{DateTime, Utc};

use locai::{
    core::MemoryManager,
    storage::{models::Entity, filters::EntityFilter},
};

use crate::{
    api::auth::{hash_password, verify_password, generate_jwt_token},
    error::{ServerError, ServerResult},
};

/// User data structure
#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Convert from Entity to User
    pub fn from_entity(entity: Entity) -> Result<Self, ServerError> {
        let id = Uuid::parse_str(&entity.id)
            .map_err(|e| ServerError::Internal(format!("Invalid user ID: {}", e)))?;
        
        let username = entity.properties.get("username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::Internal("Missing username field".to_string()))?
            .to_string();
        
        let password_hash = entity.properties.get("password_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::Internal("Missing password_hash field".to_string()))?
            .to_string();
        
        let role = entity.properties.get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::Internal("Missing role field".to_string()))?
            .to_string();
        
        let email = entity.properties.get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let created_at = entity.properties.get("created_at")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        
        let updated_at = entity.properties.get("updated_at")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        
        Ok(User {
            id,
            username,
            password_hash,
            role,
            email,
            created_at,
            updated_at,
        })
    }
    
    /// Convert User to Entity
    pub fn to_entity(&self) -> Entity {
        let mut properties = HashMap::new();
        properties.insert("username".to_string(), Value::String(self.username.clone()));
        properties.insert("password_hash".to_string(), Value::String(self.password_hash.clone()));
        properties.insert("role".to_string(), Value::String(self.role.clone()));
        properties.insert("created_at".to_string(), Value::String(self.created_at.to_rfc3339()));
        properties.insert("updated_at".to_string(), Value::String(self.updated_at.to_rfc3339()));
        
        if let Some(email) = &self.email {
            properties.insert("email".to_string(), Value::String(email.clone()));
        }
        
        Entity {
            id: self.id.to_string(),
            entity_type: "user".to_string(),
            properties: Value::Object(properties.into_iter().collect()),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Authentication service
#[derive(Debug)]
pub struct AuthService {
    jwt_secret: String,
    jwt_expiration_hours: u64,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret,
            jwt_expiration_hours: 24, // Default 24 hours
        }
    }
    
    /// Initialize the authentication system
    pub async fn initialize(&self, memory_manager: &MemoryManager, root_password: Option<String>) -> ServerResult<()> {
        tracing::info!("Initializing authentication system using storage layer");
        
        // Check if root user exists
        if let Some(_root_user) = self.get_user_by_username(memory_manager, "root").await? {
            tracing::info!("Root user already exists");
            return Ok(());
        }
        
        // Create root user
        let was_provided = root_password.is_some();
        let password = root_password.unwrap_or_else(|| {
            use crate::api::auth::generate_root_password;
            generate_root_password()
        });
        
        let _user = self.create_user(memory_manager, "root", &password, "root", None).await?;
        
        tracing::info!("Root user created successfully");
        tracing::info!("Root username: root");
        tracing::info!("Root password: {}", password);
        
        if !was_provided {
            tracing::warn!("IMPORTANT: Save the root password above! Set LOCAI_ROOT_PASSWORD environment variable to customize it.");
        }
        
        Ok(())
    }
    
    /// Create a new user
    pub async fn create_user(
        &self,
        memory_manager: &MemoryManager,
        username: &str,
        password: &str,
        role: &str,
        email: Option<String>,
    ) -> ServerResult<User> {
        // Check if username already exists
        if let Some(_existing) = self.get_user_by_username(memory_manager, username).await? {
            return Err(ServerError::Validation(format!("Username '{}' already exists", username)));
        }
        
        // Hash the password
        let password_hash = hash_password(password)?;
        
        // Create user entity
        let user = User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            password_hash,
            role: role.to_string(),
            email,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Store in database
        let entity = user.to_entity();
        let storage = memory_manager.storage();
        storage.create_entity(entity).await
            .map_err(|e| ServerError::Database(format!("Failed to create user: {}", e)))?;
        
        Ok(user)
    }
    
    /// Get user by username
    pub async fn get_user_by_username(&self, memory_manager: &MemoryManager, username: &str) -> ServerResult<Option<User>> {
        let storage = memory_manager.storage();
        
        // Use entity filtering to find user by username
        let filter = EntityFilter {
            entity_type: Some("user".to_string()),
            ..Default::default()
        };
        let entities = storage.list_entities(Some(filter), None, None).await
            .map_err(|e| ServerError::Database(format!("Failed to query users: {}", e)))?;
        
        for entity in entities {
            if let Ok(user) = User::from_entity(entity) {
                if user.username == username {
                    return Ok(Some(user));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Get user by ID
    pub async fn get_user_by_id(&self, memory_manager: &MemoryManager, user_id: &Uuid) -> ServerResult<Option<User>> {
        let storage = memory_manager.storage();
        
        match storage.get_entity(&user_id.to_string()).await {
            Ok(Some(entity)) => {
                if entity.entity_type == "user" {
                    Ok(Some(User::from_entity(entity)?))
                } else {
                    Ok(None)
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ServerError::Database(format!("Failed to get user: {}", e))),
        }
    }
    
    /// Authenticate user and return JWT token
    pub async fn authenticate(
        &self,
        memory_manager: &MemoryManager,
        username: &str,
        password: &str,
    ) -> ServerResult<(String, User, i64)> {
        // Get user by username
        let user = self.get_user_by_username(memory_manager, username).await?
            .ok_or_else(|| ServerError::Auth("Invalid username or password".to_string()))?;
        
        // Verify password
        if !verify_password(password, &user.password_hash)? {
            return Err(ServerError::Auth("Invalid username or password".to_string()));
        }
        
        // Generate JWT token
        let (token, expires_at) = generate_jwt_token(
            &user.id,
            &user.username,
            &user.role,
            &self.jwt_secret,
            self.jwt_expiration_hours,
        )?;
        
        Ok((token, user, expires_at))
    }
    
    /// List all users
    pub async fn list_users(&self, memory_manager: &MemoryManager) -> ServerResult<Vec<User>> {
        let storage = memory_manager.storage();
        
        let filter = EntityFilter {
            entity_type: Some("user".to_string()),
            ..Default::default()
        };
        let entities = storage.list_entities(Some(filter), None, None).await
            .map_err(|e| ServerError::Database(format!("Failed to list users: {}", e)))?;
        
        let mut users = Vec::new();
        for entity in entities {
            if let Ok(user) = User::from_entity(entity) {
                users.push(user);
            }
        }
        
        Ok(users)
    }
    
    /// Update user
    pub async fn update_user(&self, memory_manager: &MemoryManager, user: User) -> ServerResult<User> {
        let storage = memory_manager.storage();
        
        let entity = user.to_entity();
        storage.update_entity(entity).await
            .map_err(|e| ServerError::Database(format!("Failed to update user: {}", e)))?;
        
        Ok(user)
    }
    
    /// Delete user
    pub async fn delete_user(&self, memory_manager: &MemoryManager, user_id: &Uuid) -> ServerResult<bool> {
        let storage = memory_manager.storage();
        
        storage.delete_entity(&user_id.to_string()).await
            .map_err(|e| ServerError::Database(format!("Failed to delete user: {}", e)))
    }
} 