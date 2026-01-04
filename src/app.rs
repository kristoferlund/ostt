//! Application orchestration and command routing.
//!
//! Handles command-line argument parsing and delegates to appropriate command handlers.

use crate::commands;
use crate::logging;
use anyhow::anyhow;
use dirs;
use std::env;
use std::process;

/// Suppress ALSA library warnings that are not relevant to the user.
/// These warnings come from the cpal audio library and don't indicate actual errors.
#[allow(dead_code)]
fn suppress_alsa_warnings() {
    // Set ALSA_CARD to a dummy value to suppress "Unknown PCM" warnings
    if env::var("ALSA_CARD").is_err() {
        env::set_var("ALSA_CARD", "dummy");
    }
}

/// Application command types.
#[derive(Debug)]
enum Command {
    /// Record audio and optionally transcribe
    Record,
    /// Authenticate with a transcription provider and select model
    Auth,
    /// View transcription history
    History,
    /// Manage keywords for transcription
    Keywords,
    /// Edit configuration file
    Config,
    /// Show help message
    Help,
    /// Show version information
    Version,
    /// List available audio input devices
    ListDevices,
    /// Show recent log entries
    Logs,
    /// Invalid command provided
    Invalid(String),
}

const HELP_TEXT: &str = r#"
┏┓┏╋╋
┗┛┛┗┗

A terminal-based speech-to-text recorder with real-time waveform visualization
and automatic transcription support.

USAGE:
    ostt [COMMAND]

COMMANDS:
    record              Record audio with real-time volume metering
                        Press Enter to transcribe, Escape/q to cancel

    auth                Authenticate with a transcription provider and
                        select a model. Handles both provider selection
                        and API key management in one unified flow.

    history             View and browse your transcription history
                        Select a transcription to copy it to clipboard

    keywords            Manage keywords for improved transcription accuracy
                        Add, remove, and view keywords used by AI models

    config              Open configuration file in your preferred editor
                        Customize audio settings and provider options

    version, -V, --version
                        Show version information

    list-devices        List available audio input devices

    logs                Show recent log entries from the application

    help, -h, --help    Show this help message

EXAMPLES:
    # Record audio
    $ ostt record
    
    # Set up authentication and select a model
    $ ostt auth
    
    # View your transcription history
    $ ostt history
    
    # Edit configuration file
    $ ostt config

CONFIGURATION:
    Config file:        ~/.config/ostt/ostt.toml
    Logs:               ~/.local/state/ostt/ostt.log.*

For more information, visit: https://github.com/kristoferlund/ostt
"#;

impl Command {
    /// Parse command from command-line arguments.
    ///
    /// Returns the appropriate command based on the first argument.
    /// If no arguments or "record" is provided, returns Record command.
    /// If an unrecognized command is provided, returns Invalid.
    fn from_args() -> Self {
        let args: Vec<String> = env::args().collect();

        if args.len() > 1 {
            match args[1].as_str() {
                "record" => Command::Record,
                "auth" => Command::Auth,
                "history" => Command::History,
                "keywords" => Command::Keywords,
                "config" => Command::Config,
                "help" | "-h" | "--help" => Command::Help,
                "version" | "-V" | "--version" => Command::Version,
                "list-devices" => Command::ListDevices,
                "logs" => Command::Logs,
                invalid => Command::Invalid(invalid.to_string()),
            }
        } else {
            Command::Record
        }
    }
}

/// Runs the main application based on command-line arguments.
///
/// # Exit Codes
/// - 0: Success
/// - 1: General error
/// - 2: Usage error (invalid arguments)
///
/// # Errors
/// - If setup fails
/// - If logging initialization fails
/// - If command execution fails (e.g., authentication, recording, history viewing)
pub async fn run() -> Result<(), anyhow::Error> {
    let command = Command::from_args();

    if matches!(command, Command::Help) {
        println!("{HELP_TEXT}");
        return Ok(());
    }

    if matches!(command, Command::Version) {
        println!("ostt {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if matches!(command, Command::ListDevices) {
        return match commands::handle_list_devices() {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        };
    }

    if matches!(command, Command::Logs) {
        return match commands::handle_logs() {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        };
    }

    if let Command::Invalid(cmd) = &command {
        eprintln!("Error: unknown command '{cmd}'");
        eprintln!("Run 'ostt help' to see available commands.");
        process::exit(2);
    }

    logging::init_logging()?;

    let config_path = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?
        .join(".config")
        .join("ostt")
        .join("ostt.toml");
    if !config_path.exists() {
        tracing::info!("Configuration file not found, running setup...");
        crate::setup::run_setup().map_err(|e| {
            tracing::error!("Setup failed: {e}");
            anyhow!("Setup failed: {e}")
        })?;
        tracing::info!("Setup completed successfully");
    }

    match command {
        Command::Auth => {
            if let Err(e) = commands::handle_auth().await {
                // Check if it's a cancellation error (cliclack already displayed the message)
                let err_msg = e.to_string();
                if err_msg.contains("cancelled") || err_msg.contains("interrupted") {
                    // Silent exit - cliclack already showed "Operation cancelled"
                    process::exit(0);
                } else {
                    return Err(e);
                }
            }
        }
        Command::Record => commands::handle_record().await?,
        Command::History => commands::handle_history().await?,
        Command::Keywords => commands::handle_keywords().await?,
        Command::Config => commands::handle_config()?,
        Command::Help => unreachable!(),
        Command::Version => unreachable!(),
        Command::ListDevices => unreachable!(),
        Command::Logs => unreachable!(),
        Command::Invalid(_) => unreachable!(),
    }

    Ok(())
}
