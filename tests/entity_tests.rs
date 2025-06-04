//! Entity management CLI tests for bevy_brp_tool

mod support;
use anyhow::Result;
use serde_json::json;
use support::*;

#[tokio::test]
async fn test_cli_list_entities() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner
        .run_command_with_app(&["list_entities"], &app)
        .await?;

    // Verify
    assert!(output.success(), "list_entities command should succeed");

    let json = output.parse_json()?;
    let entities = json
        .get("entities")
        .and_then(|e| e.as_array())
        .expect("Expected entities array");
    assert!(
        entities.len() >= 4,
        "Should have at least 4 entities from setup"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_spawn_entity() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let components_json = json!({
        &test_component_type(): {
            "value": 42,
            "name": "CliTestEntity",
            "enabled": true
        }
    });

    let output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    // Verify
    assert!(output.success(), "spawn command should succeed");

    let json = output.parse_json()?;
    let entity_id = extract_entity_id(&json)?;
    assert!(entity_id > 0);

    Ok(())
}

#[tokio::test]
async fn test_cli_spawn_entity_multiple_components() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let components_json = json!({
        &test_component_type(): {
            "value": 123,
            "name": "MultiComponentEntity",
            "enabled": false
        },
        &secondary_component_type(): {
            "data": [1.1, 2.2, 3.3]
        }
    });

    let output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    // Verify
    assert!(output.success(), "spawn command should succeed");

    let json = output.parse_json()?;
    let entity_id = extract_entity_id(&json)?;
    assert!(entity_id > 0);

    Ok(())
}

#[tokio::test]
async fn test_cli_get_entity() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity
    let components_json = json!({
        &test_component_type(): {
            "value": 999,
            "name": "GetTestEntity",
            "enabled": true
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - get the entity
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "get command should succeed");

    let json = output.parse_json()?;
    // CLI get command extracts just the component data directly
    assert_eq!(json.get("value").and_then(|v| v.as_i64()), Some(999));
    assert_eq!(
        json.get("name").and_then(|v| v.as_str()),
        Some("GetTestEntity")
    );
    assert_eq!(json.get("enabled").and_then(|v| v.as_bool()), Some(true));

    Ok(())
}

#[tokio::test]
async fn test_cli_insert_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity without secondary component
    let components_json = json!({
        &test_component_type(): {
            "value": 777,
            "name": "InsertTestEntity",
            "enabled": false
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - insert secondary component
    let component_json = json!({
        &secondary_component_type(): {
            "data": [9.9, 8.8, 7.7]
        }
    });

    let output = runner
        .run_command_with_app(
            &[
                "insert",
                &entity_id.to_string(),
                &component_json.to_string(),
            ],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "insert command should succeed");

    // Verify component was inserted by getting it
    let get_output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &secondary_component_type()],
            &app,
        )
        .await?;

    assert!(
        get_output.success(),
        "get command after insert should succeed"
    );
    let get_json = get_output.parse_json()?;
    // CLI get command extracts just the component data directly
    let data = get_json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Expected data array");
    assert_eq!(data.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_cli_remove_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity with both components
    let components_json = json!({
        &test_component_type(): {
            "value": 555,
            "name": "RemoveTestEntity",
            "enabled": true
        },
        &secondary_component_type(): {
            "data": [1.0, 2.0]
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - remove secondary component
    let output = runner
        .run_command_with_app(
            &[
                "remove",
                &entity_id.to_string(),
                &secondary_component_type(),
            ],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "remove command should succeed");

    // Verify component was removed by trying to get it
    let get_output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &secondary_component_type()],
            &app,
        )
        .await?;

    // CLI succeeds but returns the full error response (not just component data)
    assert!(get_output.success(), "get command should succeed");
    let get_json = get_output.parse_json()?;

    // When component doesn't exist, CLI returns full response with errors
    // Check that there's an error for the component
    let errors = get_json.get("errors").expect("Expected errors field");
    assert!(
        errors.get(secondary_component_type()).is_some(),
        "Should have error for removed component"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_destroy_entity() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity
    let components_json = json!({
        &test_component_type(): {
            "value": 333,
            "name": "DestroyTestEntity",
            "enabled": false
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - destroy the entity
    let output = runner
        .run_command_with_app(&["destroy", &entity_id.to_string()], &app)
        .await?;

    // Verify
    assert!(output.success(), "destroy command should succeed");

    // Verify entity was destroyed by trying to get it (should fail)
    let get_output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;

    assert!(
        !get_output.success(),
        "get command after destroy should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_entity_operations_invalid_entity() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - try operations on non-existent entity
    let non_existent_id = 999999;

    // Test get on non-existent entity
    let get_output = runner
        .run_command_with_app(
            &["get", &non_existent_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;
    assert!(
        !get_output.success(),
        "get on non-existent entity should fail"
    );

    // Test destroy on non-existent entity
    let destroy_output = runner
        .run_command_with_app(&["destroy", &non_existent_id.to_string()], &app)
        .await?;
    assert!(
        !destroy_output.success(),
        "destroy on non-existent entity should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_spawn_entity_invalid_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - try to spawn with invalid component type
    let invalid_components = json!({
        "NonExistentComponent": {
            "field": "value"
        }
    });

    let output = runner
        .run_command_with_app(&["spawn", &invalid_components.to_string()], &app)
        .await?;

    // Verify
    assert!(
        !output.success(),
        "spawn with invalid component should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_list_entity_single_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity
    let components_json = json!({
        &test_component_type(): {
            "value": 42,
            "name": "CliSingleComponentEntity",
            "enabled": true
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - list entity
    let output = runner
        .run_command_with_app(&["list_entity", &entity_id.to_string()], &app)
        .await?;

    // Verify
    assert!(output.success(), "list_entity command should succeed");

    let json = output.parse_json()?;
    assert_eq!(json.get("entity").and_then(|v| v.as_u64()), Some(entity_id));
    assert_eq!(
        json.get("generation").and_then(|v| v.as_u64()),
        Some((entity_id >> 32) as u64)
    );

    let components = json.get("components").expect("Expected components field");
    assert!(components.is_object(), "Components should be an object");

    // Should have our test component
    let test_comp = components
        .get(test_component_type())
        .expect("Expected test component");
    assert_eq!(test_comp.get("value").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(
        test_comp.get("name").and_then(|v| v.as_str()),
        Some("CliSingleComponentEntity")
    );
    assert_eq!(
        test_comp.get("enabled").and_then(|v| v.as_bool()),
        Some(true)
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_list_entity_multiple_components() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity with multiple components
    let components_json = json!({
        &test_component_type(): {
            "value": 999,
            "name": "CliMultiComponentEntity",
            "enabled": false
        },
        &secondary_component_type(): {
            "data": [1.1, 2.2, 3.3]
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - list entity
    let output = runner
        .run_command_with_app(&["list_entity", &entity_id.to_string()], &app)
        .await?;

    // Verify
    assert!(output.success(), "list_entity command should succeed");

    let json = output.parse_json()?;
    assert_eq!(json.get("entity").and_then(|v| v.as_u64()), Some(entity_id));

    let components = json.get("components").expect("Expected components field");

    // Verify test component
    let test_comp = components
        .get(test_component_type())
        .expect("Expected test component");
    assert_eq!(test_comp.get("value").and_then(|v| v.as_i64()), Some(999));
    assert_eq!(
        test_comp.get("name").and_then(|v| v.as_str()),
        Some("CliMultiComponentEntity")
    );
    assert_eq!(
        test_comp.get("enabled").and_then(|v| v.as_bool()),
        Some(false)
    );

    // Verify secondary component
    let secondary_comp = components
        .get(secondary_component_type())
        .expect("Expected secondary component");
    let data = secondary_comp
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Expected data array");
    assert_eq!(data.len(), 3);
    assert!((data[0].as_f64().unwrap() - 1.1).abs() < 0.001);
    assert!((data[1].as_f64().unwrap() - 2.2).abs() < 0.001);
    assert!((data[2].as_f64().unwrap() - 3.3).abs() < 0.001);

    Ok(())
}

#[tokio::test]
async fn test_cli_list_entity_non_existent() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - try to list a non-existent entity
    let non_existent_id = 999999;
    let output = runner
        .run_command_with_app(&["list_entity", &non_existent_id.to_string()], &app)
        .await?;

    // Verify it fails appropriately
    assert!(
        !output.success(),
        "list_entity should fail for non-existent entity"
    );

    let stderr = &output.stderr;
    assert!(
        stderr.contains("does not exist"),
        "Error should mention entity doesn't exist"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_list_entity_vs_get_consistency() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity
    let components_json = json!({
        &test_component_type(): {
            "value": 777,
            "name": "CliConsistencyTestEntity",
            "enabled": true
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - get component data via both methods
    let list_output = runner
        .run_command_with_app(&["list_entity", &entity_id.to_string()], &app)
        .await?;

    let get_output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;

    // Verify both succeed
    assert!(list_output.success(), "list_entity command should succeed");
    assert!(get_output.success(), "get command should succeed");

    // Verify consistency between list_entity and get_component
    let list_json = list_output.parse_json()?;
    let get_json = get_output.parse_json()?;

    let list_components = list_json
        .get("components")
        .expect("Expected components field in list result");
    let list_test_comp = list_components
        .get(test_component_type())
        .expect("Expected test component in list result");

    // get command extracts just the component data directly, so get_json is the component data
    // Component data should be identical
    assert_eq!(
        list_test_comp, &get_json,
        "Component data should be identical between list_entity and get_component"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_list_entities_empty() -> Result<()> {
    // Test that we can distinguish between system entities and test entities
    // Since we can't easily create a truly empty app with CLI tests, we'll verify
    // the standard test setup and ensure we can identify test entities vs system entities

    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - list all entities
    let output = runner
        .run_command_with_app(&["list_entities"], &app)
        .await?;

    // Verify
    assert!(output.success(), "list_entities command should succeed");

    let json = output.parse_json()?;
    let entities = json
        .get("entities")
        .and_then(|e| e.as_array())
        .expect("Expected entities array");

    // Count entities with test components vs system entities
    let mut test_entity_count = 0;
    let mut system_entity_count = 0;

    for entity in entities {
        let components = entity.get("components");
        if let Some(components_array) = components.and_then(|c| c.as_array()) {
            // Check if any component type name matches our test components
            let has_test_component = components_array.iter().any(|comp| {
                comp.as_str()
                    .map(|s| s == test_component_type() || s == secondary_component_type())
                    .unwrap_or(false)
            });

            if has_test_component {
                test_entity_count += 1;
            } else {
                system_entity_count += 1;
            }
        }
    }

    // Verify we have exactly 4 test entities from setup_test_world
    assert_eq!(
        test_entity_count, 4,
        "Should have exactly 4 test entities from setup"
    );

    // Verify we have minimal system entities (typically 0-3 in MinimalPlugins)
    assert!(
        system_entity_count <= 3,
        "Should have minimal system entities, got {}",
        system_entity_count
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_destroy_entity_not_exists() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - try to destroy non-existent entity (using a very high ID)
    let output = runner
        .run_command_with_app(&["destroy", "999999"], &app)
        .await?;

    // Verify it fails appropriately
    assert!(
        !output.success(),
        "Should fail when destroying non-existent entity"
    );

    // Verify error message indicates entity not found
    assert!(
        output.stderr_contains("not a valid entity"),
        "Error message should indicate entity not found"
    );

    Ok(())
}
