//! Relationship Metadata Schema Validation
//!
//! Provides schema validation for relationship metadata using JSON Schema format.
//! Supports validation of metadata against type-specific schemas defined in the registry.

use serde_json::{Value, json};

/// Error types for validation operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Schema validation failed: {0}")]
    ValidationFailed(String),

    #[error("Invalid schema format: {0}")]
    InvalidSchema(String),

    #[error("Missing required field: {0}")]
    MissingRequiredField(String),

    #[error("Type mismatch for field '{field}': expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Internal validation error: {0}")]
    InternalError(String),
}

/// Schema validator for relationship metadata
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate data against a JSON Schema
    pub fn validate(schema: &Value, data: &Value) -> Result<(), ValidationError> {
        if schema.is_null() {
            // No schema means no validation
            return Ok(());
        }

        Self::validate_against_schema(schema, data)
    }

    /// Validate a single property against its schema definition
    fn validate_against_schema(schema: &Value, data: &Value) -> Result<(), ValidationError> {
        let schema_obj = schema.as_object().ok_or_else(|| {
            ValidationError::InvalidSchema("Schema must be an object".to_string())
        })?;

        // Check type constraint
        if let Some(expected_type) = schema_obj.get("type") {
            Self::validate_type(expected_type, data)?;
        }

        // Check required fields (for objects)
        if let Some(required) = schema_obj.get("required") {
            if let Some(required_array) = required.as_array() {
                if let Some(data_obj) = data.as_object() {
                    for field in required_array {
                        if let Some(field_name) = field.as_str() {
                            if !data_obj.contains_key(field_name) {
                                return Err(ValidationError::MissingRequiredField(
                                    field_name.to_string(),
                                ));
                            }
                        }
                    }
                } else {
                    return Err(ValidationError::ValidationFailed(
                        "Required fields specified but data is not an object".to_string(),
                    ));
                }
            }
        }

        // Check properties (for objects)
        if let Some(properties) = schema_obj.get("properties") {
            if let Some(properties_obj) = properties.as_object() {
                if let Some(data_obj) = data.as_object() {
                    for (prop_name, prop_schema) in properties_obj {
                        if let Some(prop_data) = data_obj.get(prop_name) {
                            Self::validate_against_schema(prop_schema, prop_data)?;
                        }
                    }
                }
            }
        }

        // Check items (for arrays)
        if let Some(items) = schema_obj.get("items") {
            if let Some(data_array) = data.as_array() {
                for item in data_array {
                    Self::validate_against_schema(items, item)?;
                }
            }
        }

        // Check enum constraint
        if let Some(enum_values) = schema_obj.get("enum") {
            if let Some(enum_array) = enum_values.as_array() {
                if !enum_array.iter().any(|v| v == data) {
                    let valid_values: Vec<String> = enum_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    return Err(ValidationError::ValidationFailed(format!(
                        "Value must be one of: {}",
                        valid_values.join(", ")
                    )));
                }
            }
        }

        // Check minimum/maximum for numbers
        if data.is_number() {
            if let Some(minimum) = schema_obj.get("minimum") {
                if let Some(min_val) = minimum.as_f64() {
                    if let Some(data_val) = data.as_f64() {
                        if data_val < min_val {
                            return Err(ValidationError::ValidationFailed(format!(
                                "Value {} is less than minimum {}",
                                data_val, min_val
                            )));
                        }
                    }
                }
            }

            if let Some(maximum) = schema_obj.get("maximum") {
                if let Some(max_val) = maximum.as_f64() {
                    if let Some(data_val) = data.as_f64() {
                        if data_val > max_val {
                            return Err(ValidationError::ValidationFailed(format!(
                                "Value {} is greater than maximum {}",
                                data_val, max_val
                            )));
                        }
                    }
                }
            }
        }

        // Check minLength/maxLength for strings
        if let Some(s) = data.as_str() {
            if let Some(min_length) = schema_obj.get("minLength") {
                if let Some(min_val) = min_length.as_u64() {
                    if (s.len() as u64) < min_val {
                        return Err(ValidationError::ValidationFailed(format!(
                            "String length {} is less than minimum {}",
                            s.len(),
                            min_val
                        )));
                    }
                }
            }

            if let Some(max_length) = schema_obj.get("maxLength") {
                if let Some(max_val) = max_length.as_u64() {
                    if (s.len() as u64) > max_val {
                        return Err(ValidationError::ValidationFailed(format!(
                            "String length {} exceeds maximum {}",
                            s.len(),
                            max_val
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate that data matches the expected type
    fn validate_type(type_spec: &Value, data: &Value) -> Result<(), ValidationError> {
        let expected_type = type_spec
            .as_str()
            .ok_or_else(|| ValidationError::InvalidSchema("Type must be a string".to_string()))?;

        let actual_type = match data {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };

        if expected_type == actual_type {
            Ok(())
        } else if expected_type == "integer" && data.is_number() {
            // Special case: integer type
            if let Some(num) = data.as_f64() {
                if num.fract() == 0.0 {
                    return Ok(());
                }
            }
            Err(ValidationError::TypeMismatch {
                field: "value".to_string(),
                expected: expected_type.to_string(),
                actual: actual_type.to_string(),
            })
        } else {
            Err(ValidationError::TypeMismatch {
                field: "value".to_string(),
                expected: expected_type.to_string(),
                actual: actual_type.to_string(),
            })
        }
    }

    /// Parse and validate a schema definition
    pub fn parse_schema(schema_value: &Value) -> Result<Value, ValidationError> {
        // Basic schema validation
        if !schema_value.is_object() && !schema_value.is_null() {
            return Err(ValidationError::InvalidSchema(
                "Schema must be an object or null".to_string(),
            ));
        }

        // Ensure schema has valid type if specified
        if let Some(schema_obj) = schema_value.as_object() {
            if let Some(type_field) = schema_obj.get("type") {
                if let Some(type_str) = type_field.as_str() {
                    match type_str {
                        "null" | "boolean" | "object" | "array" | "number" | "string"
                        | "integer" => {
                            // Valid types
                        }
                        _ => {
                            return Err(ValidationError::InvalidSchema(format!(
                                "Invalid type: {}",
                                type_str
                            )));
                        }
                    }
                }
            }
        }

        Ok(schema_value.clone())
    }

    /// Create a simple schema for a single field
    pub fn simple_schema(field_type: &str) -> Result<Value, ValidationError> {
        match field_type {
            "string" => Ok(json!({ "type": "string" })),
            "number" => Ok(json!({ "type": "number" })),
            "integer" => Ok(json!({ "type": "integer" })),
            "boolean" => Ok(json!({ "type": "boolean" })),
            "array" => Ok(json!({ "type": "array" })),
            "object" => Ok(json!({ "type": "object" })),
            _ => Err(ValidationError::InvalidSchema(format!(
                "Unknown field type: {}",
                field_type
            ))),
        }
    }

    /// Create a schema for an object with required properties
    pub fn object_schema(
        properties: Vec<(&str, &str)>,
        required: Option<Vec<&str>>,
    ) -> Result<Value, ValidationError> {
        let mut schema = json!({
            "type": "object",
            "properties": {}
        });

        if let Some(props) = schema.get_mut("properties") {
            for (name, field_type) in properties {
                let field_schema = Self::simple_schema(field_type)?;
                props[name] = field_schema;
            }
        }

        if let Some(req_fields) = required {
            schema["required"] = json!(req_fields);
        }

        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_null_schema() {
        let schema = Value::Null;
        let data = json!({"any": "thing"});
        assert!(SchemaValidator::validate(&schema, &data).is_ok());
    }

    #[test]
    fn test_validate_string_type() {
        let schema = json!({"type": "string"});
        assert!(SchemaValidator::validate(&schema, &json!("hello")).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!(123)).is_err());
    }

    #[test]
    fn test_validate_number_type() {
        let schema = json!({"type": "number"});
        assert!(SchemaValidator::validate(&schema, &json!(123.45)).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!("hello")).is_err());
    }

    #[test]
    fn test_validate_integer_type() {
        let schema = json!({"type": "integer"});
        assert!(SchemaValidator::validate(&schema, &json!(123)).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!(123.45)).is_err());
    }

    #[test]
    fn test_validate_boolean_type() {
        let schema = json!({"type": "boolean"});
        assert!(SchemaValidator::validate(&schema, &json!(true)).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!("true")).is_err());
    }

    #[test]
    fn test_validate_object_type() {
        let schema = json!({"type": "object"});
        assert!(SchemaValidator::validate(&schema, &json!({})).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!([1, 2, 3])).is_err());
    }

    #[test]
    fn test_validate_array_type() {
        let schema = json!({"type": "array"});
        assert!(SchemaValidator::validate(&schema, &json!([1, 2, 3])).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!({"a": 1})).is_err());
    }

    #[test]
    fn test_validate_required_fields() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name", "age"]
        });

        let valid_data = json!({"name": "Alice", "age": 30});
        assert!(SchemaValidator::validate(&schema, &valid_data).is_ok());

        let invalid_data = json!({"name": "Alice"});
        assert!(SchemaValidator::validate(&schema, &invalid_data).is_err());
    }

    #[test]
    fn test_validate_enum() {
        let schema = json!({"enum": ["red", "green", "blue"]});
        assert!(SchemaValidator::validate(&schema, &json!("red")).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!("yellow")).is_err());
    }

    #[test]
    fn test_validate_minimum() {
        let schema = json!({"type": "number", "minimum": 10.0});
        assert!(SchemaValidator::validate(&schema, &json!(15)).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!(5)).is_err());
    }

    #[test]
    fn test_validate_maximum() {
        let schema = json!({"type": "number", "maximum": 100.0});
        assert!(SchemaValidator::validate(&schema, &json!(50)).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!(150)).is_err());
    }

    #[test]
    fn test_validate_string_minlength() {
        let schema = json!({"type": "string", "minLength": 3});
        assert!(SchemaValidator::validate(&schema, &json!("hello")).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!("hi")).is_err());
    }

    #[test]
    fn test_validate_string_maxlength() {
        let schema = json!({"type": "string", "maxLength": 5});
        assert!(SchemaValidator::validate(&schema, &json!("hello")).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!("toolong")).is_err());
    }

    #[test]
    fn test_validate_array_items() {
        let schema = json!({
            "type": "array",
            "items": {"type": "integer"}
        });

        assert!(SchemaValidator::validate(&schema, &json!([1, 2, 3])).is_ok());
        assert!(SchemaValidator::validate(&schema, &json!([1, "two", 3])).is_err());
    }

    #[test]
    fn test_nested_object_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "metadata": {
                    "type": "object",
                    "properties": {
                        "rank": {"type": "string"},
                        "since": {"type": "integer"}
                    }
                }
            }
        });

        let valid = json!({"metadata": {"rank": "senior", "since": 2020}});
        assert!(SchemaValidator::validate(&schema, &valid).is_ok());

        let invalid = json!({"metadata": {"rank": "senior", "since": "2020"}});
        assert!(SchemaValidator::validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_parse_schema() {
        let schema = json!({"type": "string"});
        assert!(SchemaValidator::parse_schema(&schema).is_ok());
    }

    #[test]
    fn test_parse_invalid_schema() {
        let schema = json!("not an object");
        assert!(SchemaValidator::parse_schema(&schema).is_err());
    }

    #[test]
    fn test_simple_schema() {
        let schema = SchemaValidator::simple_schema("string").unwrap();
        assert_eq!(schema["type"], "string");
    }

    #[test]
    fn test_object_schema() {
        let schema = SchemaValidator::object_schema(
            vec![("name", "string"), ("age", "integer")],
            Some(vec!["name"]),
        )
        .unwrap();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
        assert!(schema["required"].is_array());
    }
}
