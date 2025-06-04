use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use serde_json::json;
use tokio::fs;
use tokio::time::{sleep, timeout};
use tokio_stream::StreamExt;

use super::types::Commands;
use crate::cli::cli_client::wait_for_app_ready;
use crate::cli::client::RemoteClient;
use crate::cli::constants::{
    BEVY_GET_RESOURCE, BEVY_GET_WATCH, BEVY_LIST_RESOURCES, BEVY_LIST_WATCH, BEVY_REGISTRY_SCHEMA,
    BEVY_REMOVE_RESOURCE, BEVY_REPARENT,
};
use crate::cli::rpc_params_builder::RpcParamsBuilder;
use crate::cli::support::{parse_json_object, parse_json_value, print_json};

/// Handle a streaming response with Ctrl+C interruption support
async fn handle_stream_response(
    mut stream: impl StreamExt<Item = Result<serde_json::Value, anyhow::Error>> + Unpin,
    entity_msg: &str,
) -> Result<()> {
    println!(
        "Streaming component changes for {} (press Ctrl+C to stop):",
        entity_msg
    );

    // Set up Ctrl+C handler
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    println!("[Waiting for updates... Press Ctrl+C to stop]\n");

    // Process stream until Ctrl+C
    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                println!("\n[Stream interrupted by user]");
                break;
            }
            update = stream.next() => {
                match update {
                    Some(Ok(value)) => {
                        print_json(&value)?;
                        println!(); // Add spacing between updates
                    }
                    Some(Err(e)) => {
                        eprintln!("Stream error: {}", e);
                        break;
                    }
                    None => {
                        println!("[Stream ended]");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Execute a command in standalone mode (app already running)
pub async fn execute_standalone_command(client: &RemoteClient, command: Commands) -> Result<()> {
    // Wait for app to be ready before executing any command
    // Exceptions:
    // - Ready command (to avoid circular dependency)
    // - Workflows command (just displays help text, no app interaction)
    match &command {
        Commands::Ready => {
            // This command doesn't need app readiness check
        }
        _ => {
            wait_for_app_ready(client).await?;
        }
    }

    match command {
        Commands::Destroy { entity } => {
            let result = client.destroy_entity(entity).await?;
            print_json(&result)?;
        }

        Commands::Get { entity, component } => {
            let result = client.get_component(entity, &component).await?;
            // Extract just the component data from the result
            if let Some(components) = result.get("components") {
                if let Some(component_data) = components.get(&component) {
                    print_json(component_data)?;
                } else {
                    print_json(&result)?;
                }
            } else {
                print_json(&result)?;
            }
        }

        Commands::GetResource { resource } => {
            let result = client
                .call_brp_method(
                    BEVY_GET_RESOURCE,
                    RpcParamsBuilder::new().resource(resource).build(),
                )
                .await?;
            print_json(&result)?;
        }

        Commands::GetWatch { entity, components } => {
            // Start streaming
            let components_refs: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
            let method = BEVY_GET_WATCH;
            let stream = client
                .stream_request(
                    method,
                    RpcParamsBuilder::new()
                        .entity(entity)
                        .component_list(components_refs)
                        .build(),
                )
                .await?;

            // Use the common stream handler
            handle_stream_response(stream, &format!("entity {}", entity)).await?;
        }

        Commands::Insert { entity, components } => {
            let obj = parse_json_object(&components, "Insert")?;
            for (component_type, component_data) in obj {
                let result = client
                    .insert_component(entity, &component_type, component_data)
                    .await?;
                print_json(&result)?;
            }
        }

        Commands::InsertResource { data } => {
            let obj = parse_json_object(&data, "InsertResource")?;
            for (resource_type, resource_data) in obj {
                let result = client
                    .insert_resource(&resource_type, resource_data)
                    .await?;
                print_json(&result)?;
            }
        }

        Commands::List => {
            let result = client.list_entities().await?;
            print_json(&result)?;
        }

        Commands::ListResources => {
            let result = client
                .call_brp_method(BEVY_LIST_RESOURCES, serde_json::Value::Null)
                .await?;
            print_json(&result)?;
        }

        Commands::ListEntity { entity } => {
            let result = client.list_entity(entity).await?;
            print_json(&result)?;
        }

        Commands::ListEntities => {
            // BRP doesn't have a direct "get all components for entity" method
            // We'll use a different approach: get all component types, then query for each type
            // This is more comprehensive than trying to get components per entity

            // First, get all available component types
            let component_types_result = client.list_entities().await?;
            let mut component_types = Vec::new();

            if let Some(types_array) = component_types_result.as_array() {
                for component_type in types_array {
                    if let Some(type_name) = component_type.as_str() {
                        component_types.push(type_name.to_string());
                    }
                }
            }

            // Now build a map of entity_id -> component_types
            let mut entity_components_map: std::collections::HashMap<u64, Vec<String>> =
                std::collections::HashMap::new();

            // Query for component types in parallel using tokio::spawn
            // We'll process them in batches to avoid overwhelming the system
            const BATCH_SIZE: usize = 10;

            for chunk in component_types.chunks(BATCH_SIZE) {
                let mut tasks = Vec::new();

                // Spawn tasks for this batch
                for component_type in chunk {
                    let client = client.clone();
                    let component_type = component_type.clone();

                    let task = tokio::spawn(async move {
                        let result = client.query_entities(vec![&component_type]).await;
                        (component_type, result)
                    });

                    tasks.push(task);
                }

                // Wait for all tasks in this batch to complete
                for task in tasks {
                    if let Ok((component_type, Ok(query_result))) = task.await {
                        if let Some(query_array) = query_result.as_array() {
                            for entity_data in query_array {
                                if let Some(entity_id) =
                                    entity_data.get("entity").and_then(|e| e.as_u64())
                                {
                                    entity_components_map
                                        .entry(entity_id)
                                        .or_default()
                                        .push(component_type.clone());
                                }
                            }
                        }
                    }
                }
            }

            // Convert to the expected output format
            let mut entities = Vec::new();
            for (entity_id, component_names) in entity_components_map {
                // Calculate generation from entity ID (upper 32 bits)
                let generation = (entity_id >> 32) as u32;

                entities.push(json!({
                    "entity": entity_id,
                    "generation": generation,
                    "components": component_names
                }));
            }

            // Sort by entity ID for consistent output
            entities.sort_by(|a, b| {
                let a_id = a.get("entity").and_then(|v| v.as_u64()).unwrap_or(0);
                let b_id = b.get("entity").and_then(|v| v.as_u64()).unwrap_or(0);
                a_id.cmp(&b_id)
            });

            let result = json!({
                "entities": entities,
                "total_count": entities.len()
            });

            print_json(&result)?;
        }

        Commands::ListWatch { entity } => {
            // Start streaming
            let method = BEVY_LIST_WATCH;
            let stream = client
                .stream_request(method, RpcParamsBuilder::new().entity(entity).build())
                .await?;

            // Use the common stream handler
            handle_stream_response(stream, &format!("entity {}", entity)).await?;
        }

        Commands::Methods => {
            let result = client
                .call_brp_method("rpc.discover", serde_json::Value::Null)
                .await?;
            print_json(&result)?;
        }

        Commands::MutateComponent {
            entity,
            component,
            patch,
        } => {
            let patch_value = parse_json_value(&patch)?;
            let result = client
                .mutate_component(entity, &component, patch_value)
                .await?;
            print_json(&result)?;
        }

        Commands::MutateResource { resource, patch } => {
            let patch_value = parse_json_value(&patch)?;
            let result = client.mutate_resource(&resource, patch_value).await?;
            print_json(&result)?;
        }

        Commands::Query { components } => {
            let components: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
            let result = client.query_entities(components).await?;
            print_json(&result)?;
        }

        Commands::Ready => {
            let result = client.is_ready().await?;
            let response = json!({
                "ready": result,
                "message": if result {
                    "App is ready and responding to BRP commands"
                } else {
                    "App is not responding to BRP commands"
                }
            });
            print_json(&response)?;
        }

        Commands::Remove { entity, component } => {
            let result = client.remove_component(entity, &component).await?;
            print_json(&result)?;
        }

        Commands::RemoveResource { resource } => {
            let result = client
                .call_brp_method(
                    BEVY_REMOVE_RESOURCE,
                    RpcParamsBuilder::new().resource(resource).build(),
                )
                .await?;
            print_json(&result)?;
        }

        Commands::Reparent { child, parent } => {
            let parent_value = if parent == "null" {
                serde_json::Value::Null
            } else {
                json!(parent.parse::<u64>()?)
            };
            let result = client
                .call_brp_method(
                    BEVY_REPARENT,
                    RpcParamsBuilder::new()
                        .entities(vec![child])
                        .parent(parent_value)
                        .build(),
                )
                .await?;
            print_json(&result)?;
        }

        Commands::Screenshot { path } => {
            let mut result = client.take_screenshot(&path).await?;

            // Poll for the file to be written with non-zero size
            let file_path = Path::new(&path);
            let poll_duration = Duration::from_millis(100);
            let timeout_duration = Duration::from_secs(5);

            let poll_result = timeout(timeout_duration, async {
                loop {
                    match fs::metadata(&file_path).await {
                        Ok(metadata) if metadata.len() > 0 => {
                            return Ok::<(), std::io::Error>(());
                        }
                        _ => {
                            sleep(poll_duration).await;
                        }
                    }
                }
            })
            .await;

            match poll_result {
                Ok(Ok(())) => {
                    // File was successfully written
                    if let Some(obj) = result.as_object_mut() {
                        obj.insert("file_written".to_string(), json!(true));
                        obj.insert("note".to_string(), json!("Screenshot saved successfully."));
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    // Timeout or error
                    if let Some(obj) = result.as_object_mut() {
                        obj.insert("file_written".to_string(), json!(false));
                        obj.insert(
                            "error".to_string(),
                            json!("Screenshot file was not written within timeout period"),
                        );
                    }
                    anyhow::bail!("Screenshot file was not written within 5 seconds");
                }
            }

            print_json(&result)?;
        }

        Commands::Shutdown => {
            let result = client.shutdown().await?;
            print_json(&result)?;
        }

        Commands::Spawn { components } => {
            let json_value = parse_json_value(&components)?;
            let result = client.spawn_entity(json_value).await?;
            print_json(&result)?;
        }

        Commands::Schema {
            with_crates,
            without_crates,
            with_types,
            without_types,
        } => {
            let mut params = serde_json::Map::new();

            if let Some(crates) = with_crates {
                params.insert("with_crates".to_string(), json!(crates));
            }
            if let Some(crates) = without_crates {
                params.insert("without_crates".to_string(), json!(crates));
            }
            if let Some(types) = with_types {
                params.insert("with_types".to_string(), json!(types));
            }
            if let Some(types) = without_types {
                params.insert("without_types".to_string(), json!(types));
            }

            let result = client
                .call_brp_method(BEVY_REGISTRY_SCHEMA, json!(params))
                .await?;
            print_json(&result)?;
        }

        Commands::Raw { args } => {
            // Raw commands are method calls that go directly to the server
            if args.is_empty() {
                anyhow::bail!("Raw command requires at least a method name");
            }

            let method = &args[0];
            let params = if args.len() > 1 {
                // Try to parse remaining args as JSON
                let remaining = args[1..].join(" ");
                if remaining.trim().is_empty() {
                    serde_json::Value::Null
                } else {
                    match serde_json::from_str(&remaining) {
                        Ok(json) => json,
                        Err(_) => {
                            // If not valid JSON, treat as a simple string parameter
                            json!(remaining)
                        }
                    }
                }
            } else {
                serde_json::Value::Null
            };

            let result = client.call_brp_method(method, params).await?;
            print_json(&result)?;
        }
    }

    Ok(())
}
