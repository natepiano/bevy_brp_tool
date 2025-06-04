//! Bevy plugin implementation for remote control functionality

use bevy::prelude::*;
use bevy::remote::http::RemoteHttpPlugin;
use bevy::remote::{BrpError, BrpResult, RemotePlugin, error_codes};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use serde_json::{Value, json};

use crate::DEFAULT_REMOTE_PORT;

/// Command prefix for BRP Tool specific commands
const BRP_TOOL_COMMAND_PREFIX: &str = "brp_tool/";

/// Plugin that adds remote control capabilities to a Bevy app
#[derive(Default)]
pub struct BrpToolPlugin {
    /// Optional custom port for remote control connections
    pub port: Option<u16>,
}

impl BrpToolPlugin {
    /// Create plugin with custom port
    pub fn with_port(port: u16) -> Self {
        Self { port: Some(port) }
    }
}

impl Plugin for BrpToolPlugin {
    fn build(&self, app: &mut App) {
        // Unfortunately, we can't easily intercept all Bevy RPC methods to add readiness checks
        // The best approach is to handle readiness at the client level (which we already did
        // for screenshots). For now, we'll just add our custom methods.

        // Add Bevy's remote plugins with our custom methods
        let remote_plugin = RemotePlugin::default()
            .with_method(
                format!("{}screenshot", BRP_TOOL_COMMAND_PREFIX),
                screenshot_handler,
            )
            .with_method(
                format!("{}shutdown", BRP_TOOL_COMMAND_PREFIX),
                shutdown_handler,
            );

        let http_plugin = if let Some(port) = self.port {
            RemoteHttpPlugin::default().with_port(port)
        } else {
            RemoteHttpPlugin::default()
        };

        app.add_plugins((remote_plugin, http_plugin));

        let port = self.port.unwrap_or(DEFAULT_REMOTE_PORT);
        app.add_systems(Startup, move |_world: &mut World| {
            setup_remote_methods(port);
        });
    }
}

fn setup_remote_methods(port: u16) {
    info!("Remote control enabled on http://localhost:{}", port);
    trace!("Available endpoints:");
    trace!("  - rpc.discover - Discover all available methods");
    trace!("  - bevy/query - Query entities and components");
    trace!("  - bevy/get - Get component data");
    trace!("  - bevy/list - List all entities");
    trace!("  - bevy/spawn - Spawn new entities");
    trace!("  - bevy/destroy - Destroy entities");
    trace!("  - bevy/insert - Insert components");
    trace!("  - bevy/remove - Remove components");
    trace!("  - brp_tool/screenshot - Take a screenshot");
    trace!("  - brp_tool/shutdown - Shutdown the app");
}

/// Handler for shutdown
fn shutdown_handler(In(_): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Send app exit event
    world.send_event(bevy::app::AppExit::Success);

    Ok(json!({
        "success": true,
        "message": "Shutdown initiated"
    }))
}

/// Handler for taking screenshots
fn screenshot_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Get the path from params
    let path = params
        .as_ref()
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| BrpError {
            code: error_codes::INVALID_PARAMS,
            message: "Missing 'path' parameter".to_string(),
            data: None,
        })?;

    // Convert to absolute path
    let path_buf = std::path::Path::new(path);
    let absolute_path = if path_buf.is_absolute() {
        path_buf.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| BrpError {
                code: error_codes::INTERNAL_ERROR,
                message: format!("Failed to get current directory: {}", e),
                data: None,
            })?
            .join(path_buf)
    };

    let absolute_path_str = absolute_path.to_string_lossy().to_string();

    // Log the full path before attempting screenshot
    info!("Screenshot requested for: {}", absolute_path_str);

    // Check if we have a primary window
    let window_exists = world.query::<&Window>().iter(world).any(|w| {
        info!(
            "Found window - resolution: {:?}, visible: {:?}",
            w.resolution, w.visible
        );
        true
    });

    if !window_exists {
        warn!("No windows found in the world!");
    }

    // Spawn a screenshot entity with a custom observer for debugging
    let path_for_observer = absolute_path_str.clone();
    let entity = world
        .spawn((
            Screenshot::primary_window(),
            Name::new(format!("Screenshot_{}", absolute_path_str)),
        ))
        .observe(move |trigger: Trigger<ScreenshotCaptured>| {
            info!(
                "Screenshot captured! Attempting to save to: {}",
                path_for_observer
            );
            let img = trigger.event().0.clone();
            match img.try_into_dynamic() {
                Ok(dyn_img) => {
                    match std::fs::create_dir_all(
                        std::path::Path::new(&path_for_observer)
                            .parent()
                            .unwrap_or(std::path::Path::new(".")),
                    ) {
                        Ok(_) => match dyn_img.save(&path_for_observer) {
                            Ok(_) => {
                                info!("Screenshot successfully saved to: {}", path_for_observer)
                            }
                            Err(e) => {
                                error!("Failed to save screenshot to {}: {}", path_for_observer, e)
                            }
                        },
                        Err(e) => error!(
                            "Failed to create directory for screenshot {}: {}",
                            path_for_observer, e
                        ),
                    }
                }
                Err(e) => error!("Failed to convert screenshot to dynamic image: {}", e),
            }
        })
        .id();

    info!("Screenshot entity spawned with ID: {:?}", entity);

    Ok(json!({
        "success": true,
        "path": absolute_path_str,
        "working_directory": std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("unknown")).to_string_lossy(),
        "note": "Screenshot capture initiated. The file will be saved asynchronously."
    }))
}
