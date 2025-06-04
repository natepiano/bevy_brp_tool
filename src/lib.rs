//! Bevy Remote Protocol (BRP) tool for controlling Bevy applications
//!
//! This crate provides the `BrpToolPlugin` which adds remote control server
//! capabilities to your Bevy application, allowing it to be controlled via HTTP/JSON-RPC
//! commands.
//!
//! # Library Usage (for Bevy apps)
//!
//! Add the plugin to your Bevy app:
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_brp_tool::BrpToolPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(BrpToolPlugin::default())
//!     .run();
//! ```
//!
//! # CLI Tool
//!
//! This crate also provides a `brp` binary command-line tool for sending
//! commands to Bevy apps that have the plugin installed. See the CLI documentation
//! for usage details.

mod plugin;

// Public API
pub use plugin::BrpToolPlugin;

/// Default port for remote control connections
///
/// This matches Bevy's RemoteHttpPlugin default port to ensure compatibility.
/// Apps using only RemotePlugin/RemoteHttpPlugin will be accessible on this port,
/// while apps with BrpToolPlugin add custom methods (screenshot, shutdown) on the same port.
pub const DEFAULT_REMOTE_PORT: u16 = 15702;

// CLI modules are exposed for testing purposes
#[allow(missing_docs)]
pub mod cli;
