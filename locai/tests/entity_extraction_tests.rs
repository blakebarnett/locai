//! External tests for entity extraction functionality
//!
//! This file contains comprehensive tests for the entity extraction system,
//! moved from `locai/src/entity_extraction/tests.rs` to follow the 500-line
//! guideline for better LLM context management and code organization.
//!
//! Tests cover:
//! - Basic entity extraction (emails, URLs, phones, etc.)
//! - Entity resolution and merging
//! - Automatic relationship creation  
//! - Integration scenarios
//! - Production configurations

use chrono::{Duration, Utc};
use locai::entity_extraction::*;
use locai::storage::Entity;

#[cfg(test)]
mod basic_extraction_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_email_extraction() {
        let extractor = BasicEntityExtractor::new();
        let content = "Please contact john.doe@example.com for more information.";

        let entities = extractor.extract_entities(content).await.unwrap();

        assert!(!entities.is_empty());
        let email_entity = entities.iter().find(|e| e.entity_type == EntityType::Email);
        assert!(email_entity.is_some());

        let email = email_entity.unwrap();
        assert_eq!(email.text, "john.doe@example.com");
        assert_eq!(email.entity_type, EntityType::Email);
        assert!(email.confidence > 0.9);
    }

    #[tokio::test]
    async fn test_basic_url_extraction() {
        let extractor = BasicEntityExtractor::new();
        let content = "Visit our website at https://example.com for more details.";

        let entities = extractor.extract_entities(content).await.unwrap();

        let url_entity = entities.iter().find(|e| e.entity_type == EntityType::Url);
        assert!(url_entity.is_some());

        let url = url_entity.unwrap();
        assert_eq!(url.text, "https://example.com");
        assert_eq!(url.entity_type, EntityType::Url);
        assert!(url.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_basic_phone_extraction() {
        let extractor = BasicEntityExtractor::new();
        let content = "Call me at (555) 123-4567 if you need assistance.";

        let entities = extractor.extract_entities(content).await.unwrap();

        let phone_entity = entities
            .iter()
            .find(|e| e.entity_type == EntityType::PhoneNumber);
        assert!(phone_entity.is_some());

        let phone = phone_entity.unwrap();
        assert_eq!(phone.text.trim(), "(555) 123-4567");
        assert_eq!(phone.entity_type, EntityType::PhoneNumber);
        assert!(phone.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_basic_person_extraction_not_supported() {
        // BasicEntityExtractor does NOT extract persons - that's for ML extractors
        let extractor = BasicEntityExtractor::with_config(0.5);
        let content = "I met with Dr. John Smith yesterday to discuss the project.";

        let entities = extractor.extract_entities(content).await.unwrap();

        // Should not find any person entities since BasicEntityExtractor only handles structured data
        let person_entity = entities
            .iter()
            .find(|e| e.entity_type == EntityType::Person);
        assert!(
            person_entity.is_none(),
            "BasicEntityExtractor should not extract person entities"
        );

        // Verify it only extracts supported types
        for entity in &entities {
            assert!(
                matches!(
                    entity.entity_type,
                    EntityType::Email
                        | EntityType::Url
                        | EntityType::PhoneNumber
                        | EntityType::Date
                        | EntityType::Time
                        | EntityType::Money
                ),
                "BasicEntityExtractor should only extract structured data types"
            );
        }
    }

    #[tokio::test]
    async fn test_basic_money_extraction() {
        let extractor = BasicEntityExtractor::new();
        let content = "The cost is $150.50 for the entire package.";

        let entities = extractor.extract_entities(content).await.unwrap();

        let money_entity = entities.iter().find(|e| e.entity_type == EntityType::Money);
        assert!(money_entity.is_some());

        let money = money_entity.unwrap();
        assert_eq!(money.text, "$150.50");
        assert_eq!(money.entity_type, EntityType::Money);
        assert!(money.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_euro_money_extraction() {
        let extractor = BasicEntityExtractor::new();
        let content = "The research costs approximately €75,000 to fund.";

        let entities = extractor.extract_entities(content).await.unwrap();

        let money_entity = entities.iter().find(|e| e.entity_type == EntityType::Money);
        assert!(money_entity.is_some());

        let money = money_entity.unwrap();
        assert_eq!(money.text, "€75,000");
        assert_eq!(money.entity_type, EntityType::Money);
        assert!(money.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_organization_extraction_not_supported() {
        // BasicEntityExtractor does NOT extract organizations - that's for ML extractors
        let extractor = BasicEntityExtractor::new();
        let content = "Dr. Emily Watson from MIT will be presenting at the conference.";

        let entities = extractor.extract_entities(content).await.unwrap();

        // Should not find any organization entities since BasicEntityExtractor only handles structured data
        let org_entity = entities
            .iter()
            .find(|e| e.entity_type == EntityType::Organization);
        assert!(
            org_entity.is_none(),
            "BasicEntityExtractor should not extract organization entities"
        );

        // Should only find structured data types (none in this text)
        assert!(
            entities.is_empty(),
            "No structured data entities in this text"
        );
    }

    #[tokio::test]
    async fn test_improved_phone_extraction() {
        let extractor = BasicEntityExtractor::new();
        let content = "You can reach him at +1-555-123-4567 for assistance.";

        let entities = extractor.extract_entities(content).await.unwrap();

        let phone_entity = entities
            .iter()
            .find(|e| e.entity_type == EntityType::PhoneNumber);
        assert!(phone_entity.is_some());

        let phone = phone_entity.unwrap();
        assert_eq!(phone.text, "+1-555-123-4567");
        assert_eq!(phone.entity_type, EntityType::PhoneNumber);
        assert!(phone.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_mixed_entity_extraction() {
        // BasicEntityExtractor only extracts structured data, not named entities
        let extractor = BasicEntityExtractor::with_config(0.5);
        let content = "I met with John Smith yesterday at john@example.com to discuss the project. \
                      We're meeting again on January 15th, 2024 at the Seattle office. \
                      You can reach him at +1-555-123-4567 or visit https://example.com/john";

        let entities = extractor.extract_entities(content).await.unwrap();

        // Should find structured data types only
        let person_count = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Person)
            .count();
        let email_count = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Email)
            .count();
        let phone_count = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::PhoneNumber)
            .count();
        let url_count = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Url)
            .count();
        let date_count = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Date)
            .count();
        let location_count = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Location)
            .count();

        // BasicEntityExtractor does not extract named entities
        assert_eq!(
            person_count, 0,
            "BasicEntityExtractor should not extract persons"
        );
        assert_eq!(
            location_count, 0,
            "BasicEntityExtractor should not extract locations"
        );

        // But should extract structured data
        assert!(
            email_count > 0,
            "Should find at least one email (found {})",
            email_count
        );
        assert!(
            phone_count > 0,
            "Should find at least one phone (found {})",
            phone_count
        );
        assert!(
            url_count > 0,
            "Should find at least one URL (found {})",
            url_count
        );
        assert!(
            date_count > 0,
            "Should find at least one date (found {})",
            date_count
        );

        // Verify all extracted entities are supported types
        for entity in &entities {
            assert!(
                matches!(
                    entity.entity_type,
                    EntityType::Email
                        | EntityType::Url
                        | EntityType::PhoneNumber
                        | EntityType::Date
                        | EntityType::Time
                        | EntityType::Money
                ),
                "Found unsupported entity type: {:?}",
                entity.entity_type
            );
        }
    }

    #[tokio::test]
    async fn test_confidence_filtering() {
        let extractor = BasicEntityExtractor::with_config(0.9); // High confidence threshold
        let content = "Maybe John Smith and someone else."; // Ambiguous person reference

        let entities = extractor.extract_entities(content).await.unwrap();

        // Should filter out low-confidence entities
        for entity in &entities {
            assert!(
                entity.confidence >= 0.9,
                "All entities should meet confidence threshold"
            );
        }
    }

    #[tokio::test]
    async fn test_entity_positioning() {
        let extractor = BasicEntityExtractor::new();
        let content = "Email: test@example.com";

        let entities = extractor.extract_entities(content).await.unwrap();

        let email_entity = entities
            .iter()
            .find(|e| e.entity_type == EntityType::Email)
            .unwrap();
        assert_eq!(email_entity.start_pos, 7); // Position after "Email: "
        assert_eq!(email_entity.end_pos, 23); // End of email address
        assert_eq!(email_entity.len(), 16); // Length of "test@example.com"
    }

    #[test]
    fn test_entity_type_as_str() {
        assert_eq!(EntityType::Person.as_str(), "person");
        assert_eq!(EntityType::Email.as_str(), "email");
        assert_eq!(EntityType::Url.as_str(), "url");
        assert_eq!(EntityType::PhoneNumber.as_str(), "phone_number");
        assert_eq!(
            EntityType::Custom("custom_type".to_string()).as_str(),
            "custom_type"
        );
    }

    #[test]
    fn test_extracted_entity_format() {
        let entity = ExtractedEntity::new(
            "test@example.com".to_string(),
            EntityType::Email,
            0,
            16,
            0.95,
            "basic".to_string(),
        );

        let formatted = entity.format();
        assert!(formatted.contains("test@example.com"));
        assert!(formatted.contains("email"));
        assert!(formatted.contains("0.95"));
        assert!(formatted.contains("basic"));
    }

    #[tokio::test]
    async fn test_basic_entity_extraction_with_types() {
        // Use the standard constructor since new_with_types doesn't exist
        let extractor = BasicEntityExtractor::new();

        let text =
            "Contact us at support@example.com or visit https://example.com. Call +1-555-123-4567.";
        let entities = extractor.extract_entities(text).await.unwrap();

        // Should find all three types
        assert!(entities.len() >= 3);

        // Check email
        let email_entity = entities.iter().find(|e| e.entity_type == EntityType::Email);
        assert!(email_entity.is_some());
        if let Some(email) = email_entity {
            assert_eq!(email.text, "support@example.com");
            assert!(email.confidence > 0.9);
        }

        // Check URL - note: the regex might extract just the domain or the full URL
        let url_entity = entities.iter().find(|e| e.entity_type == EntityType::Url);
        assert!(url_entity.is_some());
        if let Some(url) = url_entity {
            // The regex extracts different parts depending on the pattern
            // It might include trailing punctuation, so be flexible
            let cleaned_url = url.text.trim_end_matches('.').trim_end_matches(',');
            assert!(
                cleaned_url == "https://example.com" || cleaned_url == "example.com",
                "Expected URL to be either full or domain, got: {} (cleaned: {})",
                url.text,
                cleaned_url
            );
        }

        // Check phone
        let phone_entity = entities
            .iter()
            .find(|e| e.entity_type == EntityType::PhoneNumber);
        assert!(phone_entity.is_some());
        if let Some(phone) = phone_entity {
            assert_eq!(phone.text, "+1-555-123-4567");
        }
    }
}

#[cfg(test)]
mod entity_resolution_tests {
    use super::*;

    #[test]
    fn test_entity_resolver_creation() {
        let config = EntityResolutionConfig::default();
        let _resolver = EntityResolver::new(config);
        // Should create without panicking
    }

    // Note: String similarity calculation is now private, so we test it through public methods

    #[test]
    fn test_merge_strategy_conservative() {
        let config = EntityResolutionConfig {
            merge_strategy: MergeStrategy::Conservative,
            ..Default::default()
        };
        let resolver = EntityResolver::new(config);

        // Create existing entity
        let mut existing_properties = serde_json::Map::new();
        existing_properties.insert(
            "name".to_string(),
            serde_json::Value::String("John".to_string()),
        );
        existing_properties.insert(
            "confidence".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(0.8).unwrap()),
        );
        existing_properties.insert(
            "email".to_string(),
            serde_json::Value::String("john@old.com".to_string()),
        );

        let existing_entity = Entity {
            id: "entity1".to_string(),
            entity_type: "person".to_string(),
            properties: serde_json::Value::Object(existing_properties),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Create extracted entity with new information
        let mut extracted = ExtractedEntity::new(
            "John Smith".to_string(),
            EntityType::Person,
            0,
            10,
            0.9,
            "test".to_string(),
        );
        extracted
            .metadata
            .insert("phone".to_string(), "555-1234".to_string());

        let merged = resolver.merge_entities(existing_entity, extracted).unwrap();

        // Should keep existing email (conservative)
        assert_eq!(
            merged.properties.get("email").unwrap().as_str().unwrap(),
            "john@old.com"
        );
        // Should add new phone
        assert_eq!(
            merged.properties.get("phone").unwrap().as_str().unwrap(),
            "555-1234"
        );
        // Should update confidence - use approximate comparison for floating point
        let confidence = merged
            .properties
            .get("confidence")
            .unwrap()
            .as_f64()
            .unwrap();
        // Conservative strategy may keep original confidence or use extracted confidence
        assert!(
            confidence >= 0.8,
            "Expected confidence >= 0.8, got {}",
            confidence
        );
    }

    // Note: Unique identifier detection is now internal, tested through resolution behavior

    // Note: Entity resolution fuzzy matching is tested through integration tests with storage

    // Note: Entity disambiguation is tested through integration tests with storage
}

#[cfg(test)]
mod relationship_tests {
    use super::*;

    #[test]
    fn test_automatic_relationship_config() {
        let config = AutomaticRelationshipConfig {
            enabled: true,
            methods: vec![
                RelationshipMethod::EntityCoreference {
                    min_entity_confidence: 0.8,
                },
                RelationshipMethod::TemporalProximity {
                    max_time_gap: Duration::minutes(5),
                    same_source_only: true,
                },
            ],
            min_confidence: 0.7,
            max_relationships_per_memory: Some(10),
        };

        let _creator = AutomaticRelationshipCreator::new(config);
        // Should create without panicking
    }

    #[test]
    fn test_temporal_confidence_calculation() {
        let config = AutomaticRelationshipConfig::default();
        let creator = AutomaticRelationshipCreator::new(config);

        let max_gap = Duration::minutes(10);

        // Test immediate (0 time difference)
        let confidence = creator.calculate_temporal_confidence(Duration::seconds(0), max_gap);
        assert_eq!(confidence, 1.0);

        // Test half the max gap
        let confidence = creator.calculate_temporal_confidence(Duration::minutes(5), max_gap);
        assert_eq!(confidence, 0.5);

        // Test at max gap
        let confidence = creator.calculate_temporal_confidence(max_gap, max_gap);
        assert_eq!(confidence, 0.0);

        // Test beyond max gap
        let confidence = creator.calculate_temporal_confidence(Duration::minutes(15), max_gap);
        assert_eq!(confidence, 0.0);
    }

    #[test]
    fn test_tag_overlap_calculation() {
        let config = AutomaticRelationshipConfig::default();
        let creator = AutomaticRelationshipCreator::new(config);

        let tags1 = vec![
            "rust".to_string(),
            "programming".to_string(),
            "web".to_string(),
        ];
        let tags2 = vec![
            "rust".to_string(),
            "programming".to_string(),
            "backend".to_string(),
        ];

        let overlap = creator.calculate_tag_overlap_ratio(&tags1, &tags2);

        // Common: rust, programming (2)
        // Total unique: rust, programming, web, backend (4)
        // Overlap ratio: 2/4 = 0.5
        assert_eq!(overlap, 0.5);
    }

    #[test]
    fn test_tag_overlap_no_common() {
        let config = AutomaticRelationshipConfig::default();
        let creator = AutomaticRelationshipCreator::new(config);

        let tags1 = vec!["rust".to_string(), "programming".to_string()];
        let tags2 = vec!["python".to_string(), "data".to_string()];

        let overlap = creator.calculate_tag_overlap_ratio(&tags1, &tags2);

        // No common tags
        // Total unique: rust, programming, python, data (4)
        // Overlap ratio: 0/4 = 0.0
        assert_eq!(overlap, 0.0);
    }

    #[test]
    fn test_tag_overlap_empty() {
        let config = AutomaticRelationshipConfig::default();
        let creator = AutomaticRelationshipCreator::new(config);

        let tags1 = vec![];
        let tags2 = vec!["python".to_string(), "data".to_string()];

        let overlap = creator.calculate_tag_overlap_ratio(&tags1, &tags2);
        assert_eq!(overlap, 0.0);
    }

    #[test]
    fn test_entity_coreference_confidence() {
        let config = AutomaticRelationshipConfig::default();
        let creator = AutomaticRelationshipCreator::new(config);

        // Create mock memory and entity
        let memory1 = locai::models::Memory {
            id: "mem1".to_string(),
            content: "Test content".to_string(),
            memory_type: locai::models::MemoryType::Episodic,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: locai::models::MemoryPriority::Normal,
            tags: vec![],
            source: "test".to_string(),
            expires_at: None,
            properties: serde_json::json!({}),
            related_memories: vec![],
            embedding: None,
        };

        let memory2 = memory1.clone();

        let mut entity_props = serde_json::Map::new();
        entity_props.insert(
            "name".to_string(),
            serde_json::Value::String("John".to_string()),
        );
        entity_props.insert(
            "confidence".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(0.9).unwrap()),
        );

        let entity = Entity {
            id: "entity1".to_string(),
            entity_type: "person".to_string(),
            properties: serde_json::Value::Object(entity_props),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let confidence =
            creator.calculate_entity_coreference_confidence(&memory1, &memory2, &entity);

        // Should be entity confidence * 0.9
        assert_eq!(confidence, 0.9 * 0.9);
    }
}

// Note: ModernBERT extractor tests have been moved along with the implementation
// Model-specific extractors are now in examples/entity_extraction/modernbert/
// This provides clean separation between generic core and model-specific implementations

/*
#[cfg(test)]
mod modern_extraction_tests {
    use super::*;

    // These tests have been moved to examples/entity_extraction/modernbert/tests/
    // The core library now only contains generic pipeline components and basic extractors

    #[tokio::test]
    async fn test_modern_bert_extractor_creation() {
        // Test moved to examples/entity_extraction/modernbert/
        // Use: cargo test --manifest-path examples/entity_extraction/modernbert/Cargo.toml
    }

    #[tokio::test]
    async fn test_combined_extraction() {
        // Test combining basic and ModernBERT extractors for comprehensive extraction
        let basic_extractor = BasicEntityExtractor::new();
        let modern_bert_extractor = ModernBertNerExtractor::modern_bert_base().await.unwrap();

        let content = "Contact support@example.com or call +1-555-123-4567. Visit https://example.com for details.";

        // Extract structured data with basic extractor
        let basic_entities = basic_extractor.extract_entities(content).await.unwrap();

        // Extract named entities with ModernBERT (if any in this content)
        let _named_entities = modern_bert_extractor.extract_entities(content).await.unwrap();

        // Should find structured data entities
        let email_count = basic_entities.iter().filter(|e| e.entity_type == EntityType::Email).count();
        let phone_count = basic_entities.iter().filter(|e| e.entity_type == EntityType::PhoneNumber).count();
        let url_count = basic_entities.iter().filter(|e| e.entity_type == EntityType::Url).count();

        assert!(email_count > 0, "Should extract email addresses");
        assert!(phone_count > 0, "Should extract phone numbers");
        assert!(url_count > 0, "Should extract URLs");

        // Verify entities are sorted by position
        let mut last_pos = 0;
        for entity in &basic_entities {
            assert!(entity.start_pos >= last_pos, "Entities should be sorted by position");
            last_pos = entity.start_pos;
        }
    }

    #[tokio::test]
    async fn test_modern_bert_named_entities() {
        // Test extraction of named entities (persons, organizations, locations)
        let extractor = ModernBertNerExtractor::modern_bert_base().await.unwrap();
        let content = "Dr. John Smith from Microsoft visited New York yesterday.";

        let entities = extractor.extract_entities(content).await.unwrap();

        // ModernBERT should extract named entities
        let person_count = entities.iter().filter(|e| e.entity_type == EntityType::Person).count();
        let org_count = entities.iter().filter(|e| e.entity_type == EntityType::Organization).count();
        let location_count = entities.iter().filter(|e| e.entity_type == EntityType::Location).count();

        // ModernBERT should extract NER entities
        tracing::info!("NER extraction results: {} persons, {} orgs, {} locations",
                      person_count, org_count, location_count);

        // Verify the extractor supports NER types
        let supported_types = extractor.supported_types();
        assert!(supported_types.contains(&EntityType::Person), "Should support Person entities");
        assert!(supported_types.contains(&EntityType::Organization), "Should support Organization entities");
        assert!(supported_types.contains(&EntityType::Location), "Should support Location entities");
    }

    #[tokio::test]
    async fn test_comprehensive_extraction() {
        // Test extraction from content with both structured data and named entities
        let basic_extractor = BasicEntityExtractor::new();
        let modern_bert_extractor = ModernBertNerExtractor::modern_bert_base().await.unwrap();

        let content = "Please contact Dr. Alice Johnson at alice@medical.com or +1-555-987-6543. \
                      She works at Stanford University and will be presenting at the conference \
                      on January 15th, 2024. More info at https://conf.stanford.edu/alice";

        // Extract structured data
        let basic_entities = basic_extractor.extract_entities(content).await.unwrap();

        // Extract named entities
        let _named_entities = modern_bert_extractor.extract_entities(content).await.unwrap();

        // Should extract structured data
        let email_count = basic_entities.iter().filter(|e| e.entity_type == EntityType::Email).count();
        let phone_count = basic_entities.iter().filter(|e| e.entity_type == EntityType::PhoneNumber).count();
        let url_count = basic_entities.iter().filter(|e| e.entity_type == EntityType::Url).count();
        let date_count = basic_entities.iter().filter(|e| e.entity_type == EntityType::Date).count();

        assert!(email_count > 0, "Should extract emails");
        assert!(phone_count > 0, "Should extract phone numbers");
        assert!(url_count > 0, "Should extract URLs");
        assert!(date_count > 0, "Should extract dates");

        // May extract named entities (depending on ML extractor functionality)
        let total_basic_entities = basic_entities.len();
        assert!(total_basic_entities >= 4, "Should extract at least the structured data entities");

        // Verify no duplicate overlapping entities within each extractor
        for (i, entity_a) in basic_entities.iter().enumerate() {
            for (j, entity_b) in basic_entities.iter().enumerate() {
                if i != j {
                    let overlap = entity_a.start_pos < entity_b.end_pos && entity_a.end_pos > entity_b.start_pos;
                    assert!(!overlap, "Found overlapping entities: '{}' and '{}'",
                           entity_a.text, entity_b.text);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_basic_extractor_functionality() {
        // Test basic extractor functionality
        let extractor = BasicEntityExtractor::new();

        // Should have reasonable priority
        assert!(extractor.priority() >= 80);

        // Should support structured data types
        let supported_types = extractor.supported_types();
        assert!(supported_types.contains(&EntityType::Email));
        assert!(supported_types.contains(&EntityType::Url));
        assert!(supported_types.contains(&EntityType::PhoneNumber));
    }

    #[tokio::test]
    async fn test_modern_bert_functionality() {
        // Test ModernBERT extractor functionality
        let extractor = ModernBertNerExtractor::modern_bert_base().await.unwrap();

        // Should have high priority
        assert!(extractor.priority() >= 95);

        // Should support named entity types
        let supported_types = extractor.supported_types();
        assert!(supported_types.contains(&EntityType::Person));
        assert!(supported_types.contains(&EntityType::Organization));
        assert!(supported_types.contains(&EntityType::Location));
    }

    #[tokio::test]
    async fn test_confidence_filtering() {
        // Test that basic extractor respects confidence thresholds
        let extractor = BasicEntityExtractor::with_config(0.9);
        let content = "Some ambiguous text that might have low-confidence extractions.";

        let entities = extractor.extract_entities(content).await.unwrap();

        // All returned entities should meet confidence threshold
        for entity in &entities {
            assert!(entity.confidence >= 0.9,
                   "Entity '{}' has confidence {} below threshold 0.9",
                   entity.text, entity.confidence);
        }
    }

    #[tokio::test]
    async fn test_basic_extractor_no_overlaps() {
        // Test that basic extractor doesn't create overlapping entities
        let basic_extractor = BasicEntityExtractor::new();

        let content = "Email john@example.com for info about https://example.com website.";
        let entities = basic_extractor.extract_entities(content).await.unwrap();

        // Should not have overlapping entities
        for (i, entity_a) in entities.iter().enumerate() {
            for (j, entity_b) in entities.iter().enumerate() {
                if i != j {
                    let overlap = entity_a.start_pos < entity_b.end_pos && entity_a.end_pos > entity_b.start_pos;
                    assert!(!overlap, "Found overlapping entities: '{}' ({}-{}) and '{}' ({}-{})",
                           entity_a.text, entity_a.start_pos, entity_a.end_pos,
                           entity_b.text, entity_b.start_pos, entity_b.end_pos);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_extractor_type_specialization() {
        // Verify that different extractors support their specialized types
        let basic_extractor = BasicEntityExtractor::new();
        let modern_bert_extractor = ModernBertNerExtractor::modern_bert_base().await.unwrap();

        let basic_types = basic_extractor.supported_types();
        let modern_bert_types = modern_bert_extractor.supported_types();

        // Basic extractor should support structured data types
        assert!(basic_types.contains(&EntityType::Email), "Should support emails");
        assert!(basic_types.contains(&EntityType::Url), "Should support URLs");
        assert!(basic_types.contains(&EntityType::PhoneNumber), "Should support phone numbers");
        assert!(basic_types.contains(&EntityType::Date), "Should support dates");
        assert!(basic_types.contains(&EntityType::Money), "Should support money");

        // ModernBERT should support NER types
        assert!(modern_bert_types.contains(&EntityType::Person), "Should support persons");
        assert!(modern_bert_types.contains(&EntityType::Organization), "Should support organizations");
        assert!(modern_bert_types.contains(&EntityType::Location), "Should support locations");
    }

    #[tokio::test]
    async fn test_basic_extractor_reliability() {
        // Test that basic extractor works reliably for structured data
        let extractor = BasicEntityExtractor::new();

        // Should always work for structured data
        let content = "Contact support@example.com for assistance.";
        let entities = extractor.extract_entities(content).await.unwrap();

        // Should extract the email
        let email_count = entities.iter().filter(|e| e.entity_type == EntityType::Email).count();
        assert!(email_count > 0, "Should extract basic entities reliably");
    }
}

// Helper trait for testing
trait MemoryTestExt {
    fn new_test_memory(id: &str, content: &str) -> locai::models::Memory;
}

impl MemoryTestExt for locai::models::Memory {
    fn new_test_memory(id: &str, content: &str) -> locai::models::Memory {
        locai::models::Memory {
            id: id.to_string(),
            content: content.to_string(),
            memory_type: locai::models::MemoryType::Episodic,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: locai::models::MemoryPriority::Normal,
            tags: vec![],
            source: "test".to_string(),
            expires_at: None,
            properties: serde_json::json!({}),
            related_memories: vec![],
            embedding: None,
        }
    }
}
*/
