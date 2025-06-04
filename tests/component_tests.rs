//! Component operation tests for bevy_brp_tool

mod support;
use anyhow::Result;
use serde_json::json;
use support::*;

#[tokio::test]
async fn test_get_component_exists() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Find an entity with TestComponent (from setup)
    let output = runner
        .run_command_with_app(&["query", &test_component_type()], &app)
        .await?;
    assert!(output.success());
    let entities = output.parse_json()?;
    let entity_id = entities
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|e| e.get("entity"))
        .and_then(|id| id.as_u64())
        .expect("Should have at least one entity with TestComponent");

    // Execute
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());
    let component_data = output.parse_json()?;

    // Verify
    assert!(component_data.get("value").is_some());
    assert!(component_data.get("name").is_some());
    assert!(component_data.get("enabled").is_some());

    Ok(())
}

#[tokio::test]
async fn test_get_component_missing() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Find an entity with TestComponent but not SecondaryComponent
    let output = runner
        .run_command_with_app(&["query", &test_component_type()], &app)
        .await?;
    assert!(output.success());
    let test_entities = output.parse_json()?;

    let output = runner
        .run_command_with_app(&["query", &secondary_component_type()], &app)
        .await?;
    assert!(output.success());
    let secondary_entities = output.parse_json()?;

    let test_ids: Vec<u64> = test_entities
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e.get("entity").and_then(|id| id.as_u64()))
        .collect();

    let secondary_ids: Vec<u64> = secondary_entities
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e.get("entity").and_then(|id| id.as_u64()))
        .collect();

    // Find an entity with TestComponent but not SecondaryComponent
    let entity_id = test_ids
        .iter()
        .find(|id| !secondary_ids.contains(id))
        .copied()
        .expect("Should have an entity with TestComponent but not SecondaryComponent");

    // Execute - try to get non-existent component
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &secondary_component_type()],
            &app,
        )
        .await?;
    assert!(output.success()); // Command succeeds but returns error in JSON
    let response = output.parse_json()?;

    // Verify it returns an error in the response
    assert!(
        response.get("errors").is_some(),
        "Should return errors when getting missing component"
    );

    Ok(())
}

#[tokio::test]
async fn test_insert_component_new() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Create an entity without SecondaryComponent
    let spawn_json = json!({
        &test_component_type(): {
            "value": 50,
            "name": "TestInsert",
            "enabled": true
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &spawn_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let spawn_response = output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_response)?;

    // Execute - add SecondaryComponent
    let component_json = json!({
        &secondary_component_type(): {
            "data": [10.0, 20.0, 30.0]
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
    if !output.success() {
        eprintln!("Insert failed. stderr: {}", output.stderr);
        eprintln!("stdout: {}", output.stdout);
    }
    assert!(output.success());

    // Verify component is now present
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &secondary_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());
    let component = output.parse_json()?;
    let data = component
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Expected data array");
    assert_eq!(data.len(), 3);
    assert_eq!(data[0].as_f64(), Some(10.0));

    Ok(())
}

#[tokio::test]
async fn test_insert_component_replace() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Create an entity with TestComponent
    let spawn_json = json!({
        &test_component_type(): {
            "value": 100,
            "name": "original",
            "enabled": true
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &spawn_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let spawn_response = output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_response)?;

    // Execute - replace with new values
    let component_json = json!({
        &test_component_type(): {
            "value": 200,
            "name": "replaced",
            "enabled": false
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
    assert!(output.success());

    // Verify replacement succeeded
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());
    let component = output.parse_json()?;
    assert_eq!(component.get("value").and_then(|v| v.as_i64()), Some(200));
    assert_eq!(
        component.get("name").and_then(|v| v.as_str()),
        Some("replaced")
    );
    assert_eq!(
        component.get("enabled").and_then(|v| v.as_bool()),
        Some(false)
    );

    Ok(())
}

#[tokio::test]
async fn test_remove_component_exists() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Create an entity with multiple components
    let spawn_json = json!({
        &test_component_type(): {
            "value": 100,
            "name": "TestRemove",
            "enabled": true
        },
        &secondary_component_type(): {
            "data": [1.0, 2.0]
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &spawn_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let spawn_response = output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_response)?;

    // Verify component exists
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());

    // Execute - remove component
    let output = runner
        .run_command_with_app(
            &["remove", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());

    // Verify component is gone
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());
    let response = output.parse_json()?;
    // Should have errors for missing component
    assert!(
        response.get("errors").is_some(),
        "Should return errors when getting removed component"
    );

    Ok(())
}

#[tokio::test]
async fn test_remove_component_missing() -> Result<()> {
    // Test that removing a non-existent component succeeds (idempotent behavior)
    // This validates that BRP remove operations don't error on missing components

    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Create an entity without SecondaryComponent
    let spawn_json = json!({
        &test_component_type(): {
            "value": 60,
            "name": "TestRemoveMissing",
            "enabled": false
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &spawn_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let spawn_response = output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_response)?;

    // Execute - try to remove non-existent component
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

    // BRP remove should succeed silently (idempotent behavior)
    // Removing a non-existent component should not error
    assert!(
        output.success(),
        "Remove should succeed even for non-existent components (idempotent)"
    );

    // Verify the component was never there (should still not exist)
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &secondary_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());
    let get_result = output.parse_json()?;
    assert!(
        get_result.get("errors").is_some(),
        "Component should not exist after removing non-existent component"
    );

    Ok(())
}

#[tokio::test]
async fn test_mutate_component_partial() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Create an entity with TestComponent
    let spawn_json = json!({
        &test_component_type(): {
            "value": 100,
            "name": "original",
            "enabled": true
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &spawn_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let spawn_response = output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_response)?;

    // Execute - partial update (only change value and enabled)
    // Use mutate_component CLI command
    let patch_json = json!({
        "value": 200,
        "enabled": false
    });
    let output = runner
        .run_command_with_app(
            &[
                "mutate_component",
                &entity_id.to_string(),
                &test_component_type(),
                &patch_json.to_string(),
            ],
            &app,
        )
        .await?;
    assert!(output.success());

    // Verify partial update
    let output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());
    let component = output.parse_json()?;
    assert_eq!(component.get("value").and_then(|v| v.as_i64()), Some(200));
    assert_eq!(
        component.get("name").and_then(|v| v.as_str()),
        Some("original")
    );
    assert_eq!(
        component.get("enabled").and_then(|v| v.as_bool()),
        Some(false)
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity with a test component
    let components_json = json!({
        &test_component_type(): {
            "value": 100,
            "name": "OriginalName",
            "enabled": true
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - mutate the component
    let patch = json!({
        "value": 200,
        "name": "UpdatedName"
    });

    let output = runner
        .run_command_with_app(
            &[
                "mutate_component",
                &entity_id.to_string(),
                &test_component_type(),
                &patch.to_string(),
            ],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "mutate_component command should succeed");

    // Verify the changes by getting the component
    let get_output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;

    assert!(
        get_output.success(),
        "get command after mutate should succeed"
    );
    let get_json = get_output.parse_json()?;

    assert_eq!(get_json.get("value").and_then(|v| v.as_i64()), Some(200));
    assert_eq!(
        get_json.get("name").and_then(|v| v.as_str()),
        Some("UpdatedName")
    );
    // enabled should remain unchanged
    assert_eq!(
        get_json.get("enabled").and_then(|v| v.as_bool()),
        Some(true)
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_component_partial_update() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity with a test component
    let components_json = json!({
        &test_component_type(): {
            "value": 300,
            "name": "PartialTest",
            "enabled": false
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - mutate only one field
    let patch = json!({
        "enabled": true
    });

    let output = runner
        .run_command_with_app(
            &[
                "mutate_component",
                &entity_id.to_string(),
                &test_component_type(),
                &patch.to_string(),
            ],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "mutate_component command should succeed");

    // Verify the changes by getting the component
    let get_output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &test_component_type()],
            &app,
        )
        .await?;

    assert!(
        get_output.success(),
        "get command after mutate should succeed"
    );
    let get_json = get_output.parse_json()?;

    // Only enabled should change
    assert_eq!(get_json.get("value").and_then(|v| v.as_i64()), Some(300));
    assert_eq!(
        get_json.get("name").and_then(|v| v.as_str()),
        Some("PartialTest")
    );
    assert_eq!(
        get_json.get("enabled").and_then(|v| v.as_bool()),
        Some(true)
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_component_nonexistent_entity() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - try to mutate component on non-existent entity
    let patch = json!({
        "value": 999
    });

    let output = runner
        .run_command_with_app(
            &[
                "mutate_component",
                "999999",
                &test_component_type(),
                &patch.to_string(),
            ],
            &app,
        )
        .await?;

    // Verify
    assert!(
        !output.success(),
        "mutate_component on non-existent entity should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_component_nonexistent_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity without the component
    let components_json = json!({
        &test_component_type(): {
            "value": 400,
            "name": "NoSecondaryComponent",
            "enabled": true
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - try to mutate a component that doesn't exist on the entity
    let patch = json!({
        "data": [1.0, 2.0, 3.0]
    });

    let output = runner
        .run_command_with_app(
            &[
                "mutate_component",
                &entity_id.to_string(),
                &secondary_component_type(),
                &patch.to_string(),
            ],
            &app,
        )
        .await?;

    // Verify
    assert!(
        !output.success(),
        "mutate_component on non-existent component should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_component_invalid_json() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity with a test component
    let components_json = json!({
        &test_component_type(): {
            "value": 500,
            "name": "InvalidJsonTest",
            "enabled": true
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - try to mutate with invalid JSON
    let output = runner
        .run_command_with_app(
            &[
                "mutate_component",
                &entity_id.to_string(),
                &test_component_type(),
                "{invalid json}",
            ],
            &app,
        )
        .await?;

    // Verify
    assert!(
        !output.success(),
        "mutate_component with invalid JSON should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_component_with_secondary_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity with a secondary component
    let components_json = json!({
        &secondary_component_type(): {
            "data": [1.1, 2.2, 3.3]
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;

    let spawn_json = spawn_output.parse_json()?;
    let entity_id = extract_entity_id(&spawn_json)?;

    // Execute - mutate the secondary component
    let patch = json!({
        "data": [9.9, 8.8, 7.7, 6.6]
    });

    let output = runner
        .run_command_with_app(
            &[
                "mutate_component",
                &entity_id.to_string(),
                &secondary_component_type(),
                &patch.to_string(),
            ],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "mutate_component command should succeed");

    // Verify the changes by getting the component
    let get_output = runner
        .run_command_with_app(
            &["get", &entity_id.to_string(), &secondary_component_type()],
            &app,
        )
        .await?;

    assert!(
        get_output.success(),
        "get command after mutate should succeed"
    );
    let get_json = get_output.parse_json()?;

    let data = get_json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Expected data array");
    assert_eq!(data.len(), 4);

    // Use approximate comparison for floating point values
    let tolerance = 0.01;
    assert!((data[0].as_f64().unwrap() - 9.9).abs() < tolerance);
    assert!((data[1].as_f64().unwrap() - 8.8).abs() < tolerance);
    assert!((data[2].as_f64().unwrap() - 7.7).abs() < tolerance);
    assert!((data[3].as_f64().unwrap() - 6.6).abs() < tolerance);

    Ok(())
}
