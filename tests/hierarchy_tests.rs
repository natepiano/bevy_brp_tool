//! Hierarchy and parent-child relationship tests for bevy_brp_tool

mod support;
use anyhow::Result;
use serde_json::json;
use support::*;

#[tokio::test]
async fn test_reparent_to_entity() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Create two entities
    let parent_json = json!({
        &test_component_type(): {
            "value": 10,
            "name": "NewParent",
            "enabled": true
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &parent_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let parent_response = output.parse_json()?;
    let parent_id = extract_entity_id(&parent_response)?;

    let child_json = json!({
        &test_component_type(): {
            "value": 20,
            "name": "ChildToReparent",
            "enabled": true
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &child_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let child_response = output.parse_json()?;
    let child_id = extract_entity_id(&child_response)?;

    // Execute - make child a child of parent
    let output = runner
        .run_command_with_app(
            &["reparent", &child_id.to_string(), &parent_id.to_string()],
            &app,
        )
        .await?;
    assert!(output.success());

    // Verify the hierarchy was actually created
    // 1. Check that child has ChildOf component pointing to parent
    let output = runner
        .run_command_with_app(&["query", "bevy_ecs::hierarchy::ChildOf"], &app)
        .await?;
    assert!(output.success());
    let child_of_query = output.parse_json()?;
    let child_has_parent = child_of_query
        .as_array()
        .expect("Expected array of entities")
        .iter()
        .any(|entity| {
            let entity_id = entity.get("entity").and_then(|e| e.as_u64()).unwrap_or(0);
            let parent_id_in_child_of = entity
                .get("components")
                .and_then(|c| c.get("bevy_ecs::hierarchy::ChildOf"))
                .and_then(|p| p.as_u64())
                .unwrap_or(0);
            entity_id == child_id && parent_id_in_child_of == parent_id
        });

    assert!(
        child_has_parent,
        "Child entity should have ChildOf component pointing to parent"
    );

    // 2. Check that parent has Children component containing child
    let output = runner
        .run_command_with_app(
            &[
                "get",
                &parent_id.to_string(),
                "bevy_ecs::hierarchy::Children",
            ],
            &app,
        )
        .await?;
    assert!(output.success());
    let children_array = output.parse_json()?;

    // CLI returns the component data directly
    let parent_has_child = children_array
        .as_array()
        .expect("Children component should be an array")
        .iter()
        .any(|c| c.as_u64().unwrap_or(0) == child_id);

    assert!(
        parent_has_child,
        "Parent entity should have Children component containing child"
    );

    Ok(())
}

#[tokio::test]
async fn test_reparent_to_root() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Create parent and child entities with a hierarchy
    let parent_json = json!({
        &test_component_type(): {
            "value": 30,
            "name": "ParentToRemove",
            "enabled": true
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &parent_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let parent_response = output.parse_json()?;
    let parent_id = extract_entity_id(&parent_response)?;

    let child_json = json!({
        &test_component_type(): {
            "value": 40,
            "name": "ChildToOrphan",
            "enabled": true
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &child_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let child_response = output.parse_json()?;
    let child_id = extract_entity_id(&child_response)?;

    // First establish the parent-child relationship
    let output = runner
        .run_command_with_app(
            &["reparent", &child_id.to_string(), &parent_id.to_string()],
            &app,
        )
        .await?;
    assert!(output.success());

    // Verify hierarchy was created
    let output = runner
        .run_command_with_app(&["query", "bevy_ecs::hierarchy::ChildOf"], &app)
        .await?;
    assert!(output.success());
    let child_of_query_before = output.parse_json()?;
    let has_parent_before = child_of_query_before
        .as_array()
        .expect("Expected array of entities")
        .iter()
        .any(|entity| {
            let entity_id = entity.get("entity").and_then(|e| e.as_u64()).unwrap_or(0);
            entity_id == child_id
        });
    assert!(has_parent_before, "Child should have parent before removal");

    // Execute - remove parent (make top-level)
    let output = runner
        .run_command_with_app(&["reparent", &child_id.to_string(), "null"], &app)
        .await?;
    assert!(output.success());

    // Verify the parent was removed
    // 1. Check that child no longer has ChildOf component
    let output = runner
        .run_command_with_app(&["query", "bevy_ecs::hierarchy::ChildOf"], &app)
        .await?;
    assert!(output.success());
    let child_of_query_after = output.parse_json()?;
    let has_parent_after = child_of_query_after
        .as_array()
        .expect("Expected array of entities")
        .iter()
        .any(|entity| {
            let entity_id = entity.get("entity").and_then(|e| e.as_u64()).unwrap_or(0);
            entity_id == child_id
        });

    assert!(
        !has_parent_after,
        "Child entity should no longer have ChildOf component"
    );

    // 2. Check that parent no longer has Children component (or child is not in it)
    // Note: The Children component might still exist but be empty, or be removed entirely
    let output = runner
        .run_command_with_app(
            &[
                "get",
                &parent_id.to_string(),
                "bevy_ecs::hierarchy::Children",
            ],
            &app,
        )
        .await?;
    if output.success() {
        let result = output.parse_json();
        match result {
            Ok(children_array) => {
                // CLI returns component directly, check if it's an array
                if let Some(arr) = children_array.as_array() {
                    let parent_still_has_child =
                        arr.iter().any(|c| c.as_u64().unwrap_or(0) == child_id);
                    assert!(
                        !parent_still_has_child,
                        "Parent should not have child in Children component"
                    );
                }
            }
            Err(_) => {
                // If parse fails, might be an error response which is also valid
            }
        }
    }
    // Children component might be removed entirely, which is also valid

    Ok(())
}

#[tokio::test]
async fn test_reparent_invalid_child() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Create a parent
    let parent_json = json!({
        &test_component_type(): {
            "value": 30,
            "name": "Parent",
            "enabled": true
        }
    });
    let output = runner
        .run_command_with_app(&["spawn", &parent_json.to_string()], &app)
        .await?;
    assert!(output.success());
    let parent_response = output.parse_json()?;
    let parent_id = extract_entity_id(&parent_response)?;

    // Execute - try to reparent non-existent entity
    let output = runner
        .run_command_with_app(&["reparent", "999999", &parent_id.to_string()], &app)
        .await?;

    // Verify it fails
    assert!(
        !output.success(),
        "Should fail when reparenting non-existent entity"
    );

    Ok(())
}
