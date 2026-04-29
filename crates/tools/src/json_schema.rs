use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonSchemaPrimitiveType {
    String,
    Number,
    Boolean,
    Integer,
    Object,
    Array,
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonSchemaType {
    Single(JsonSchemaPrimitiveType),
    Multiple(Vec<JsonSchemaPrimitiveType>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Bool(bool),
    Schema(Box<JsonSchema>),
}

impl From<bool> for AdditionalProperties {
    fn from(b: bool) -> Self {
        AdditionalProperties::Bool(b)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<JsonSchemaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<JsonValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<AdditionalProperties>,
}

impl JsonSchema {
    pub fn string(description: Option<&str>) -> Self {
        JsonSchema {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::String)),
            description: description.map(|d| d.to_string()),
            ..Default::default()
        }
    }

    pub fn boolean(description: Option<&str>) -> Self {
        JsonSchema {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Boolean)),
            description: description.map(|d| d.to_string()),
            ..Default::default()
        }
    }

    pub fn integer(description: Option<&str>) -> Self {
        JsonSchema {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Integer)),
            description: description.map(|d| d.to_string()),
            ..Default::default()
        }
    }

    pub fn number(description: Option<&str>) -> Self {
        JsonSchema {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Number)),
            description: description.map(|d| d.to_string()),
            ..Default::default()
        }
    }

    pub fn array(items: JsonSchema, description: Option<&str>) -> Self {
        JsonSchema {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Array)),
            description: description.map(|d| d.to_string()),
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }

    pub fn object(
        properties: BTreeMap<String, JsonSchema>,
        required: Option<Vec<String>>,
        additional_properties: Option<bool>,
    ) -> Self {
        JsonSchema {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Object)),
            properties: Some(properties),
            required,
            additional_properties: additional_properties.map(Into::into),
            ..Default::default()
        }
    }

    pub fn to_json_value(&self) -> JsonValue {
        serde_json::to_value(self).unwrap_or(JsonValue::Null)
    }
}
