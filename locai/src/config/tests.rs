#[cfg(test)]
mod tests {
    use crate::config::{
        ConfigBuilder, GraphStorageType, LocaiConfig, LogLevel, VectorStorageType, validation,
    };
    use std::path::PathBuf;

    #[test]
    fn test_default_config() {
        let config = LocaiConfig::default();
        assert_eq!(
            config.storage.graph.storage_type,
            GraphStorageType::SurrealDB
        );
        assert_eq!(
            config.storage.vector.storage_type,
            VectorStorageType::SurrealDB
        );
        assert_eq!(config.ml.embedding.model_name, "text-embedding-3-small");
        assert_eq!(config.logging.level, LogLevel::Info);
    }

    #[test]
    fn test_config_builder_with_surrealdb() {
        let config = ConfigBuilder::new()
            .with_data_dir("/tmp/test_data")
            .with_graph_storage_type(GraphStorageType::SurrealDB)
            .with_vector_storage_type(VectorStorageType::SurrealDB)
            .with_embedding_model("test-model")
            .with_log_level(LogLevel::Debug)
            .build()
            .unwrap();

        assert_eq!(config.storage.data_dir, PathBuf::from("/tmp/test_data"));
        assert_eq!(
            config.storage.graph.storage_type,
            GraphStorageType::SurrealDB
        );
        assert_eq!(
            config.storage.vector.storage_type,
            VectorStorageType::SurrealDB
        );
        assert_eq!(config.ml.embedding.model_name, "test-model");
        assert_eq!(config.logging.level, LogLevel::Debug);
    }

    #[test]
    fn test_config_builder_with_memory() {
        let config = ConfigBuilder::new()
            .with_data_dir("/tmp/test_data")
            .with_vector_storage_type(VectorStorageType::Memory)
            .with_embedding_model("test-model")
            .with_log_level(LogLevel::Debug)
            .build()
            .unwrap();

        assert_eq!(config.storage.data_dir, PathBuf::from("/tmp/test_data"));
        assert_eq!(
            config.storage.graph.storage_type,
            GraphStorageType::SurrealDB
        );
        assert_eq!(
            config.storage.vector.storage_type,
            VectorStorageType::Memory
        );
        assert_eq!(config.ml.embedding.model_name, "test-model");
        assert_eq!(config.logging.level, LogLevel::Debug);
    }

    #[test]
    fn test_validation() {
        // Test valid configuration
        let valid = ConfigBuilder::new().build();
        assert!(valid.is_ok());

        // Test that validation passes for default config
        let config = LocaiConfig::default();
        let result = validation::validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_predefined_configs() {
        let dev = ConfigBuilder::development().build().unwrap();
        let test = ConfigBuilder::testing().build().unwrap();

        assert_eq!(dev.storage.graph.storage_type, GraphStorageType::SurrealDB);
        assert_eq!(
            dev.storage.vector.storage_type,
            VectorStorageType::SurrealDB
        );
        assert_eq!(dev.logging.level, LogLevel::Debug);

        assert_eq!(test.storage.data_dir, PathBuf::from("./test_data"));
    }

    #[test]
    fn test_predefined_configs_production() {
        let prod = ConfigBuilder::production().build().unwrap();

        assert_eq!(prod.storage.graph.storage_type, GraphStorageType::SurrealDB);
        assert_eq!(
            prod.storage.vector.storage_type,
            VectorStorageType::SurrealDB
        );
        assert_eq!(prod.logging.level, LogLevel::Info);
    }

    #[test]
    fn test_config_serialization() {
        let config = ConfigBuilder::new()
            .with_data_dir("/tmp/test_data")
            .with_embedding_model("test-model")
            .build()
            .unwrap();

        // Test serialization to JSON
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LocaiConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.storage.data_dir, deserialized.storage.data_dir);
        assert_eq!(
            config.ml.embedding.model_name,
            deserialized.ml.embedding.model_name
        );
    }
}
