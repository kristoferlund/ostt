//! Application orchestration and command routing.
//!
//! Handles command-line argument parsing and delegates to appropriate command handlers.

use crate::commands;
use crate::logging;
use anyhow::anyhow;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use dirs;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
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

/// Checks if setup is needed (version mismatch or missing config) and runs setup if required.
///
/// This is called early in the startup sequence, before command handling.
/// It checks:
/// 1. If config file doesn't exist, runs full setup
/// 2. If config version is older than app version, runs setup and logs migration
/// 3. If config version matches app version, does nothing
async fn check_and_run_setup() -> Result<(), anyhow::Error> {
    let config_path = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?
        .join(".config")
        .join("ostt")
        .join("ostt.toml");

    match crate::setup::version::check_setup_needed(&config_path)? {
        Some(old_version) => {
            // Setup is needed - either config doesn't exist or version is older
            tracing::info!(
                "Setup needed - migrating from version {} to {}",
                old_version,
                env!("CARGO_PKG_VERSION")
            );
            crate::setup::run_setup().map_err(|e| {
                tracing::error!("Setup failed: {e}");
                anyhow!("Setup failed: {e}")
            })?;
            crate::setup::version::update_config_version(&config_path).map_err(|e| {
                tracing::error!("Failed to update config version: {e}");
                anyhow!("Failed to update config version: {e}")
            })?;
            tracing::info!(
                "Setup completed successfully - migrated to version {}",
                env!("CARGO_PKG_VERSION")
            );
        }
        None => {
            // Config exists and version matches, no setup needed
            tracing::debug!("Config version up to date ({})", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}

/// A terminal-based speech-to-text recorder with real-time waveform visualization
#[derive(Parser)]
#[command(name = "ostt")]
#[command(version)]
#[command(about = "\n\n ┏┓┏╋╋ \n ┗┛┛┗┗")]
#[command(
    long_about = "\n\n ┏┓┏╋╋ \n ┗┛┛┗┗\n\nA terminal-based speech-to-text recorder with real-time waveform visualization\nand automatic transcription support.\n\nDEFAULT COMMAND:\n    If no command is specified, 'record' is used by default.\n    Record options (-c, -o) can be used without explicitly saying 'record'.\n\nEXAMPLES:\n    # Record and pipe to other command (default stdout)\n    $ ostt | grep word\n    $ ostt record | grep word\n    \n    # Record and copy to clipboard\n    $ ostt -c\n    $ ostt record -c\n    \n    # Record and write to file\n    $ ostt -o output.txt\n    $ ostt record -o output.txt\n    \n    # Retry most recent recording and pipe output\n    $ ostt retry | wc -w\n    \n    # Retry recording #2 and copy to clipboard\n    $ ostt retry 2 -c\n    \n    # Transcribe a pre-recorded audio file\n    $ ostt transcribe recording.ogg\n    \n    # Transcribe and copy to clipboard\n    $ ostt transcribe voice-memo.mp3 -c\n    \n    # Set up authentication and select a model\n    $ ostt auth\n    \n    # View your transcription history\n    $ ostt history\n    \n    # Edit configuration file\n    $ ostt config"
)]
#[command(
    after_help = "CONFIGURATION:\n    Config file:        ~/.config/ostt/ostt.toml\n    Logs:               ~/.local/state/ostt/ostt.log.*\n\nFor more information, visit: https://github.com/kristoferlund/ostt"
)]
struct Cli {
    /// Copy transcription to clipboard instead of stdout (record default command)
    #[arg(short, long, global = true)]
    clipboard: bool,

    /// Write transcription to file instead of stdout (record default command)
    #[arg(short, long, value_name = "FILE", global = true)]
    output: Option<String>,

    /// Enable processing after transcription
    #[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")]
    process: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Record audio with real-time visualization (default)
    ///
    /// Press Enter to transcribe, Space to pause/resume, Escape/q to cancel.
    /// By default, transcription outputs to stdout for piping to other commands.
    #[command(visible_alias = "r")]
    Record {
        /// Copy transcription to clipboard instead of stdout
        #[arg(short, long)]
        clipboard: bool,

        /// Write transcription to file instead of stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Enable processing after transcription. Optionally specify action ID to skip picker.
        #[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")]
        process: Option<String>,
    },

    /// Retry transcription of a previous recording
    ///
    /// Re-transcribe a recording using the current model/provider settings.
    /// Useful when transcription failed or you want to try a different model.
    Retry {
        /// Recording index (1 = most recent, 2 = second most recent, etc.)
        #[arg(value_name = "N")]
        index: Option<usize>,

        /// Copy transcription to clipboard instead of stdout
        #[arg(short, long)]
        clipboard: bool,

        /// Write transcription to file instead of stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Enable processing after transcription. Optionally specify action ID to skip picker.
        #[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")]
        process: Option<String>,
    },

    /// Transcribe a pre-recorded audio file
    ///
    /// Transcribe an existing audio file using the configured provider/model.
    /// Supports the same output options as record and retry.
    ///
    /// Examples:
    ///   ostt transcribe recording.ogg
    ///   ostt transcribe voice-memo.mp3 -c
    ///   ostt transcribe meeting.wav -o transcript.txt
    ///   ostt transcribe audio.ogg | grep keyword
    #[command(visible_alias = "t")]
    Transcribe {
        /// Path to the audio file to transcribe
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Copy transcription to clipboard instead of stdout
        #[arg(short, long)]
        clipboard: bool,

        /// Write transcription to file instead of stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Enable processing after transcription. Optionally specify action ID to skip picker.
        #[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")]
        process: Option<String>,
    },

    /// Replay a previous recording using system audio player
    ///
    /// Play back the audio of a previous recording without transcribing.
    /// Uses afplay (macOS) or aplay (Linux).
    #[command(visible_alias = "rp")]
    Replay {
        /// Recording index (1 = most recent, 2 = second most recent, etc.)
        #[arg(value_name = "N")]
        index: Option<usize>,
    },

    /// Authenticate with a transcription provider and select model
    ///
    /// Configure your AI provider credentials and choose which model to use.
    /// Handles both provider selection and API key management in one flow.
    #[command(visible_alias = "a")]
    Auth,

    /// View and browse transcription history
    ///
    /// Browse previous transcriptions, select one to copy to clipboard.
    /// Use arrow keys to navigate, Enter to copy, Esc to exit.
    #[command(visible_alias = "h")]
    History,

    /// Manage keywords for improved transcription accuracy
    ///
    /// Add technical terms, names, or domain-specific vocabulary to help
    /// the AI transcribe more accurately.
    #[command(visible_alias = "k")]
    Keywords,

    /// Open configuration file in your preferred editor
    ///
    /// Edit audio settings, provider options, and other configuration.
    /// Uses $EDITOR environment variable or falls back to nano/vim.
    #[command(visible_alias = "c")]
    Config,

    /// List available audio input devices
    ///
    /// Shows device IDs, names, and configurations to help configure
    /// the correct input device in ostt.toml.
    #[command(name = "list-devices")]
    ListDevices,

    /// Show recent log entries from the application
    ///
    /// Display the last 50 lines of the most recent log file.
    /// Useful for troubleshooting issues.
    Logs,

    /// Post-process a transcription from history
    ///
    /// Run a processing action on an existing transcription.
    /// Shows the action picker if no action is specified.
    ///
    #[command(
        after_help = "EXAMPLES:\n    ostt process                      Process most recent, show picker\n    ostt process clean                Process most recent with the clean action\n    ostt process 5                    Process #5, show picker\n    ostt process 5 clean -c           Process #5 with clean, copy to clipboard\n    ostt process --list               List configured actions"
    )]
    #[command(visible_alias = "p")]
    Process {
        /// History index or action ID
        #[arg(value_name = "INDEX_OR_ACTION")]
        index_or_action: Option<String>,

        /// Action ID when the first argument is a history index
        #[arg(value_name = "ACTION")]
        action: Option<String>,

        /// List all configured actions and exit
        #[arg(long)]
        list: bool,

        /// Copy result to clipboard instead of stdout (shadows global -c)
        #[arg(short, long)]
        clipboard: bool,

        /// Write result to file instead of stdout (shadows global -o)
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,
    },

    /// Launch ostt in a popup terminal window
    ///
    /// Spawns a terminal emulator with ostt running inside it. Pressing the
    /// same hotkey again (re-running `ostt launch`) sends SIGUSR1 to the
    /// running ostt process, which finishes recording and triggers transcription.
    ///
    /// Configure window settings in ~/.config/ostt/ostt.toml under [popup].
    ///
    /// Examples:
    ///   ostt launch -c                  # Record, transcribe, copy to clipboard
    ///   ostt launch -c -p clean         # Record, transcribe, clean, copy
    ///   ostt launch -- -c -p translate  # Record, transcribe, translate, copy
    #[command(visible_alias = "l")]
    Launch {
        /// Arguments to pass to the ostt instance (e.g. "-c", "-p clean")
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Generate shell completion script
    ///
    /// Generate completion script for your shell. Save the output to your
    /// shell's completion directory or source it directly.
    ///
    /// Examples:
    ///   ostt completions bash > ostt.bash
    ///   ostt completions zsh > _ostt
    ///   ostt completions fish > ostt.fish
    ///   ostt completions bash --install
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
        /// Install completions to the standard system directory
        #[arg(long, short)]
        install: bool,
    },
}

fn resolve_process_args(
    index_or_action: Option<String>,
    action: Option<String>,
) -> Result<(Option<usize>, Option<String>), anyhow::Error> {
    match (index_or_action, action) {
        (None, None) => Ok((None, None)),
        (Some(first), None) => match first.parse::<usize>() {
            Ok(index) => Ok((Some(index), None)),
            Err(_) => Ok((None, Some(first))),
        },
        (Some(first), Some(action)) => {
            let index = first.parse::<usize>().map_err(|_| {
                anyhow!(
                    "Invalid process arguments. Use 'ostt process [INDEX] [ACTION]' or 'ostt process [ACTION]'."
                )
            })?;
            Ok((Some(index), Some(action)))
        }
        (None, Some(_)) => unreachable!("clap cannot populate the second positional first"),
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
    let cli = Cli::parse();

    // Handle commands that don't need logging or config setup
    match &cli.command {
        Some(Commands::Completions {
            shell,
            install: true,
        }) => {
            let dir = completion_dir(*shell);
            let filename = completion_filename(*shell);
            fs::create_dir_all(&dir).map_err(|e| anyhow!("Failed to create {dir:?}: {e}"))?;
            let path = dir.join(&filename);
            let file =
                fs::File::create(&path).map_err(|e| anyhow!("Failed to create {path:?}: {e}"))?;
            generate(
                *shell,
                &mut Cli::command(),
                "ostt",
                &mut io::BufWriter::new(file),
            );
            println!("Completions installed to {}", path.display());
            return Ok(());
        }
        Some(Commands::Completions {
            shell,
            install: false,
        }) => {
            generate(*shell, &mut Cli::command(), "ostt", &mut io::stdout());
            return Ok(());
        }
        Some(Commands::ListDevices) => {
            return match commands::handle_list_devices() {
                Ok(()) => Ok(()),
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            };
        }
        Some(Commands::Logs) => {
            return match commands::handle_logs() {
                Ok(()) => Ok(()),
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            };
        }
        _ => {}
    }

    // Initialize logging for all other commands
    logging::init_logging()?;

    // Check if setup is needed (version check or missing config)
    check_and_run_setup().await?;

    // Route to appropriate command handler
    match cli.command {
        None | Some(Commands::Record { .. }) => {
            // Default command is record
            // Merge top-level options with explicit record command options
            // If both are specified, the explicit record command options take precedence
            let (clipboard, output, process) = match cli.command {
                Some(Commands::Record {
                    clipboard,
                    output,
                    process,
                }) => (clipboard, output, process),
                None => (cli.clipboard, cli.output, cli.process),
                _ => unreachable!(),
            };
            commands::handle_record(clipboard, output, process).await?;
        }
        Some(Commands::Retry {
            index,
            clipboard,
            output,
            process,
        }) => {
            commands::handle_retry(index, clipboard, output, process).await?;
        }
        Some(Commands::Transcribe {
            file,
            clipboard,
            output,
            process,
        }) => {
            commands::handle_transcribe(file, clipboard, output, process).await?;
        }
        Some(Commands::Replay { index }) => {
            commands::handle_replay(index).await?;
        }
        Some(Commands::Auth) => {
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
        Some(Commands::History) => {
            commands::handle_history().await?;
        }
        Some(Commands::Keywords) => {
            commands::handle_keywords().await?;
        }
        Some(Commands::Config) => {
            commands::handle_config()?;
        }
        Some(Commands::Process {
            index_or_action,
            action,
            list,
            clipboard,
            output,
        }) => {
            let (index, action) = resolve_process_args(index_or_action, action)?;
            commands::handle_process(index, action, list, clipboard, output).await?;
        }
        Some(Commands::Launch { args }) => {
            // Reconstruct the full ostt args list. Global flags (-c, -o, -p) are
            // consumed by clap before they reach the Launch args vec, so we
            // re-inject them here so they get passed to the spawned ostt instance.
            let mut full_args = args;
            if let Some(process) = cli.process {
                if !process.is_empty() {
                    full_args.insert(0, process);
                }
                full_args.insert(0, "-p".to_string());
            }
            if cli.clipboard {
                full_args.insert(0, "-c".to_string());
            }
            if let Some(ref out) = cli.output {
                full_args.insert(0, out.clone());
                full_args.insert(0, "-o".to_string());
            }
            commands::handle_launch(full_args).await?;
        }
        Some(Commands::Completions { .. }) | Some(Commands::ListDevices) | Some(Commands::Logs) => {
            unreachable!("These commands are handled earlier")
        }
    }

    Ok(())
}

fn completion_dir(shell: Shell) -> PathBuf {
    match shell {
        Shell::Bash => PathBuf::from("/etc/bash_completion.d"),
        Shell::Zsh => {
            if cfg!(target_os = "macos") {
                PathBuf::from("/usr/local/share/zsh/site-functions")
            } else {
                PathBuf::from("/usr/share/zsh/site-functions")
            }
        }
        Shell::Fish => {
            let home = env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            PathBuf::from(home).join(".config/fish/completions")
        }
        Shell::PowerShell => {
            // PowerShell uses a different mechanism; users can still
            // redirect manually: ostt completions powershell > profile.ps1
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
        _ => env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

fn completion_filename(shell: Shell) -> String {
    match shell {
        Shell::Bash => "ostt".to_string(),
        Shell::Zsh => "_ostt".to_string(),
        Shell::Fish => "ostt.fish".to_string(),
        Shell::PowerShell => "ostt.ps1".to_string(),
        _ => "ostt".to_string(),
    }
}
