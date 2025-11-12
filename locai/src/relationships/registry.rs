//! Relationship Type Registry
//!
//! Provides a thread-safe, runtime-configurable registry for relationship types.
//! Supports dynamic registration of custom types while maintaining backward compatibility
//! with existing relationship enums.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error types for registry operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistryError {
    #[error("Relationship type already exists: {0}")]
    TypeAlreadyExists(String),

    #[error("Relationship type not found: {0}")]
    TypeNotFound(String),

    #[error("Invalid type name: {0}")]
    InvalidTypeName(String),

    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    #[error("Type in use, cannot delete: {0}")]
    TypeInUse(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Definition of a relationship type with metadata about its characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipTypeDef {
    /// Unique name for this relationship type
    pub name: String,

    /// Optional inverse relationship type name
    /// Example: "knows" might have inverse "known_by"
    pub inverse: Option<String>,

    /// Whether this relationship is symmetric (bidirectional)
    /// Example: "married_to" is symmetric
    pub symmetric: bool,

    /// Whether this relationship is transitive
    /// Example: "part_of" is transitive (if A is part of B and B is part of C, then A is part of C)
    pub transitive: bool,

    /// JSON Schema for validating metadata on relationships of this type
    pub metadata_schema: Option<Value>,

    /// Version of this type definition (for migration/compatibility)
    pub version: u32,

    /// When this type was registered
    pub created_at: DateTime<Utc>,

    /// Custom metadata about this type
    pub custom_metadata: HashMap<String, Value>,
}

impl RelationshipTypeDef {
    /// Create a new relationship type definition
    pub fn new(name: String) -> Result<Self, RegistryError> {
        if name.trim().is_empty() {
            return Err(RegistryError::InvalidTypeName(
                "Type name cannot be empty".to_string(),
            ));
        }

        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(RegistryError::InvalidTypeName(
                "Type name can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        Ok(Self {
            name,
            inverse: None,
            symmetric: false,
            transitive: false,
            metadata_schema: None,
            version: 1,
            created_at: Utc::now(),
            custom_metadata: HashMap::new(),
        })
    }

    /// Set the inverse relationship type
    pub fn with_inverse(mut self, inverse: String) -> Self {
        self.inverse = Some(inverse);
        self
    }

    /// Mark this type as symmetric
    pub fn symmetric(mut self) -> Self {
        self.symmetric = true;
        self
    }

    /// Mark this type as transitive
    pub fn transitive(mut self) -> Self {
        self.transitive = true;
        self
    }

    /// Set the metadata schema for this type
    pub fn with_metadata_schema(mut self, schema: Value) -> Self {
        self.metadata_schema = Some(schema);
        self
    }

    /// Add custom metadata
    pub fn with_custom_metadata(mut self, key: String, value: Value) -> Self {
        self.custom_metadata.insert(key, value);
        self
    }
}

impl Default for RelationshipTypeDef {
    fn default() -> Self {
        Self {
            name: "generic".to_string(),
            inverse: None,
            symmetric: false,
            transitive: false,
            metadata_schema: None,
            version: 1,
            created_at: Utc::now(),
            custom_metadata: HashMap::new(),
        }
    }
}

/// Thread-safe registry for relationship type definitions with optional persistence
#[derive(Clone, Debug)]
pub struct RelationshipTypeRegistry {
    types: Arc<RwLock<HashMap<String, RelationshipTypeDef>>>,
    storage: Option<Arc<dyn RelationshipTypeStorage>>,
}

/// Trait for persisting relationship type definitions
#[async_trait::async_trait]
pub trait RelationshipTypeStorage: Send + Sync + std::fmt::Debug {
    async fn save_type(&self, def: &RelationshipTypeDef) -> Result<(), RegistryError>;
    async fn load_all_types(&self) -> Result<Vec<RelationshipTypeDef>, RegistryError>;
    async fn delete_type(&self, name: &str) -> Result<(), RegistryError>;
}

impl RelationshipTypeRegistry {
    /// Create a new empty registry without persistence
    pub fn new() -> Self {
        Self {
            types: Arc::new(RwLock::new(HashMap::new())),
            storage: None,
        }
    }

    /// Create a new registry with persistence backend
    pub fn with_storage(storage: Arc<dyn RelationshipTypeStorage>) -> Self {
        Self {
            types: Arc::new(RwLock::new(HashMap::new())),
            storage: Some(storage),
        }
    }

    /// Load all types from storage (if available)
    pub async fn load_from_storage(&self) -> Result<usize, RegistryError> {
        if let Some(ref storage) = self.storage {
            let stored_types = storage.load_all_types().await?;
            let count = stored_types.len();
            
            let mut types = self.types.write().await;
            for def in stored_types {
                types.insert(def.name.clone(), def);
            }
            
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Register a new relationship type
    pub async fn register(&self, def: RelationshipTypeDef) -> Result<(), RegistryError> {
        let mut types = self.types.write().await;

        if types.contains_key(&def.name) {
            return Err(RegistryError::TypeAlreadyExists(def.name));
        }

        // If this type has an inverse, verify the inverse type name is valid
        if let Some(ref inverse_name) = def.inverse {
            if inverse_name.trim().is_empty() {
                return Err(RegistryError::InvalidTypeName(
                    "Inverse type name cannot be empty".to_string(),
                ));
            }
        }

        types.insert(def.name.clone(), def.clone());
        drop(types); // Release lock before persisting

        // Persist to storage if available
        if let Some(ref storage) = self.storage {
            storage.save_type(&def).await?;
        }

        Ok(())
    }

    /// Get a relationship type definition by name
    pub async fn get(&self, name: &str) -> Option<RelationshipTypeDef> {
        let types = self.types.read().await;
        types.get(name).cloned()
    }

    /// List all registered relationship types
    pub async fn list(&self) -> Vec<RelationshipTypeDef> {
        let types = self.types.read().await;
        types.values().cloned().collect()
    }

    /// Check if a type is registered
    pub async fn exists(&self, name: &str) -> bool {
        let types = self.types.read().await;
        types.contains_key(name)
    }

    /// Delete a relationship type
    pub async fn delete(&self, name: &str) -> Result<(), RegistryError> {
        let mut types = self.types.write().await;

        if !types.contains_key(name) {
            return Err(RegistryError::TypeNotFound(name.to_string()));
        }

        types.remove(name);
        drop(types); // Release lock before persisting

        // Persist deletion to storage if available
        if let Some(ref storage) = self.storage {
            storage.delete_type(name).await?;
        }

        Ok(())
    }

    /// Update an existing type definition
    pub async fn update(&self, def: RelationshipTypeDef) -> Result<(), RegistryError> {
        let mut types = self.types.write().await;

        if !types.contains_key(&def.name) {
            return Err(RegistryError::TypeNotFound(def.name.clone()));
        }

        types.insert(def.name.clone(), def.clone());
        drop(types); // Release lock before persisting

        // Persist update to storage if available
        if let Some(ref storage) = self.storage {
            storage.save_type(&def).await?;
        }

        Ok(())
    }

    /// Seed the registry with common relationship types
    pub async fn seed_common_types(&self) -> Result<(), RegistryError> {
        // Seed types from the existing RelationshipType enum
        let common_types = vec![
            RelationshipTypeDef::new("friendship".to_string())?
                .symmetric()
                .with_custom_metadata("category".to_string(), Value::String("social".to_string())),
            RelationshipTypeDef::new("rivalry".to_string())?
                .with_custom_metadata(
                    "category".to_string(),
                    Value::String("competitive".to_string()),
                ),
            RelationshipTypeDef::new("professional".to_string())?
                .with_custom_metadata(
                    "category".to_string(),
                    Value::String("work".to_string()),
                ),
            RelationshipTypeDef::new("mentorship".to_string())?
                .with_inverse("mentee".to_string())
                .with_custom_metadata("category".to_string(), Value::String("learning".to_string())),
            RelationshipTypeDef::new("family".to_string())?
                .symmetric()
                .with_custom_metadata(
                    "category".to_string(),
                    Value::String("kinship".to_string()),
                ),
            RelationshipTypeDef::new("romance".to_string())?
                .symmetric()
                .with_custom_metadata(
                    "category".to_string(),
                    Value::String("intimate".to_string()),
                ),
            RelationshipTypeDef::new("antagonistic".to_string())?
                .with_custom_metadata(
                    "category".to_string(),
                    Value::String("hostile".to_string()),
                ),
            RelationshipTypeDef::new("neutral".to_string())?
                .symmetric()
                .with_custom_metadata(
                    "category".to_string(),
                    Value::String("neutral".to_string()),
                ),
            RelationshipTypeDef::new("alliance".to_string())?
                .symmetric()
                .with_custom_metadata(
                    "category".to_string(),
                    Value::String("collaborative".to_string()),
                ),
            RelationshipTypeDef::new("competition".to_string())?
                .with_custom_metadata(
                    "category".to_string(),
                    Value::String("competitive".to_string()),
                ),
        ];

        for type_def in common_types {
            // Skip if already exists (don't error on re-seeding)
            let _ = self.register(type_def).await;
        }

        Ok(())
    }

    /// Get all registered type names
    pub async fn get_type_names(&self) -> Vec<String> {
        let types = self.types.read().await;
        types.keys().cloned().collect()
    }

    /// Count total registered types
    pub async fn count(&self) -> usize {
        let types = self.types.read().await;
        types.len()
    }

    /// Clear all registered types
    pub async fn clear(&self) {
        let mut types = self.types.write().await;
        types.clear();
    }
}

impl Default for RelationshipTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_new_type() {
        let registry = RelationshipTypeRegistry::new();
        let type_def = RelationshipTypeDef::new("custom_type".to_string()).unwrap();
        assert!(registry.register(type_def).await.is_ok());
    }

    #[tokio::test]
    async fn test_register_duplicate_fails() {
        let registry = RelationshipTypeRegistry::new();
        let type_def = RelationshipTypeDef::new("custom_type".to_string()).unwrap();
        registry.register(type_def.clone()).await.unwrap();
        assert!(registry.register(type_def).await.is_err());
    }

    #[tokio::test]
    async fn test_get_type() {
        let registry = RelationshipTypeRegistry::new();
        let type_def = RelationshipTypeDef::new("custom_type".to_string()).unwrap();
        registry.register(type_def.clone()).await.unwrap();

        let retrieved = registry.get("custom_type").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "custom_type");
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let registry = RelationshipTypeRegistry::new();
        assert!(registry.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_list_types() {
        let registry = RelationshipTypeRegistry::new();
        let type1 = RelationshipTypeDef::new("type1".to_string()).unwrap();
        let type2 = RelationshipTypeDef::new("type2".to_string()).unwrap();

        registry.register(type1).await.unwrap();
        registry.register(type2).await.unwrap();

        let list = registry.list().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_type() {
        let registry = RelationshipTypeRegistry::new();
        let type_def = RelationshipTypeDef::new("custom_type".to_string()).unwrap();
        registry.register(type_def).await.unwrap();
        assert!(registry.exists("custom_type").await);

        assert!(registry.delete("custom_type").await.is_ok());
        assert!(!registry.exists("custom_type").await);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_fails() {
        let registry = RelationshipTypeRegistry::new();
        assert!(registry.delete("nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn test_update_type() {
        let registry = RelationshipTypeRegistry::new();
        let mut type_def = RelationshipTypeDef::new("custom_type".to_string()).unwrap();
        registry.register(type_def.clone()).await.unwrap();

        type_def.symmetric = true;
        assert!(registry.update(type_def).await.is_ok());

        let retrieved = registry.get("custom_type").await.unwrap();
        assert!(retrieved.symmetric);
    }

    #[tokio::test]
    async fn test_seed_common_types() {
        let registry = RelationshipTypeRegistry::new();
        assert!(registry.seed_common_types().await.is_ok());
        assert!(registry.get("friendship").await.is_some());
        assert!(registry.get("rivalry").await.is_some());
    }

    #[tokio::test]
    async fn test_symmetric_type() {
        let type_def = RelationshipTypeDef::new("married_to".to_string())
            .unwrap()
            .symmetric();
        assert!(type_def.symmetric);
    }

    #[tokio::test]
    async fn test_transitive_type() {
        let type_def = RelationshipTypeDef::new("part_of".to_string())
            .unwrap()
            .transitive();
        assert!(type_def.transitive);
    }

    #[tokio::test]
    async fn test_with_inverse() {
        let type_def = RelationshipTypeDef::new("mentor".to_string())
            .unwrap()
            .with_inverse("mentee".to_string());
        assert_eq!(type_def.inverse, Some("mentee".to_string()));
    }

    #[tokio::test]
    async fn test_invalid_type_name_empty() {
        let result = RelationshipTypeDef::new("".to_string());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_type_name_special_chars() {
        let result = RelationshipTypeDef::new("type@invalid".to_string());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let registry = RelationshipTypeRegistry::new();
        let type_def = RelationshipTypeDef::new("custom_type".to_string()).unwrap();
        registry.register(type_def).await.unwrap();

        let reg1 = registry.clone();
        let reg2 = registry.clone();

        let handle1 = tokio::spawn(async move {
            reg1.get("custom_type").await.is_some()
        });

        let handle2 = tokio::spawn(async move {
            reg2.get("custom_type").await.is_some()
        });

        assert!(handle1.await.unwrap());
        assert!(handle2.await.unwrap());
    }

    #[tokio::test]
    async fn test_count() {
        let registry = RelationshipTypeRegistry::new();
        assert_eq!(registry.count().await, 0);

        let type_def = RelationshipTypeDef::new("type1".to_string()).unwrap();
        registry.register(type_def).await.unwrap();
        assert_eq!(registry.count().await, 1);
    }
}
