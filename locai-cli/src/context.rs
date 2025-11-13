use locai::config::ConfigBuilder;
use locai::prelude::*;
use locai::relationships::RelationshipTypeRegistry;

pub struct LocaiCliContext {
    pub memory_manager: MemoryManager,
    pub relationship_type_registry: RelationshipTypeRegistry,
}

impl LocaiCliContext {
    pub async fn new(data_dir: Option<String>) -> locai::Result<Self> {
        let mm = if let Some(dir) = data_dir {
            let config = ConfigBuilder::new()
                .with_data_dir(dir)
                .with_default_storage()
                .with_default_ml()
                .with_default_logging()
                .build()?;
            locai::init(config).await?
        } else {
            locai::init_with_defaults().await?
        };
        
        let registry = RelationshipTypeRegistry::new();
        
        Ok(Self { 
            memory_manager: mm,
            relationship_type_registry: registry,
        })
    }
}

