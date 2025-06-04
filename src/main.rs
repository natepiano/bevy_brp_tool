//! CLI tool for controlling Bevy apps remotely

mod cli;

use anyhow::Result;
use bevy_brp_tool::DEFAULT_REMOTE_PORT;
use clap::Parser;
use cli::commands::{Cli, extract_command_from_error, format_command, parse_command_string};
use cli::constants::BIN_NAME;
use cli::{cli_client, commands, detached, error_formatter, help, managed, support};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            // Check if this is a missing arguments error for a subcommand
            let error_msg = error.to_string();
            if error_msg.contains("the following required arguments were not provided") {
                // Try to extract the command name from the error
                if let Some(command_name) = extract_command_from_error(&error_msg) {
                    let args = error_formatter::get_command_args(&command_name);
                    if !args.is_empty() {
                        error_formatter::display_missing_args_error(
                            &format!("{} {}", BIN_NAME, command_name),
                            &args,
                        );
                        std::process::exit(2);
                    }
                }
            }
            // Fall back to default clap error handling
            error.exit();
        }
    };

    // Handle --list-commands flag
    if cli.list_commands {
        help::display_all_commands();
        return Ok(());
    }

    // Handle --help-for flag for specific commands
    if let Some(help_command) = cli.help_for {
        help::display_command_help(&help_command, cli.profile.as_deref());
        return Ok(());
    }

    // Handle --workflows flag
    if cli.workflows {
        help::display_workflow_examples();
        return Ok(());
    }

    // Handle --agent flag
    if cli.agent {
        help::display_agent_instructions();
        return Ok(());
    }

    // Handle --brp flag
    if cli.brp {
        help::display_brp_configuration();
        return Ok(());
    }

    // Handle --info flag
    if cli.info {
        match detached::get_session_info(cli.port).await? {
            Some(info) => {
                println!("{}", support::format_json(&info)?);
            }
            None => {
                eprintln!("No detached session found on port {}", cli.port);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // Handle --cleanup-logs flag
    if cli.cleanup_logs {
        detached::cleanup_all_logs().await?;
        return Ok(());
    }

    // Handle --detect flag
    if cli.detect {
        match help::display_detected_app(cli.profile.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error detecting app: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // Validate mutually exclusive options
    if cli.detached && cli.managed_commands.is_some() {
        eprintln!("Error: Cannot use --detached and --managed-commands together");
        std::process::exit(1);
    }

    // Validate that --detached doesn't have commands
    if cli.detached && (cli.managed_commands.is_some() || cli.command.is_some()) {
        eprintln!("Error: --detached cannot be used with commands. It only starts the app.");
        std::process::exit(1);
    }

    // Validate that --app is only used with --detached or --managed-commands
    if cli.app.is_some() && !cli.detached && cli.managed_commands.is_none() {
        eprintln!("Error: --app/-a can only be used with --detached/-d or --managed-commands/-m");
        eprintln!("  Use: {} -a <APP> -d", BIN_NAME);
        eprintln!("  Or:  {} -a <APP> -m '<commands>'", BIN_NAME);
        std::process::exit(1);
    }

    // Handle command precedence: --managed-commands takes priority over direct command
    let (effective_commands, direct_command) = match (&cli.managed_commands, &cli.command) {
        (Some(commands), Some(cmd)) => {
            // Both provided - warn and use --managed-commands
            eprintln!(
                "Warning: Direct command '{}' used with --managed-commands - direct command '{}' ignored",
                format_command(cmd.clone()),
                format_command(cmd.clone())
            );
            (Some(commands.clone()), None)
        }
        (Some(commands), None) => (Some(commands.clone()), None),
        (None, Some(cmd)) => (None, Some(cmd.clone())),
        (None, None) => (None, None),
    };

    if cli.detached {
        // Detached mode: start app in background with temp log file
        let session = detached::start_detached(cli.app, cli.port, cli.profile).await?;
        println!("\nDetached session started:");
        println!("  PID: {}", session.pid);
        println!("  Port: {}", session.port);
        println!("  Log file: {:?}", session.log_file);
        println!("\nUse '{} --info' to get session details", BIN_NAME);
        println!("Use '{} shutdown' to stop the app", BIN_NAME);
        return Ok(());
    } else if cli.managed_commands.is_some() {
        // Managed commands mode: start app and execute commands directly

        // Commands come from --managed-commands flag
        let commands = cli.managed_commands.clone();

        managed::run_managed(cli.app, commands, cli.port, cli.profile).await?;
    } else {
        // Standalone mode: connect to existing app

        // Handle both --commands and direct commands
        if let Some(commands) = effective_commands {
            // First check if app is running
            let running_instances = cli_client::detect_running_instances(cli.port).await?;

            match running_instances.len() {
                0 => {
                    eprintln!(
                        "Error: No app is running on port {}. Start the app first or use --managed mode.",
                        cli.port
                    );
                    std::process::exit(1);
                }
                1 => {
                    // Execute multiple commands from --commands flag
                    let client = cli::client::RemoteClient::new(cli.port);

                    // Parse and execute each command in the comma-separated list
                    for command_str in commands.split(',') {
                        let command_str = command_str.trim();
                        if command_str.is_empty() {
                            continue;
                        }

                        // Handle special "wait:N" command
                        if command_str.starts_with("wait:") {
                            if let Some(seconds_str) = command_str.strip_prefix("wait:") {
                                if let Ok(seconds) = seconds_str.parse::<u64>() {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(seconds))
                                        .await;
                                    continue;
                                }
                            }
                            eprintln!("Invalid wait command: {}", command_str);
                            continue;
                        }

                        // Parse and execute the command
                        match parse_command_string(command_str) {
                            Ok(parsed_command) => {
                                if let Err(e) =
                                    commands::execute_standalone_command(&client, parsed_command)
                                        .await
                                {
                                    eprintln!("Error executing command '{}': {}", command_str, e);
                                    std::process::exit(1);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse command '{}': {}", command_str, e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                _ => {
                    // Multiple instances detected
                    eprintln!(
                        "Error: Multiple app instances detected on ports: {:?}",
                        running_instances
                    );
                    eprintln!("Please specify which instance to connect to using --port <PORT>");
                    eprintln!("\nAvailable instances:");
                    for port in &running_instances {
                        eprintln!("  - Port {}", port);
                    }
                    std::process::exit(1);
                }
            }
        } else if let Some(command) = direct_command {
            // Execute single direct command

            // Detect running instances
            let running_instances = cli_client::detect_running_instances(cli.port).await?;

            match running_instances.len() {
                0 => {
                    eprintln!(
                        "Error: No app is running on port {}. Start the app first or use --managed mode.",
                        cli.port
                    );
                    std::process::exit(1);
                }
                1 => {
                    // Exactly one instance - proceed normally
                    let client = cli::client::RemoteClient::new(running_instances[0]);
                    commands::execute_standalone_command(&client, command).await?;
                }
                _ => {
                    // Multiple instances detected
                    eprintln!(
                        "Error: Multiple app instances detected on ports: {:?}",
                        running_instances
                    );
                    eprintln!("Please specify which instance to connect to using --port <PORT>");
                    eprintln!("\nAvailable instances:");
                    for port in &running_instances {
                        eprintln!("  - Port {}", port);
                    }
                    std::process::exit(1);
                }
            }
        } else {
            // No commands provided
            eprintln!("Error: No command specified. Use --help for usage information.");
            std::process::exit(1);
        }
    }

    Ok(())
}
