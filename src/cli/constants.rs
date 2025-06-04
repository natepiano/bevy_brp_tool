//! Constants for the CLI

/// The binary name, determined at compile time from Cargo.toml when building the binary
/// Falls back to "brp" for library builds and tests
pub const BIN_NAME: &str = match option_env!("CARGO_BIN_NAME") {
    Some(name) => name,
    None => "brp",
};

/// Command prefix for Bevy Remote Protocol commands
pub const BEVY_COMMAND_PREFIX: &str = "bevy/";

/// Command prefix for BRP Tool specific commands
pub const BRP_TOOL_COMMAND_PREFIX: &str = "brp_tool/";

// Bevy Remote Protocol commands
pub const BEVY_QUERY: &str = "bevy/query";
pub const BEVY_LIST: &str = "bevy/list";
pub const BEVY_LIST_RESOURCES: &str = "bevy/list_resources";
pub const BEVY_GET: &str = "bevy/get";
pub const BEVY_GET_RESOURCE: &str = "bevy/get_resource";
pub const BEVY_INSERT: &str = "bevy/insert";
pub const BEVY_INSERT_RESOURCE: &str = "bevy/insert_resource";
pub const BEVY_SPAWN: &str = "bevy/spawn";
pub const BEVY_DESTROY: &str = "bevy/destroy";
pub const BEVY_REMOVE: &str = "bevy/remove";
pub const BEVY_REMOVE_RESOURCE: &str = "bevy/remove_resource";
pub const BEVY_MUTATE_COMPONENT: &str = "bevy/mutate_component";
pub const BEVY_MUTATE_RESOURCE: &str = "bevy/mutate_resource";
pub const BEVY_REPARENT: &str = "bevy/reparent";
pub const BEVY_REGISTRY_SCHEMA: &str = "bevy/registry/schema";

// Streaming variants
pub const BEVY_GET_WATCH: &str = "bevy/get+watch";
pub const BEVY_LIST_WATCH: &str = "bevy/list+watch";

// BRP Tool specific commands
pub const BRP_TOOL_SCREENSHOT: &str = "brp_tool/screenshot";
pub const BRP_TOOL_SHUTDOWN: &str = "brp_tool/shutdown";

// Entity ID constants
/// Type used for entity IDs in BRP commands
pub const ENTITY_ID_TYPE: &str = "u64";
/// Example entity ID for documentation and error messages
pub const ENTITY_ID_EXAMPLE: &str = "12345";

// Polling constants
/// Polling interval in milliseconds used for waiting operations
/// Used in support/port_utils.rs for wait_for_port_connectable
pub const POLL_INTERVAL_MS: u64 = 50;

/// Macro to include help text files and replace placeholders
#[macro_export]
macro_rules! include_help {
    ($filename:expr) => {{
        // Try from commands subdirectory first, then from cli directory
        const HELP_TEXT: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/help_text_files/",
            $filename,
            ".txt"
        ));
        let mut text = HELP_TEXT.replace("{{BIN_NAME}}", $crate::cli::constants::BIN_NAME);

        // Replace {{BIN_VERSION}} with the crate version
        if text.contains("{{BIN_VERSION}}") {
            const VERSION: &str = env!("CARGO_PKG_VERSION");
            text = text.replace("{{BIN_VERSION}}", VERSION);
        }

        // Replace {{TEMP_DIR}} with the platform-specific temp directory
        if text.contains("{{TEMP_DIR}}") {
            let temp_dir = std::env::temp_dir().display().to_string();
            text = text.replace("{{TEMP_DIR}}", &temp_dir);
        }

        // Leave {{DETECTED_APP}} as-is for lazy evaluation when help is actually displayed

        text
    }};
}
