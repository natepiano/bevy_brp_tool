//! Support utilities for the BRP CLI tool
//!
//! This module provides common functionality used throughout the CLI,
//! organized into logical submodules.

// Module declarations
mod app_detection;
mod binary_discovery;
mod entity;
mod json;
mod polling;
mod port_utils;

// Re-export public functions from submodules
pub use app_detection::detect_bevy_app;
pub use binary_discovery::find_workspace_binary_with_target_dir;
pub use entity::parse_entity_arg;
pub use json::{format_json, parse_json_object, parse_json_value, print_json};
pub use polling::poll_until_ready;
pub use port_utils::{is_connection_error, is_port_available, wait_for_port_connectable};
