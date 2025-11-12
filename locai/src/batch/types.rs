//! Batch operation types for bulk memory and relationship operations
//!
//! This module defines the types used for batch operations, allowing clients
//! to perform multiple operations in a single request, optionally as a transaction.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single operation in a batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", content = "data")]
pub enum BatchOperation {
    /// Create a new memory
    CreateMemory {
        /// Memory content
        content: String,
        /// Memory type (e.g., "fact", "episode", or custom type)
        memory_type: String,
        /// Optional priority level: 0 (Low), 1 (Normal), 2 (High), 3 (Critical)
        priority: Option<u8>,
        /// Optional tags for categorization
        tags: Option<Vec<String>>,
        /// Optional source/origin of the memory
        source: Option<String>,
        /// Optional custom properties
        properties: Option<Value>,
    },

    /// Update an existing memory
    UpdateMemory {
        /// Memory ID to update
        id: String,
        /// Optional new content
        content: Option<String>,
        /// Optional new priority
        priority: Option<u8>,
        /// Optional new tags (replaces existing)
        tags: Option<Vec<String>>,
        /// Optional new properties (merged)
        properties: Option<Value>,
    },

    /// Delete a memory
    DeleteMemory {
        /// Memory ID to delete
        id: String,
    },

    /// Create a new relationship
    CreateRelationship {
        /// Source memory or entity ID
        source: String,
        /// Target memory or entity ID
        target: String,
        /// Relationship type (e.g., "knows", "has_character")
        relationship_type: String,
        /// Optional relationship properties
        properties: Option<Value>,
        /// Optional enforcement of constraints (symmetry, transitivity)
        enforce_constraints: Option<bool>,
    },

    /// Update an existing relationship
    UpdateRelationship {
        /// Relationship ID to update
        id: String,
        /// Optional new properties
        properties: Option<Value>,
    },

    /// Delete a relationship
    DeleteRelationship {
        /// Relationship ID to delete
        id: String,
    },

    /// Update memory metadata only
    UpdateMetadata {
        /// Memory ID to update
        memory_id: String,
        /// Metadata properties to update
        metadata: Value,
    },
}

/// Result of a single batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BatchResult {
    /// Operation succeeded
    Success {
        /// Index in the original operations array
        operation_index: usize,
        /// ID of the created/updated/deleted resource
        resource_id: String,
        /// Optional message
        message: Option<String>,
    },

    /// Operation failed
    Error {
        /// Index in the original operations array
        operation_index: usize,
        /// Error message
        error: String,
        /// Optional error code
        error_code: Option<String>,
    },
}

/// Response from a batch operation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    /// Individual operation results
    pub results: Vec<BatchResult>,
    /// Total number of operations processed
    pub completed: usize,
    /// Number of operations that failed
    pub failed: usize,
    /// Whether all operations were executed in a transaction
    pub transaction: bool,
    /// Transaction ID if available (for tracking/debugging)
    pub transaction_id: Option<String>,
}

impl BatchResponse {
    /// Create a new empty batch response
    pub fn new(transaction: bool) -> Self {
        Self {
            results: Vec::new(),
            completed: 0,
            failed: 0,
            transaction,
            transaction_id: None,
        }
    }

    /// Add a successful result
    pub fn add_success(&mut self, operation_index: usize, resource_id: String) {
        self.results.push(BatchResult::Success {
            operation_index,
            resource_id,
            message: None,
        });
        self.completed += 1;
    }

    /// Add a successful result with message
    pub fn add_success_with_message(
        &mut self,
        operation_index: usize,
        resource_id: String,
        message: String,
    ) {
        self.results.push(BatchResult::Success {
            operation_index,
            resource_id,
            message: Some(message),
        });
        self.completed += 1;
    }

    /// Add an error result
    pub fn add_error(&mut self, operation_index: usize, error: String) {
        self.results.push(BatchResult::Error {
            operation_index,
            error,
            error_code: None,
        });
        self.failed += 1;
    }

    /// Add an error result with code
    pub fn add_error_with_code(
        &mut self,
        operation_index: usize,
        error: String,
        error_code: String,
    ) {
        self.results.push(BatchResult::Error {
            operation_index,
            error,
            error_code: Some(error_code),
        });
        self.failed += 1;
    }

    /// Check if all operations were successful
    pub fn all_successful(&self) -> bool {
        self.failed == 0
    }

    /// Check if any operations failed
    pub fn has_errors(&self) -> bool {
        self.failed > 0
    }
}

/// Errors that can occur during batch execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchError {
    /// Batch exceeds maximum size
    TooLarge {
        /// Number of operations submitted
        submitted: usize,
        /// Maximum allowed
        max_size: usize,
    },
    /// Transaction failure
    TransactionFailed {
        /// Error details
        reason: String,
    },
    /// Rollback failure
    RollbackFailed {
        /// Resource ID that failed to rollback
        resource_id: String,
    },
    /// Invalid operation
    InvalidOperation {
        /// Operation index
        index: usize,
        /// Error details
        reason: String,
    },
    /// Storage error
    StorageError {
        /// Error message
        message: String,
    },
    /// Validation error
    ValidationError {
        /// Error message
        message: String,
    },
}

impl std::fmt::Display for BatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge {
                submitted,
                max_size,
            } => write!(
                f,
                "Batch size {} exceeds maximum {}",
                submitted, max_size
            ),
            Self::TransactionFailed { reason } => write!(f, "Transaction failed: {}", reason),
            Self::RollbackFailed { resource_id } => write!(f, "Failed to rollback resource: {}", resource_id),
            Self::InvalidOperation { index, reason } => {
                write!(f, "Operation {} invalid: {}", index, reason)
            }
            Self::StorageError { message } => write!(f, "Storage error: {}", message),
            Self::ValidationError { message } => write!(f, "Validation error: {}", message),
        }
    }
}

impl std::error::Error for BatchError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_response_creation() {
        let response = BatchResponse::new(false);
        assert_eq!(response.completed, 0);
        assert_eq!(response.failed, 0);
        assert!(!response.transaction);
        assert!(response.all_successful());
    }

    #[test]
    fn test_batch_response_add_success() {
        let mut response = BatchResponse::new(false);
        response.add_success(0, "mem_123".to_string());

        assert_eq!(response.completed, 1);
        assert_eq!(response.failed, 0);
        assert!(response.all_successful());
        assert_eq!(response.results.len(), 1);
    }

    #[test]
    fn test_batch_response_add_error() {
        let mut response = BatchResponse::new(false);
        response.add_error(0, "Something went wrong".to_string());

        assert_eq!(response.completed, 0);
        assert_eq!(response.failed, 1);
        assert!(!response.all_successful());
        assert!(response.has_errors());
    }

    #[test]
    fn test_batch_response_mixed() {
        let mut response = BatchResponse::new(true);
        response.add_success(0, "mem_123".to_string());
        response.add_error(1, "Invalid data".to_string());
        response.add_success(2, "rel_456".to_string());

        assert_eq!(response.completed, 2);
        assert_eq!(response.failed, 1);
        assert_eq!(response.results.len(), 3);
        assert!(!response.all_successful());
    }

    #[test]
    fn test_batch_operation_serialization() {
        let op = BatchOperation::CreateMemory {
            content: "Test memory".to_string(),
            memory_type: "fact".to_string(),
            priority: Some(2),
            tags: Some(vec!["important".to_string()]),
            source: None,
            properties: None,
        };

        let json = serde_json::to_string(&op).expect("Should serialize");
        let deserialized: BatchOperation =
            serde_json::from_str(&json).expect("Should deserialize");

        match deserialized {
            BatchOperation::CreateMemory {
                content,
                memory_type,
                priority,
                ..
            } => {
                assert_eq!(content, "Test memory");
                assert_eq!(memory_type, "fact");
                assert_eq!(priority, Some(2));
            }
            _ => panic!("Unexpected operation type"),
        }
    }
}
