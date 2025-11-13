//! Command handlers for the Locai CLI

pub mod batch;
pub mod entity;
pub mod graph;
pub mod memory;
pub mod quickstart;
pub mod relationship;
pub mod relationship_type;
pub mod tutorial;

pub use batch::handle_batch_command;
pub use entity::handle_entity_command;
pub use graph::handle_graph_command;
pub use memory::handle_memory_command;
pub use quickstart::handle_quickstart_command;
pub use relationship::handle_relationship_command;
pub use relationship_type::handle_relationship_type_command;
pub use tutorial::handle_tutorial_command;
