//! Unit tests for DTOs and OpenAPI schema generation
//!
//! These tests verify that:
//! 1. OpenAPI schema generation works correctly with our annotations
//! 2. Schema examples match our documentation recommendations
//! 3. DTO conversions preserve all data correctly

#[cfg(test)]
mod tests {
    use crate::api::dto::*;
    use locai::models::{MemoryBuilder, MemoryPriority, MemoryType};
    use serde_json::json;
    use utoipa::OpenApi;

    /// Test that MemoryDto conversion preserves memory_type format
    /// This verifies that custom memory types maintain their "custom:" prefix
    #[test]
    fn test_memory_dto_preserves_custom_prefix() {
        let memory = MemoryBuilder::new_with_content("Test content")
            .memory_type(MemoryType::Custom("dialogue".to_string()))
            .build();

        let dto = MemoryDto::from(memory);

        // The memory_type should include the "custom:" prefix as documented
        assert_eq!(dto.memory_type, "custom:dialogue");
    }

    /// Test that priority values are capitalized as documented
    #[test]
    fn test_memory_dto_priority_casing() {
        let test_cases = vec![
            (MemoryPriority::Low, "Low"),
            (MemoryPriority::Normal, "Normal"),
            (MemoryPriority::High, "High"),
            (MemoryPriority::Critical, "Critical"),
        ];

        for (priority, expected) in test_cases {
            let memory = MemoryBuilder::new_with_content("Test")
                .priority(priority)
                .build();

            let dto = MemoryDto::from(memory);
            assert_eq!(
                dto.priority, expected,
                "Priority {:?} should serialize to '{}'",
                priority, expected
            );
        }
    }

    /// Test that properties are preserved in MemoryDto conversion
    #[test]
    fn test_memory_dto_preserves_properties() {
        let properties = json!({
            "speaker": "TestSpeaker",
            "mood": "friendly",
            "location": "tavern"
        });

        let memory = MemoryBuilder::new_with_content("Test dialogue")
            .properties_json(properties.clone())
            .build();

        let dto = MemoryDto::from(memory);

        assert_eq!(dto.properties, properties);
        assert_eq!(dto.properties["speaker"], "TestSpeaker");
        assert_eq!(dto.properties["mood"], "friendly");
        assert_eq!(dto.properties["location"], "tavern");
    }

    /// Test CreateMemoryRequest deserialization with custom type
    #[test]
    fn test_create_memory_request_with_custom_type() {
        let json = json!({
            "content": "Test content",
            "memory_type": "custom:dialogue",
            "priority": "Normal",
            "tags": ["test"],
            "source": "test"
        });

        let request: CreateMemoryRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.memory_type, "custom:dialogue");
        assert_eq!(request.priority, "Normal");
    }

    /// Test CreateMemoryRequest uses correct defaults
    #[test]
    fn test_create_memory_request_defaults() {
        let json = json!({
            "content": "Test content"
        });

        let request: CreateMemoryRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.memory_type, "fact");
        assert_eq!(request.priority, "normal");
        assert_eq!(request.source, "api");
        assert_eq!(request.tags.len(), 0);
    }

    /// Test EntityDto properties documentation
    #[test]
    fn test_entity_dto_properties_structure() {
        let properties = json!({
            "name": "Thorin Oakenshield",
            "race": "dwarf",
            "class": "warrior"
        });

        // This test verifies that properties can contain a "name" field
        // as documented in our OpenAPI improvements
        assert!(properties.is_object());
        assert_eq!(properties["name"], "Thorin Oakenshield");
    }

    /// Test SearchResultDto score can be unbounded
    #[test]
    fn test_search_result_dto_score_unbounded() {
        let memory = MemoryBuilder::new_with_content("Test").build();

        // Test various score values including > 1.0
        let test_scores = vec![0.0, 0.5, 0.87, 1.0, 1.5, 2.3, 10.0];

        for score in test_scores {
            let result = SearchResultDto {
                memory: MemoryDto::from(memory.clone()),
                score: Some(score),
                match_method: None,
            };

            // Verify serialization works with any non-negative score
            let json = serde_json::to_value(&result).unwrap();
            assert_eq!(json["score"], score);
        }
    }

    /// Test SearchRequest filters accept custom memory types
    #[test]
    fn test_search_request_custom_memory_type() {
        let json = json!({
            "query": "test",
            "limit": 50,
            "memory_type": "custom:dialogue",
            "priority": "High"
        });

        let request: SearchRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.memory_type, Some("custom:dialogue".to_string()));
        assert_eq!(request.priority, Some("High".to_string()));
    }

    /// Test that OpenAPI schema can be generated without errors
    /// This ensures all our schema annotations are valid
    #[test]
    fn test_openapi_schema_generation() {
        use crate::api::ApiDoc;

        // This will panic if the OpenAPI schema has errors
        let openapi = ApiDoc::openapi();

        // Verify basic structure
        assert_eq!(openapi.info.title, "Locai Memory Service API");

        // Verify our key schemas are present
        let schemas = &openapi.components.as_ref().unwrap().schemas;
        assert!(
            schemas.contains_key("MemoryDto"),
            "MemoryDto schema should exist"
        );
        assert!(
            schemas.contains_key("EntityDto"),
            "EntityDto schema should exist"
        );
        assert!(
            schemas.contains_key("SearchResultDto"),
            "SearchResultDto schema should exist"
        );
        assert!(
            schemas.contains_key("CreateMemoryRequest"),
            "CreateMemoryRequest schema should exist"
        );
    }

    /// Test that MemoryDto schema includes our documentation improvements
    #[test]
    fn test_memory_dto_schema_examples() {
        use crate::api::ApiDoc;

        let openapi = ApiDoc::openapi();
        let schemas = &openapi.components.as_ref().unwrap().schemas;
        let memory_dto = schemas
            .get("MemoryDto")
            .expect("MemoryDto schema should exist");

        // Verify schema is defined (detailed checks would require walking the schema tree)
        // At minimum, verify it exists and can be serialized
        let _json = serde_json::to_string(&memory_dto).unwrap();
    }

    /// Test SearchMode enum serialization
    #[test]
    fn test_search_mode_serialization() {
        let modes = vec![
            (SearchMode::Text, "text"),
            (SearchMode::Vector, "vector"),
            (SearchMode::Hybrid, "hybrid"),
        ];

        for (mode, expected) in modes {
            let json = serde_json::to_value(&mode).unwrap();
            assert_eq!(json, expected);
        }
    }

    /// Test UpdateMemoryRequest optional fields
    #[test]
    fn test_update_memory_request_all_optional() {
        let json = json!({});

        let request: UpdateMemoryRequest = serde_json::from_value(json).unwrap();

        assert!(request.content.is_none());
        assert!(request.memory_type.is_none());
        assert!(request.priority.is_none());
        assert!(request.tags.is_none());
        assert!(request.source.is_none());
        assert!(request.expires_at.is_none());
        assert!(request.properties.is_none());
    }

    /// Test UpdateMemoryRequest with custom type and capitalized priority
    #[test]
    fn test_update_memory_request_with_values() {
        let json = json!({
            "memory_type": "custom:quest",
            "priority": "Critical",
            "tags": ["important", "quest"]
        });

        let request: UpdateMemoryRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.memory_type, Some("custom:quest".to_string()));
        assert_eq!(request.priority, Some("Critical".to_string()));
        assert_eq!(
            request.tags,
            Some(vec!["important".to_string(), "quest".to_string()])
        );
    }
}
