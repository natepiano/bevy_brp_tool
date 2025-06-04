//! Builder for constructing JSON-RPC parameters
//!
//! This module provides a fluent builder API for creating JSON parameters
//! for RPC calls, reducing duplication and improving maintainability.

use serde_json::{Map, Value, json};

/// Builder for JSON-RPC parameters
#[derive(Default)]
pub struct RpcParamsBuilder {
    params: Map<String, Value>,
}

impl RpcParamsBuilder {
    /// Create a new empty builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an entity ID parameter
    pub fn entity(mut self, entity: u64) -> Self {
        self.params.insert("entity".to_string(), json!(entity));
        self
    }

    /// Add a component name parameter
    pub fn component(mut self, component: impl Into<String>) -> Self {
        self.params
            .insert("component".to_string(), json!(component.into()));
        self
    }

    /// Add a resource name parameter
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.params
            .insert("resource".to_string(), json!(resource.into()));
        self
    }

    /// Add a path parameter (for file operations)
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.params.insert("path".to_string(), json!(path.into()));
        self
    }

    /// Add a components parameter (for spawn operations)
    pub fn components(mut self, components: Value) -> Self {
        self.params.insert("components".to_string(), components);
        self
    }

    /// Add component data for insert operations (creates components map)
    pub fn component_data(mut self, component: &str, data: Value) -> Self {
        let mut components = serde_json::Map::new();
        components.insert(component.to_string(), data);
        self.params
            .insert("components".to_string(), Value::Object(components));
        self
    }

    /// Add component list for remove/get operations (creates components array)
    pub fn component_list(mut self, components: Vec<&str>) -> Self {
        let component_names: Vec<Value> = components.into_iter().map(|c| json!(c)).collect();
        self.params
            .insert("components".to_string(), Value::Array(component_names));
        self
    }

    /// Add a parent parameter (for hierarchy operations)
    pub fn parent(mut self, parent: Value) -> Self {
        self.params.insert("parent".to_string(), parent);
        self
    }

    /// Add entities parameter for reparent operations (array of entity IDs)
    pub fn entities(mut self, entities: Vec<u64>) -> Self {
        self.params.insert("entities".to_string(), json!(entities));
        self
    }

    /// Add any custom field
    pub fn field(mut self, key: impl Into<String>, value: Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// Build the final JSON value
    pub fn build(self) -> Value {
        Value::Object(self.params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_component_builder() {
        let params = RpcParamsBuilder::new()
            .entity(123)
            .component("Transform")
            .build();

        assert_eq!(params["entity"], json!(123));
        assert_eq!(params["component"], json!("Transform"));
    }

    #[test]
    fn test_custom_field() {
        let params = RpcParamsBuilder::new()
            .field("custom", json!("value"))
            .build();

        assert_eq!(params["custom"], json!("value"));
    }
}
