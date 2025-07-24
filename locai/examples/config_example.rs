use locai::config::ConfigBuilder;
use locai::config::ConfigLoader;
use locai::config::LogLevel;
use locai::config::{GraphStorageType, VectorStorageType};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Method 1: Use ConfigLoader to load from file and environment
    let mut loader = ConfigLoader::new();
    loader.load_file("examples/locai.toml")?
          .load_env();
    
    let config = loader.extract()?;
    
    println!("Configuration loaded from file:");
    println!("  Data dir: {:?}", config.storage.data_dir);
    println!("  Graph storage: {:?}", config.storage.graph.storage_type);
    println!("  Vector storage: {:?}", config.storage.vector.storage_type);
    println!("  Embedding model: {}", config.ml.embedding.model_name);
    
    // Method 2: Use ConfigBuilder for programmatic configuration
    let config = ConfigBuilder::new()
        .with_data_dir("./custom_data")
        .with_graph_storage_type(GraphStorageType::SurrealDB)
        .with_vector_storage_type(VectorStorageType::SurrealDB)
        .with_embedding_model("BAAI/bge-large-en")
        .with_log_level(LogLevel::Debug)
        .build()?;
    
    println!("\nConfiguration created with builder:");
    println!("  Data dir: {:?}", config.storage.data_dir);
    println!("  Graph storage: {:?}", config.storage.graph.storage_type);
    println!("  Vector storage: {:?}", config.storage.vector.storage_type);
    println!("  Embedding model: {}", config.ml.embedding.model_name);
    println!("  Log level: {:?}", config.logging.level);
    
    // Method 3: Use predefined configurations
    let dev_config = ConfigBuilder::development().build()?;
    let test_config = ConfigBuilder::testing().build()?;
    let prod_config = ConfigBuilder::production().build()?;
    
    println!("\nPredefined configurations:");
    println!("  Development - Storage: {:?}", dev_config.storage.graph.storage_type);
    println!("  Testing - Data dir: {:?}", test_config.storage.data_dir);
    println!("  Production - Vector storage: {:?}", prod_config.storage.vector.storage_type);
    
    Ok(())
} 