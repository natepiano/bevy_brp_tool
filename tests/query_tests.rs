//! Query operations CLI tests for bevy_brp_tool

mod support;
use anyhow::Result;
use serde_json::json;
use support::*;

#[tokio::test]
async fn test_cli_query_single_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner
        .run_command_with_app(&["query", &test_component_type()], &app)
        .await?;

    // Verify
    assert!(output.success(), "query command should succeed");

    let json = output.parse_json()?;
    let entities = json.as_array().expect("Expected array of entities");

    // Should have at least 2 entities with TestComponent from setup
    assert!(
        entities.len() >= 2,
        "Should have at least 2 entities with TestComponent"
    );

    // Verify structure of first entity
    let first_entity = &entities[0];
    assert!(
        first_entity.get("entity").is_some(),
        "Should have entity ID"
    );
    assert!(
        first_entity.get("components").is_some(),
        "Should have components field"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_query_multiple_components() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity with both component types
    let components_json = json!({
        &test_component_type(): {
            "value": 777,
            "name": "BothComponents",
            "enabled": true
        },
        &secondary_component_type(): {
            "data": [1.0, 2.0, 3.0]
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;
    assert!(spawn_output.success(), "spawn should succeed");

    // Execute - query for entities with both components
    let output = runner
        .run_command_with_app(
            &["query", &test_component_type(), &secondary_component_type()],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "query command should succeed");

    let json = output.parse_json()?;
    let entities = json.as_array().expect("Expected array of entities");

    // Should have at least 1 entity with both components (our spawned one + one from setup)
    assert!(
        !entities.is_empty(),
        "Should have at least 1 entity with both components"
    );

    // Find our spawned entity
    let spawned_entity = extract_entity_id(&spawn_output.parse_json()?)?;
    let found_entity = entities
        .iter()
        .find(|e| e.get("entity").and_then(|id| id.as_u64()) == Some(spawned_entity));

    assert!(
        found_entity.is_some(),
        "Should find our spawned entity in query results"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_query_nonexistent_component() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Test 1: Query for non-existent component (should return empty array)
    let output = runner
        .run_command_with_app(&["query", "NonExistentComponent"], &app)
        .await?;

    // Verify - CLI should succeed with empty results for non-existent components
    assert!(
        output.success(),
        "Query command should succeed for non-existent component, returning empty results. Got stderr: {}",
        output.stderr
    );

    // Parse the JSON output to verify it's an empty array
    let json_result = output.parse_json()?;
    let entities = json_result.as_array().expect("Expected array response");
    assert!(
        entities.is_empty(),
        "Should return empty array for non-existent component type"
    );

    // Test 2: Query for SecondaryComponent (should return exactly 2 entities from setup)
    let output2 = runner
        .run_command_with_app(&["query", &secondary_component_type()], &app)
        .await?;

    assert!(
        output2.success(),
        "Query command should succeed for SecondaryComponent"
    );

    let json_result2 = output2.parse_json()?;
    let entities2 = json_result2.as_array().expect("Expected array response");
    assert_eq!(
        entities2.len(),
        2,
        "Should have exactly 2 entities with SecondaryComponent (Entity3 and Entity4 from setup)"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_query_empty_args_fails() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - query with no components (should fail)
    let output = runner.run_command_with_app(&["query"], &app).await?;

    // Verify
    assert!(
        !output.success(),
        "query command should fail with no arguments"
    );
    assert!(
        output.stderr_contains("required arguments"),
        "Should have missing arguments error"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_list_component_types() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command_with_app(&["list"], &app).await?;

    // Verify
    assert!(output.success(), "list command should succeed");

    let json = output.parse_json()?;
    let component_types = json.as_array().expect("Expected array of component types");

    // Should have at least our test components
    assert!(
        component_types.len() >= 2,
        "Should have at least 2 component types"
    );

    let type_names: Vec<String> = component_types
        .iter()
        .filter_map(|t| t.as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        type_names.contains(&test_component_type()),
        "Should contain TestComponent"
    );
    assert!(
        type_names.contains(&secondary_component_type()),
        "Should contain SecondaryComponent"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_query_with_filter() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Spawn multiple entities with different values
    let components1_json = json!({
        &test_component_type(): {
            "value": 100,
            "name": "Entity100",
            "enabled": true
        }
    });

    let components2_json = json!({
        &test_component_type(): {
            "value": 200,
            "name": "Entity200",
            "enabled": false
        }
    });

    let spawn1_output = runner
        .run_command_with_app(&["spawn", &components1_json.to_string()], &app)
        .await?;
    assert!(spawn1_output.success(), "spawn1 should succeed");

    let spawn2_output = runner
        .run_command_with_app(&["spawn", &components2_json.to_string()], &app)
        .await?;
    assert!(spawn2_output.success(), "spawn2 should succeed");

    // Execute - query for entities with TestComponent
    let output = runner
        .run_command_with_app(&["query", &test_component_type()], &app)
        .await?;

    // Verify
    assert!(output.success(), "query command should succeed");

    let json = output.parse_json()?;
    let entities = json.as_array().expect("Expected array of entities");

    // Should have at least our spawned entities plus the setup ones
    assert!(
        entities.len() >= 4,
        "Should have at least 4 entities with TestComponent"
    );

    // Verify both our entities are in the results
    let entity1_id = extract_entity_id(&spawn1_output.parse_json()?)?;
    let entity2_id = extract_entity_id(&spawn2_output.parse_json()?)?;

    let found_entity1 = entities
        .iter()
        .any(|e| e.get("entity").and_then(|id| id.as_u64()) == Some(entity1_id));
    let found_entity2 = entities
        .iter()
        .any(|e| e.get("entity").and_then(|id| id.as_u64()) == Some(entity2_id));

    assert!(found_entity1, "Should find first spawned entity");
    assert!(found_entity2, "Should find second spawned entity");

    Ok(())
}

#[tokio::test]
async fn test_cli_query_verify_component_data() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Spawn an entity with specific data
    let components_json = json!({
        &test_component_type(): {
            "value": 42,
            "name": "SpecificEntity",
            "enabled": true
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;
    assert!(spawn_output.success(), "spawn should succeed");
    let spawned_entity_id = extract_entity_id(&spawn_output.parse_json()?)?;

    // Execute - query for entities with TestComponent
    let output = runner
        .run_command_with_app(&["query", &test_component_type()], &app)
        .await?;

    // Verify
    assert!(output.success(), "query command should succeed");

    let json = output.parse_json()?;
    let entities = json.as_array().expect("Expected array of entities");

    // Find our specific entity
    let found_entity = entities
        .iter()
        .find(|e| e.get("entity").and_then(|id| id.as_u64()) == Some(spawned_entity_id));

    assert!(found_entity.is_some(), "Should find our spawned entity");
    let entity = found_entity.unwrap();

    // Verify component data is included
    let components = entity.get("components").expect("Should have components");
    let test_comp = components
        .get(test_component_type())
        .expect("Should have test component");

    assert_eq!(test_comp.get("value").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(
        test_comp.get("name").and_then(|v| v.as_str()),
        Some("SpecificEntity")
    );
    assert_eq!(
        test_comp.get("enabled").and_then(|v| v.as_bool()),
        Some(true)
    );

    Ok(())
}
