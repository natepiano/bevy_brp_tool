//! Client for controlling Bevy apps remotely

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde_json::{Value, json};
use tokio_stream::Stream;

use super::constants::{
    BEVY_DESTROY, BEVY_GET, BEVY_INSERT, BEVY_INSERT_RESOURCE, BEVY_LIST, BEVY_MUTATE_COMPONENT,
    BEVY_MUTATE_RESOURCE, BEVY_QUERY, BEVY_REMOVE, BEVY_SPAWN, BRP_TOOL_SCREENSHOT,
    BRP_TOOL_SHUTDOWN,
};
use super::rpc_params_builder::RpcParamsBuilder;
use super::sse::parse_sse_stream;
use super::support::is_connection_error;

/// Client for sending remote control commands to a Bevy application.
///
/// This client is primarily intended for integration testing. For interactive
/// control of Bevy apps, use the `brp` CLI tool.
#[derive(Clone)]
pub struct RemoteClient {
    base_url: String,
    port: u16,
    client: reqwest::Client,
}

impl RemoteClient {
    /// Create a new remote client connecting to the specified port
    pub fn new(port: u16) -> Self {
        Self {
            base_url: format!("http://localhost:{}", port),
            port,
            client: reqwest::Client::new(),
        }
    }

    /// Get the port this client is connected to
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Generate a unique request ID using current timestamp
    ///
    /// We use timestamp-based IDs instead of a counter to avoid needing mutable
    /// state. This allows methods like `is_ready()` to be immutable. The timestamp
    /// provides sufficient uniqueness for our synchronous request/response pattern,
    /// and would support future async patterns if needed.
    fn generate_request_id() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }

    /// Send a JSON-RPC request
    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let request_id = Self::generate_request_id();

        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "id": request_id,
            "params": params
        });

        let response = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await?;

        let result: Value = response.json().await?;

        if let Some(error) = result.get("error") {
            // Try to extract error code and message for better error handling
            if let Some(error_obj) = error.as_object() {
                let code = error_obj.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                let message = error_obj
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                anyhow::bail!("Remote error [{}]: {}", code, message);
            } else {
                anyhow::bail!("Remote error: {}", error);
            }
        }

        Ok(result["result"].clone())
    }

    /// Query entities with specific components
    pub async fn query_entities(&self, components: Vec<&str>) -> Result<Value> {
        self.request(
            BEVY_QUERY,
            RpcParamsBuilder::new()
                .field("data", json!({ "components": components }))
                .build(),
        )
        .await
    }

    /// Get all entities
    pub async fn list_entities(&self) -> Result<Value> {
        self.request(BEVY_LIST, serde_json::Value::Null).await
    }

    /// Get all component data for a single entity
    /// This is a composite method that fetches all component types, then gets data for each
    /// component that exists on the entity
    pub async fn list_entity(&self, entity: u64) -> Result<Value> {
        // First, get all available component types
        let component_types_result = self.list_entities().await?;
        let mut component_types = Vec::new();

        if let Some(types_array) = component_types_result.as_array() {
            for component_type in types_array {
                if let Some(type_name) = component_type.as_str() {
                    component_types.push(type_name);
                }
            }
        }

        // Now get data for each component type that exists on this entity
        let mut components = serde_json::Map::new();

        for component_type in &component_types {
            if let Ok(component_result) = self.get_component(entity, component_type).await {
                // Extract the component data if it exists
                if let Some(components_obj) = component_result.get("components") {
                    if let Some(component_data) = components_obj.get(component_type) {
                        // Only include if the component actually exists (not null)
                        if !component_data.is_null() {
                            components.insert(component_type.to_string(), component_data.clone());
                        }
                    }
                }
            }
        }

        // Check if entity exists (has any components)
        if components.is_empty() {
            // Try to query for this specific entity to see if it exists at all
            let mut entity_exists = false;
            for component_type in &component_types {
                if let Ok(query_result) = self.query_entities(vec![component_type]).await {
                    if let Some(query_array) = query_result.as_array() {
                        for entity_data in query_array {
                            if let Some(entity_id) =
                                entity_data.get("entity").and_then(|e| e.as_u64())
                            {
                                if entity_id == entity {
                                    entity_exists = true;
                                    break;
                                }
                            }
                        }
                        if entity_exists {
                            break;
                        }
                    }
                }
            }

            if !entity_exists {
                anyhow::bail!("Entity {} does not exist", entity);
            }
        }

        // Calculate generation from entity ID (upper 32 bits)
        let generation = (entity >> 32) as u32;

        Ok(json!({
            "entity": entity,
            "generation": generation,
            "components": components
        }))
    }

    /// Get component data for an entity
    pub async fn get_component(&self, entity: u64, component: &str) -> Result<Value> {
        self.request(
            BEVY_GET,
            RpcParamsBuilder::new()
                .entity(entity)
                .component_list(vec![component])
                .build(),
        )
        .await
    }

    /// Insert a component on an entity
    pub async fn insert_component(
        &self,
        entity: u64,
        component: &str,
        data: Value,
    ) -> Result<Value> {
        self.request(
            BEVY_INSERT,
            RpcParamsBuilder::new()
                .entity(entity)
                .component_data(component, data)
                .build(),
        )
        .await
    }

    /// Spawn a new entity
    pub async fn spawn_entity(&self, components: Value) -> Result<Value> {
        self.request(
            BEVY_SPAWN,
            RpcParamsBuilder::new().components(components).build(),
        )
        .await
    }

    /// Destroy an entity
    pub async fn destroy_entity(&self, entity: u64) -> Result<Value> {
        self.request(BEVY_DESTROY, RpcParamsBuilder::new().entity(entity).build())
            .await
    }

    /// Take a screenshot (requires custom method on server)
    pub async fn take_screenshot(&self, path: &str) -> Result<Value> {
        self.request(
            BRP_TOOL_SCREENSHOT,
            RpcParamsBuilder::new().path(path).build(),
        )
        .await
    }

    /// Shutdown the app (requires custom method on server)
    pub async fn shutdown(&self) -> Result<Value> {
        self.request(BRP_TOOL_SHUTDOWN, json!({})).await
    }

    /// Execute a BRP (Bevy Remote Protocol) method
    pub async fn call_brp_method(&self, method: &str, params: Value) -> Result<Value> {
        self.request(method, params).await
    }

    // All test convenience methods have been removed.
    // Tests now use the CLI directly via CliTestRunner.

    /// Check if the app is ready by polling with a standard BRP command
    pub async fn is_ready(&self) -> Result<bool> {
        // Try a simple BRP call to check if the app is responsive
        // We use bevy/list as it's a lightweight command that should always work
        match self.request(BEVY_LIST, serde_json::Value::Null).await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if this is a connection error (server not running)
                // vs other errors (server running but having issues)
                let error_str = e.to_string();
                if is_connection_error(&error_str) {
                    // Propagate connection errors
                    Err(e)
                } else {
                    // For other errors, assume server is running but not ready
                    Ok(false)
                }
            }
        }
    }

    /// Remove a component from an entity
    pub async fn remove_component(&self, entity: u64, component: &str) -> Result<Value> {
        self.request(
            BEVY_REMOVE,
            RpcParamsBuilder::new()
                .entity(entity)
                .component_list(vec![component])
                .build(),
        )
        .await
    }

    /// Mutate a single component field
    pub async fn mutate_component_field(
        &self,
        entity: u64,
        component: &str,
        path: &str,
        value: Value,
    ) -> Result<Value> {
        self.request(
            BEVY_MUTATE_COMPONENT,
            RpcParamsBuilder::new()
                .entity(entity)
                .component(component)
                .field("path", json!(path))
                .field("value", value)
                .build(),
        )
        .await
    }

    /// Mutate multiple component fields (convenience method)
    /// Takes a JSON object and applies each field as a separate mutation
    pub async fn mutate_component(
        &self,
        entity: u64,
        component: &str,
        patch: Value,
    ) -> Result<Value> {
        if let Some(obj) = patch.as_object() {
            let mut last_result = json!(null);
            for (field_name, field_value) in obj {
                last_result = self
                    .mutate_component_field(entity, component, field_name, field_value.clone())
                    .await?;
            }
            Ok(last_result)
        } else {
            anyhow::bail!("Patch must be a JSON object with field names and values");
        }
    }

    /// Insert or update a resource
    pub async fn insert_resource(&self, resource_type: &str, data: Value) -> Result<Value> {
        self.request(
            BEVY_INSERT_RESOURCE,
            RpcParamsBuilder::new()
                .resource(resource_type)
                .field("value", data)
                .build(),
        )
        .await
    }

    /// Mutate a single resource field
    pub async fn mutate_resource_field(
        &self,
        resource: &str,
        path: &str,
        value: Value,
    ) -> Result<Value> {
        self.request(
            BEVY_MUTATE_RESOURCE,
            RpcParamsBuilder::new()
                .resource(resource)
                .field("path", json!(path))
                .field("value", value)
                .build(),
        )
        .await
    }

    /// Mutate multiple resource fields (convenience method)
    /// Takes a JSON object and applies each field as a separate mutation
    pub async fn mutate_resource(&self, resource: &str, patch: Value) -> Result<Value> {
        if let Some(obj) = patch.as_object() {
            let mut last_result = json!(null);
            for (field_name, field_value) in obj {
                last_result = self
                    .mutate_resource_field(resource, field_name, field_value.clone())
                    .await?;
            }
            Ok(last_result)
        } else {
            anyhow::bail!("Patch must be a JSON object with field names and values");
        }
    }

    /// Send a streaming JSON-RPC request (for SSE endpoints)
    pub async fn stream_request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<impl Stream<Item = Result<Value>>> {
        let request_id = Self::generate_request_id();

        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "id": request_id,
            "params": params
        });

        let response = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await?;

        // Check for non-2xx status codes
        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("HTTP error {}: {}", status, body);
        }

        // Convert response to byte stream
        let stream = response.bytes_stream();

        // Parse SSE events from the stream
        Ok(parse_sse_stream(stream))
    }
}
