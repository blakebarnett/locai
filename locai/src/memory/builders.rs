//! Memory builder convenience methods
//! 
//! This module provides convenient methods for creating different types of memories
//! with various options and configurations.

use crate::models::{MemoryType, MemoryPriority, MemoryBuilder};
use crate::memory::operations::MemoryOperations;
use crate::{Result};
use std::sync::Arc;

/// Memory builder convenience methods
#[derive(Debug)]
pub struct MemoryBuilders {
    operations: Arc<MemoryOperations>,
}

impl MemoryBuilders {
    /// Create a new memory builders instance
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }

    /// Add a fact memory (convenience method)
    /// 
    /// # Arguments
    /// * `content` - The content of the fact memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_fact<S: Into<String>>(&self, content: S) -> Result<String> {
        let memory = MemoryBuilder::fact(content).build();
        self.operations.store_memory(memory).await
    }
    
    /// Add a conversation memory (convenience method)
    /// 
    /// # Arguments
    /// * `content` - The content of the conversation memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_conversation<S: Into<String>>(&self, content: S) -> Result<String> {
        let memory = MemoryBuilder::conversation(content).build();
        self.operations.store_memory(memory).await
    }
    
    /// Add a procedural memory (convenience method)
    /// 
    /// # Arguments
    /// * `content` - The content of the procedural memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_procedural<S: Into<String>>(&self, content: S) -> Result<String> {
        let memory = MemoryBuilder::procedural(content).build();
        self.operations.store_memory(memory).await
    }
    
    /// Add an episodic memory (convenience method)
    /// 
    /// # Arguments
    /// * `content` - The content of the episodic memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_episodic<S: Into<String>>(&self, content: S) -> Result<String> {
        let memory = MemoryBuilder::episodic(content).build();
        self.operations.store_memory(memory).await
    }
    
    /// Add an identity memory (convenience method)
    /// 
    /// # Arguments
    /// * `content` - The content of the identity memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_identity<S: Into<String>>(&self, content: S) -> Result<String> {
        let memory = MemoryBuilder::identity(content).build();
        self.operations.store_memory(memory).await
    }

    /// Add a world memory (convenience method)
    /// 
    /// # Arguments
    /// * `content` - The content of the world memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_world<S: Into<String>>(&self, content: S) -> Result<String> {
        let memory = MemoryBuilder::world(content).build();
        self.operations.store_memory(memory).await
    }
    
    /// Add an action memory (convenience method)
    /// 
    /// # Arguments
    /// * `content` - The content of the action memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_action<S: Into<String>>(&self, content: S) -> Result<String> {
        let memory = MemoryBuilder::action(content).build();
        self.operations.store_memory(memory).await
    }
    
    /// Add an event memory (convenience method)
    /// 
    /// # Arguments
    /// * `content` - The content of the event memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_event<S: Into<String>>(&self, content: S) -> Result<String> {
        let memory = MemoryBuilder::event(content).build();
        self.operations.store_memory(memory).await
    }
    
    /// Add a memory with a specific type
    /// 
    /// # Arguments
    /// * `content` - The content of the memory
    /// * `memory_type` - The type of memory
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_memory<S: Into<String>>(&self, content: S, memory_type: MemoryType) -> Result<String> {
        let memory = MemoryBuilder::new_with_content(content)
            .memory_type(memory_type)
            .build();
        self.operations.store_memory(memory).await
    }
    
    /// Add a memory with customization options
    /// 
    /// # Arguments
    /// * `content` - The content of the memory
    /// * `options` - A function that customizes the memory builder
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_memory_with_options<S, F>(&self, content: S, options: F) -> Result<String>
    where 
        S: Into<String>,
        F: FnOnce(MemoryBuilder) -> MemoryBuilder 
    {
        let builder = MemoryBuilder::new_with_content(content);
        let memory = options(builder).build();
        self.operations.store_memory(memory).await
    }

    /// Add a memory with priority
    /// 
    /// # Arguments
    /// * `content` - The content of the memory
    /// * `memory_type` - The type of memory
    /// * `priority` - The priority level
    /// 
    /// # Returns
    /// The ID of the stored memory
    pub async fn add_memory_with_priority<S: Into<String>>(
        &self, 
        content: S, 
        memory_type: MemoryType,
        priority: MemoryPriority
    ) -> Result<String> {
        let memory = MemoryBuilder::new_with_content(content)
            .memory_type(memory_type)
            .priority(priority)
            .build();
        self.operations.store_memory(memory).await
    }
} 