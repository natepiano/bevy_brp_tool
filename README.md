# bevy_brp_tool

[![Crates.io](https://img.shields.io/crates/v/bevy_brp_tool.svg)](https://crates.io/crates/bevy_brp_tool)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/natepiano/bevy_brp_tool#license)
[![Docs](https://docs.rs/bevy_brp_tool/badge.svg)](https://docs.rs/bevy_brp_tool)
[![CI](https://github.com/natepiano/bevy_brp_tool/actions/workflows/ci.yml/badge.svg)](https://github.com/natepiano/bevy_brp_tool/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/bevy_brp_tool.svg)](https://crates.io/crates/bevy_brp_tool)

Remote control your Bevy apps using the Bevy Remote Protocol (BRP).
Make it easy for agentic coders such as Claude Code to interact with your Bevy apps while developing and testing.

This crate provides two distinct components:

1. **Library**: A Bevy plugin (`BrpToolPlugin`) for adding remote control capabilities to your Bevy applications - very Optional.
2. **CLI Tool**: A command-line tool (`brp`) for sending commands to Bevy apps - either configured by you with the BRP, or using the `BrpToolPlugin` provided by this crate.

## Library Usage

### What the Plugin Does

The `BrpToolPlugin` is a lightweight plugin that:
1. **Configures Bevy Remote Protocol (BRP)** - Sets up the necessary HTTP/JSON-RPC server
2. **Adds screenshot capability** - Provides a `brp_tool/screenshot` method for capturing screenshots
3. **Adds shutdown capability** - Provides a `brp_tool/shutdown` method for graceful app termination

The plugin code is minimal and adds very little overhead to your application.

### Using the Plugin (Optional)

```rust
use bevy::prelude::*;
use bevy_brp_tool::BrpToolPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BrpToolPlugin::default()) // Port 15702
        // .add_plugins(BrpToolPlugin::with_port(8080)) // Custom port
        .run();
}
```

### Without the Plugin

**You don't need to use `BrpToolPlugin` at all!** The `brp` CLI tool works with any Bevy app that has BRP configured. If you prefer to configure BRP yourself:

```rust
use bevy::prelude::*;
use bevy::remote::{
    RemotePlugin,
    http::RemoteHttpPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .run();
}
```

In this case, you'll have access to all standard BRP methods (spawn, query, get, etc.) but not the custom `brp_tool/screenshot` and `brp_tool/shutdown` methods.

**Port Compatibility Note**: Both approaches use port 15702 by default, so the `brp` CLI tool can connect to apps with either setup.

### Making Components Work with BRP

For components and resources to work with BRP, they need:

```rust
#[derive(Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct MyComponent { /* ... */ }

// Register in your app:
app.register_type::<MyComponent>();
```

A lot of boilerplate but there it is - maybe it will change one day. But once you've done this, MyComponent is ready to be used with BRP.

### Library API

The library exposes only two public items:
- `BrpToolPlugin` - The plugin configuring BRP on your behalf and adding a couple of useful methods
- `DEFAULT_REMOTE_PORT` - The default port constant (15702, matches RemoteHttpPlugin) - convenience

## CLI Usage (For Testing and Debugging)

The `brp` CLI tool allows you to control Bevy apps that have the plugin installed:

```bash
# List all component types
brp list

# Query entities with Transform components
brp query bevy_transform::components::transform::Transform

# Get component data from entity
brp get 12345 bevy_transform::components::transform::Transform

# Spawn entity with components
brp spawn '{"bevy_core::name::Name": "MyEntity"}'

# Take a screenshot
brp screenshot ./debug.png

# See all commands
brp --list-commands
```

### Managed Mode

Start your app and run commands automatically:

```bash
# Start app, run commands, then shutdown
brp --managed-commands 'list,screenshot ./test.png,shutdown'
```

### CLI Help

- `brp --help` - General usage information
- `brp --list-commands` - List all available commands
- `brp --help-for <COMMAND>` - Detailed help for a specific command
- `brp --brp` - Help for configuring Bevy Remote Protocol
- `brp --agent` - Interactive workflow guide

## Installation

### For CLI Tool Users

```bash
# Install the CLI tool globally
cargo install bevy_brp_tool
```

### For Bevy App Developers

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_brp_tool = "0.1"
```

Or use cargo-add:

```bash
cargo add bevy_brp_tool
```

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
