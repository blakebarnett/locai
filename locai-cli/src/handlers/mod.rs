//! Command handlers for the Locai CLI

pub mod memory;
pub mod entity;
pub mod relationship;
pub mod graph;
pub mod batch;
pub mod relationship_type;
pub mod tutorial;
pub mod quickstart;

pub use memory::handle_memory_command;
pub use entity::handle_entity_command;
pub use relationship::handle_relationship_command;
pub use graph::handle_graph_command;
pub use batch::handle_batch_command;
pub use relationship_type::handle_relationship_type_command;
pub use tutorial::handle_tutorial_command;
pub use quickstart::handle_quickstart_command;
