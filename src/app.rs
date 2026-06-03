//! Application orchestration and command routing.
//!
//! Handles command-line argument parsing and delegates to appropriate command handlers.

use crate::commands;
use crate::logging;
use anyhow::anyhow;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
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

/// Checks if setup is needed and applies only the required setup step.
///
/// This is called early in the startup sequence, before command handling.
/// It checks:
/// 1. If config file doesn't exist, creates the default config
/// 2. If config version is older than app version, preserves user config and updates version
/// 3. If config version matches app version, does nothing
async fn check_and_run_setup() -> Result<(), anyhow::Error> {
    let config_path = crate::app_dirs::config_path()?;

    match crate::setup::version::check_setup_needed(&config_path)? {
        Some(old_version) => {
            tracing::info!(
                "Setup needed - updating from version {} to {}",
                old_version,
                env!("CARGO_PKG_VERSION")
            );

            if !config_path.exists() {
                crate::setup::run_setup().map_err(|e| {
                    tracing::error!("Setup failed: {e}");
                    anyhow!("Setup failed: {e}")
                })?;
            }

            crate::setup::version::update_config_version(&config_path).map_err(|e| {
                tracing::error!("Failed to update config version: {e}");
                anyhow!("Failed to update config version: {e}")
            })?;
            tracing::info!(
                "Setup completed successfully - config version is {}",
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

fn load_config() -> anyhow::Result<crate::config::OsttConfig> {
    crate::config::OsttConfig::load().map_err(|err| {
        tracing::error!("Failed to load configuration: {err}");
        anyhow!("Configuration error: {err}\nPlease check your ~/.config/ostt/ostt.toml file and try again.")
    })
}

/// A terminal-based speech-to-text recorder with real-time waveform visualization
#[derive(Parser)]
#[command(name = "ostt")]
#[command(version)]
#[command(about = "\n\n┏┓┏╋╋ \n┗┛┛┗┗")]
#[command(
    long_about = "\n\n┏┓┏╋╋ \n┗┛┛┗┗\n\nA terminal-based speech-to-text recorder with real-time waveform visualization\nand automatic transcription support.\n\nDEFAULT COMMAND:\n    If no command is specified, 'record' is used by default.\n    Record options (-c, -o) can be used without explicitly saying 'record'.\n\nEXAMPLES:\n    # Record and pipe to other command (default stdout)\n    $ ostt | grep word\n    $ ostt record | grep word\n    \n    # Record and copy to clipboard\n    $ ostt -c\n    $ ostt record -c\n    $ ostt -m deepgram/nova-3 -c\n    \n    # Record and write to file\n    $ ostt -o output.txt\n    $ ostt record -o output.txt\n    \n    # Retry most recent recording and pipe output\n    $ ostt retry | wc -w\n    \n    # Retry recording #2 and copy to clipboard\n    $ ostt retry 2 -c\n    \n    # Transcribe a pre-recorded audio file\n    $ ostt transcribe recording.ogg\n    $ ostt transcribe recording.ogg -m openai/gpt-4o-transcribe\n    \n    # Transcribe and copy to clipboard\n    $ ostt transcribe voice-memo.mp3 -c\n    \n    # Set up authentication for cloud providers\n    $ ostt auth\n    \n    # Choose cloud or local transcription model\n    $ ostt model\n    \n    # View your transcription history\n    $ ostt history\n    \n    # Manage transcription keywords\n    $ ostt keyword\n\n    # Manage deterministic text replace rules\n    $ ostt replace\n    \n    # Edit configuration file\n    $ ostt config"
)]
#[command(
    after_help = "CONFIGURATION:\n    Config file:        ~/.config/ostt/ostt.toml\n    Logs:               ~/.local/state/ostt/ostt.log.*\n\nFor more information, visit: https://github.com/kristoferlund/ostt"
)]
struct Cli {
    /// Copy transcription to clipboard instead of stdout (record default command)
    #[arg(short, long)]
    clipboard: bool,

    /// Paste transcription into the focused app (record default command)
    #[arg(long)]
    paste: bool,

    /// Write transcription to file instead of stdout (record default command)
    #[arg(short, long, value_name = "FILE")]
    output: Option<String>,

    /// Enable processing after transcription
    #[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")]
    process: Option<String>,

    /// Override transcription model for this run
    #[arg(short = 'm', long = "model", value_name = "PROVIDER/MODEL")]
    model: Option<String>,

    /// Override a transcription param for this run, as key=value
    #[arg(long = "param", value_name = "KEY=VALUE", global = true)]
    params: Vec<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum ListFormat {
    Table,
    Json,
}

impl ListFormat {
    fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
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

        /// Paste transcription into the focused app
        #[arg(long)]
        paste: bool,

        /// Write transcription to file instead of stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Enable processing after transcription. Optionally specify action ID to skip picker.
        #[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")]
        process: Option<String>,

        /// Override transcription model for this run
        #[arg(short = 'm', long = "model", value_name = "PROVIDER/MODEL")]
        model: Option<String>,
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

        /// Paste transcription into the focused app
        #[arg(long)]
        paste: bool,

        /// Write transcription to file instead of stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Enable processing after transcription. Optionally specify action ID to skip picker.
        #[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")]
        process: Option<String>,

        /// Override transcription model for this run
        #[arg(short = 'm', long = "model", value_name = "PROVIDER/MODEL")]
        model: Option<String>,
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

        /// Paste transcription into the focused app
        #[arg(long)]
        paste: bool,

        /// Write transcription to file instead of stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Enable processing after transcription. Optionally specify action ID to skip picker.
        #[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")]
        process: Option<String>,

        /// Override transcription model for this run
        #[arg(short = 'm', long = "model", value_name = "PROVIDER/MODEL")]
        model: Option<String>,
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

    /// Manage cloud provider credentials
    ///
    /// Configure or remove AI provider credentials.
    #[command(visible_alias = "a")]
    Auth {
        #[command(subcommand)]
        command: Option<AuthCommand>,
    },

    /// Choose and manage cloud or local transcription models
    ///
    /// Opens an interactive model picker. Choose a cloud model from authenticated
    /// providers, download and activate local models, or add a custom local model.
    #[command(name = "model")]
    Model {
        #[command(subcommand)]
        command: Option<ModelCommand>,
    },

    /// View and browse transcription history
    ///
    /// Browse previous transcriptions, select one to copy to clipboard.
    /// Use arrow keys to navigate, Enter to copy, Esc to exit.
    #[command(visible_alias = "h")]
    History {
        #[command(subcommand)]
        command: Option<HistoryCommand>,
    },

    /// Manage keywords for improved transcription accuracy
    ///
    /// Add technical terms, names, or domain-specific vocabulary to help
    /// the AI transcribe more accurately.
    #[command(visible_alias = "k")]
    #[command(name = "keyword")]
    Keyword {
        #[command(subcommand)]
        command: Option<KeywordCommand>,
    },

    /// Manage deterministic text replace rules
    ///
    /// Configure final-text replace rules for casing, acronyms, product names,
    /// and common transcription corrections.
    Replace,

    #[command(name = "__paste", hide = true)]
    PasteHelper,

    /// Open configuration file in your preferred editor
    ///
    /// Edit audio settings, provider options, and other configuration.
    /// Uses $EDITOR environment variable or falls back to nano/vim.
    #[command(visible_alias = "c")]
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },

    /// Show recent log entries from the application
    ///
    /// Display the last 50 lines of the most recent log file.
    /// Useful for troubleshooting issues.
    Logs {
        #[command(subcommand)]
        command: Option<LogsCommand>,
    },

    /// Post-process a transcription from history
    ///
    /// Run a processing action on an existing transcription.
    /// Shows the action picker if no action is specified.
    ///
    #[command(
        after_help = "EXAMPLES:\n    ostt process                      Process most recent, show picker\n    ostt process clean                Process most recent with the clean action\n    ostt process 5                    Process #5, show picker\n    ostt process 5 clean -c           Process #5 with clean, copy to clipboard\n    ostt process list                 List configured actions"
    )]
    #[command(visible_alias = "p")]
    Process {
        /// History index or action ID
        #[arg(value_name = "INDEX_OR_ACTION")]
        index_or_action: Option<String>,

        /// Action ID when the first argument is a history index
        #[arg(value_name = "ACTION")]
        action: Option<String>,

        /// Copy result to clipboard instead of stdout (shadows global -c)
        #[arg(short, long)]
        clipboard: bool,

        /// Paste result into the focused app
        #[arg(long)]
        paste: bool,

        /// Write result to file instead of stdout (shadows global -o)
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Output format for `ostt process list`
        #[arg(long, value_enum, default_value_t = ListFormat::Table)]
        format: ListFormat,
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
    ///   ostt completions install bash
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Option<Shell>,
        #[command(subcommand)]
        command: Option<CompletionsCommand>,
    },

    /// Manage the local model daemon
    ///
    /// The daemon keeps a local Whisper model loaded in memory so transcriptions
    /// start instantly instead of reloading the model on every call. It always
    /// serves the currently active model (configured with `ostt model`).
    ///
    /// Examples:
    ///   ostt daemon start            # start the daemon for the active model
    ///   ostt daemon stop             # stop the running daemon
    ///   ostt daemon restart          # restart with the current active model
    ///   ostt daemon status           # show running status and service info
    ///   ostt daemon install          # install as a login service (auto-start)
    ///   ostt daemon uninstall        # remove the login service
    #[command(visible_alias = "d")]
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
}

#[derive(Subcommand)]
enum ModelCommand {
    /// List available transcription models
    List {
        /// Filter by provider ID
        #[arg(long)]
        provider: Option<String>,
        /// Show only downloaded local models
        #[arg(long)]
        installed: bool,
        /// Output format
        #[arg(long, value_enum, default_value_t = ListFormat::Table)]
        format: ListFormat,
    },
    /// Show the currently selected transcription model
    Current,
    /// List supported transcription params for a transcription model
    Params {
        /// Model to inspect. Defaults to the currently selected model.
        #[arg(value_name = "PROVIDER/MODEL")]
        model: Option<String>,
        /// Output format
        #[arg(long, value_enum, default_value_t = ListFormat::Table)]
        format: ListFormat,
    },
    /// Select the active transcription model
    Select {
        #[arg(value_name = "PROVIDER/MODEL")]
        model: String,
    },
    /// Manage local transcription models
    Local {
        #[command(subcommand)]
        command: LocalModelCommand,
    },
}

#[derive(Subcommand)]
enum LocalModelCommand {
    /// Download a local model
    Download {
        #[arg(value_name = "MODEL_ID|PROVIDER/MODEL")]
        model_id: String,
    },
    /// Remove a downloaded local model
    Remove {
        #[arg(value_name = "MODEL_ID|PROVIDER/MODEL")]
        model_id: String,
    },
}

#[derive(Subcommand)]
enum KeywordCommand {
    /// List transcription keywords
    List {
        /// Output format
        #[arg(long, value_enum, default_value_t = ListFormat::Table)]
        format: ListFormat,
    },
    /// Add transcription keywords
    Add {
        #[arg(value_name = "KEYWORD", required = true)]
        keywords: Vec<String>,
    },
    /// Remove transcription keywords
    Remove {
        #[arg(value_name = "KEYWORD", required = true)]
        keywords: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Print the config file path
    Path,
    /// List audio input devices for config selection
    ListDevices,
}

#[derive(Subcommand)]
enum LogsCommand {
    /// Follow the latest log file
    Follow,
    /// Print the latest log file path
    Path,
}

#[derive(Subcommand)]
enum HistoryCommand {
    /// List transcription history
    List {
        /// Limit to N most recent entries
        #[arg(long)]
        limit: Option<usize>,
        /// Output format
        #[arg(long, value_enum, default_value_t = ListFormat::Table)]
        format: ListFormat,
    },
    /// Show a transcription from history
    Show {
        /// History index (1 = most recent)
        #[arg(value_name = "N")]
        index: Option<usize>,
    },
    /// Copy a transcription from history to clipboard
    Copy {
        /// History index (1 = most recent)
        #[arg(value_name = "N")]
        index: Option<usize>,
    },
}

#[derive(Subcommand)]
enum CompletionsCommand {
    /// Install completions to the standard system directory
    Install {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(clap::Subcommand)]
enum DaemonCommand {
    /// Start the daemon for the active local model
    Start,
    /// Stop the running daemon
    Stop,
    /// Restart the daemon with the currently active local model
    Restart,
    /// Show daemon status (running, model, PID, service)
    Status,
    /// Install daemon as a login service (auto-start on login)
    Install,
    /// Remove the daemon login service
    Uninstall,
    /// [Internal] Run the daemon process — used by the service manager and daemon start
    #[command(hide = true)]
    Run {
        /// Model ID to load (defaults to the currently active local model)
        #[arg(long)]
        model_id: Option<String>,
        /// Exit after this many seconds of inactivity (omit for no timeout)
        #[arg(long)]
        idle_timeout_secs: Option<u64>,
    },
}

#[derive(Subcommand)]
enum AuthCommand {
    /// Add or update a cloud provider credential
    Login {
        /// Provider ID to log in to
        provider: Option<String>,
    },
    /// Remove a cloud provider credential
    Logout {
        /// Provider ID to log out from
        provider: Option<String>,
    },
    /// List authenticated providers
    List {
        /// Output format
        #[arg(long, value_enum, default_value_t = ListFormat::Table)]
        format: ListFormat,
    },
    /// Show authentication status
    Status,
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
    // The `daemon run` subcommand is the long-running daemon process itself.
    // It must use daemon-specific logging and skip the normal setup flow.
    if let Some(Commands::Daemon {
        command:
            DaemonCommand::Run {
                ref model_id,
                idle_timeout_secs,
            },
    }) = cli.command
    {
        logging::init_logging()?;
        return commands::daemon::handle_daemon_run(model_id.clone(), idle_timeout_secs).await;
    }

    match &cli.command {
        Some(Commands::Completions {
            command: Some(CompletionsCommand::Install { shell }),
            ..
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
            shell: Some(shell),
            command: None,
        }) => {
            generate(*shell, &mut Cli::command(), "ostt", &mut io::stdout());
            return Ok(());
        }
        Some(Commands::Completions {
            shell: None,
            command: None,
        }) => {
            return Err(anyhow!(
                "Use 'ostt completions <SHELL>' or 'ostt completions install <SHELL>'."
            ));
        }
        Some(Commands::Logs { command: None }) => {
            return match commands::handle_logs() {
                Ok(()) => Ok(()),
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            };
        }
        Some(Commands::Logs {
            command: Some(LogsCommand::Path),
        }) => return commands::logs::handle_logs_path(),
        Some(Commands::Logs {
            command: Some(LogsCommand::Follow),
        }) => return commands::logs::handle_logs_follow(),
        Some(Commands::Config { command: None }) => {
            check_and_run_setup().await?;
            return commands::handle_config();
        }
        Some(Commands::Config {
            command: Some(ConfigCommand::Path),
        }) => return commands::config::handle_config_path(),
        _ => {}
    }

    // Initialize logging for all other commands
    logging::init_logging()?;

    // Check if setup is needed (version check or missing config)
    check_and_run_setup().await?;

    let config_data = load_config()?;

    // Route to appropriate command handler
    match cli.command {
        None | Some(Commands::Record { .. }) => {
            // Default command is record
            // Merge top-level options with explicit record command options
            // If both are specified, the explicit record command options take precedence
            let (clipboard, paste, output, process, model) = match cli.command {
                Some(Commands::Record {
                    clipboard,
                    paste,
                    output,
                    process,
                    model,
                }) => (clipboard, paste, output, process, model.or(cli.model)),
                None => (cli.clipboard, cli.paste, cli.output, cli.process, cli.model),
                _ => unreachable!(),
            };
            ensure_single_output_mode(clipboard, paste, output.as_ref())?;
            let model_override = model
                .as_deref()
                .map(crate::config::parse_provider_model)
                .transpose()?;
            commands::handle_record(
                &config_data,
                clipboard,
                paste,
                output,
                process,
                model_override,
                &cli.params,
            )
            .await?;
        }
        Some(Commands::Retry {
            index,
            clipboard,
            paste,
            output,
            process,
            model,
        }) => {
            ensure_single_output_mode(clipboard, paste, output.as_ref())?;
            let model_override = model
                .or(cli.model)
                .as_deref()
                .map(crate::config::parse_provider_model)
                .transpose()?;
            commands::handle_retry(
                &config_data,
                index,
                clipboard,
                paste,
                output,
                process,
                model_override,
                &cli.params,
            )
            .await?;
        }
        Some(Commands::Transcribe {
            file,
            clipboard,
            paste,
            output,
            process,
            model,
        }) => {
            ensure_single_output_mode(clipboard, paste, output.as_ref())?;
            let model_override = model
                .or(cli.model)
                .as_deref()
                .map(crate::config::parse_provider_model)
                .transpose()?;
            commands::handle_transcribe(
                &config_data,
                file,
                clipboard,
                paste,
                output,
                process,
                model_override,
                &cli.params,
            )
            .await?;
        }
        Some(Commands::Replay { index }) => {
            commands::handle_replay(index).await?;
        }
        Some(Commands::Auth { command }) => {
            let result = match command.unwrap_or(AuthCommand::Login { provider: None }) {
                AuthCommand::Login { provider } => {
                    commands::auth::handle_auth_login(provider).await
                }
                AuthCommand::Logout { provider } => {
                    commands::auth::handle_logout(&config_data, provider).await
                }
                AuthCommand::List { format } => commands::auth::handle_auth_list(format.is_json()),
                AuthCommand::Status => commands::auth::handle_auth_status(),
            };

            if let Err(e) = result {
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
        Some(Commands::Model { command }) => match command {
            None => commands::handle_model().await?,
            Some(ModelCommand::List {
                provider,
                installed,
                format,
            }) => commands::model::handle_model_list(provider, installed, format.is_json()).await?,
            Some(ModelCommand::Current) => commands::model::handle_model_current()?,
            Some(ModelCommand::Params { model, format }) => {
                commands::model::handle_model_params(model, format.is_json())?
            }
            Some(ModelCommand::Select { model }) => {
                commands::model::handle_model_select(model).await?
            }
            Some(ModelCommand::Local { command }) => match command {
                LocalModelCommand::Download { model_id } => {
                    commands::model::handle_model_local_download(model_id).await?
                }
                LocalModelCommand::Remove { model_id } => {
                    commands::model::handle_model_local_remove(model_id).await?
                }
            },
        },
        Some(Commands::History { command }) => match command {
            None => commands::handle_history().await?,
            Some(HistoryCommand::List { limit, format }) => {
                commands::history::handle_history_list(limit, format.is_json())?
            }
            Some(HistoryCommand::Show { index }) => commands::history::handle_history_show(index)?,
            Some(HistoryCommand::Copy { index }) => commands::history::handle_history_copy(index)?,
        },
        Some(Commands::Keyword { command }) => match command {
            None => commands::handle_keywords().await?,
            Some(KeywordCommand::List { format }) => {
                commands::keywords::handle_keyword_list(format.is_json())?
            }
            Some(KeywordCommand::Add { keywords }) => {
                commands::keywords::handle_keyword_add(keywords)?
            }
            Some(KeywordCommand::Remove { keywords }) => {
                commands::keywords::handle_keyword_remove(keywords)?
            }
        },
        Some(Commands::Replace) => commands::handle_replace().await?,
        Some(Commands::PasteHelper) => crate::paste::handle_paste_helper(&config_data)?,
        Some(Commands::Config { command }) => match command {
            None => commands::handle_config()?,
            Some(ConfigCommand::Path) => commands::config::handle_config_path()?,
            Some(ConfigCommand::ListDevices) => commands::handle_list_devices()?,
        },
        Some(Commands::Process {
            index_or_action,
            action,
            format,
            ..
        }) if index_or_action.as_deref() == Some("list") && action.is_none() => {
            commands::process::handle_process_list(&config_data, format.is_json())?;
        }
        Some(Commands::Process {
            index_or_action,
            action,
            clipboard,
            paste,
            output,
            ..
        }) => {
            ensure_single_output_mode(clipboard, paste, output.as_ref())?;
            let (index, action) = resolve_process_args(index_or_action, action)?;
            commands::handle_process(&config_data, index, action, false, clipboard, paste, output)
                .await?;
        }
        Some(Commands::Launch { args }) => {
            ensure_single_output_mode(cli.clipboard, cli.paste, cli.output.as_ref())?;
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
            if cli.paste {
                full_args.insert(0, "--paste".to_string());
            }
            if let Some(ref out) = cli.output {
                full_args.insert(0, out.clone());
                full_args.insert(0, "-o".to_string());
            }
            if let Some(ref model) = cli.model {
                full_args.insert(0, model.clone());
                full_args.insert(0, "-m".to_string());
            }
            for param in cli.params.iter().rev() {
                full_args.insert(0, param.clone());
                full_args.insert(0, "--param".to_string());
            }
            commands::handle_launch(&config_data, full_args).await?;
        }
        Some(Commands::Daemon { command }) => match command {
            DaemonCommand::Start => commands::daemon::handle_daemon_start(&config_data).await?,
            DaemonCommand::Stop => commands::daemon::handle_daemon_stop().await?,
            DaemonCommand::Restart => commands::daemon::handle_daemon_restart(&config_data).await?,
            DaemonCommand::Status => commands::daemon::handle_daemon_status(&config_data).await?,
            DaemonCommand::Install => commands::daemon::handle_daemon_install()?,
            DaemonCommand::Uninstall => commands::daemon::handle_daemon_uninstall()?,
            DaemonCommand::Run { .. } => {
                unreachable!("daemon run is handled before logging init")
            }
        },
        Some(Commands::Completions { .. }) | Some(Commands::Logs { .. }) => {
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

fn ensure_single_output_mode(
    clipboard: bool,
    paste: bool,
    output: Option<&String>,
) -> anyhow::Result<()> {
    let selected = usize::from(clipboard) + usize::from(paste) + usize::from(output.is_some());
    if selected > 1 {
        anyhow::bail!("Choose one output mode: stdout, --clipboard, --output, or --paste.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_accepts_model_override_for_default_record() {
        let cli = Cli::try_parse_from(["ostt", "-m", "deepgram/nova-3"]).expect("parse cli");
        assert_eq!(cli.model.as_deref(), Some("deepgram/nova-3"));
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_accepts_model_override_for_transcribe() {
        let cli = Cli::try_parse_from([
            "ostt",
            "transcribe",
            "audio.ogg",
            "-m",
            "berget/openai/whisper-large-v3",
        ])
        .expect("parse cli");

        match cli.command {
            Some(Commands::Transcribe { model, .. }) => {
                assert_eq!(model.as_deref(), Some("berget/openai/whisper-large-v3"));
            }
            _ => panic!("expected transcribe command"),
        }
    }

    #[test]
    fn cli_accepts_model_override_for_retry() {
        let cli = Cli::try_parse_from(["ostt", "retry", "-m", "groq/whisper-large-v3"])
            .expect("parse cli");

        match cli.command {
            Some(Commands::Retry { model, .. }) => {
                assert_eq!(model.as_deref(), Some("groq/whisper-large-v3"));
            }
            _ => panic!("expected retry command"),
        }
    }

    #[test]
    fn cli_accepts_model_override_for_launch() {
        let cli = Cli::try_parse_from(["ostt", "launch", "-m", "deepgram/nova-3", "-c"])
            .expect("parse cli");
        match cli.command {
            Some(Commands::Launch { args }) => {
                assert_eq!(args, ["-m", "deepgram/nova-3", "-c"]);
            }
            _ => panic!("expected launch command"),
        }
    }

    #[test]
    fn cli_accepts_model_params_listing_format() {
        let cli = Cli::try_parse_from([
            "ostt",
            "model",
            "params",
            "openai/gpt-4o-transcribe",
            "--format",
            "json",
        ])
        .expect("parse cli");

        match cli.command {
            Some(Commands::Model {
                command:
                    Some(ModelCommand::Params {
                        model: Some(model),
                        format,
                    }),
            }) => {
                assert_eq!(model, "openai/gpt-4o-transcribe");
                assert_eq!(format, ListFormat::Json);
            }
            _ => panic!("expected model params command"),
        }
    }

    #[test]
    fn cli_uses_singular_keyword_command() {
        assert!(Cli::try_parse_from(["ostt", "keywords"]).is_err());

        let cli = Cli::try_parse_from(["ostt", "keyword", "list", "--format", "json"])
            .expect("parse cli");
        match cli.command {
            Some(Commands::Keyword {
                command: Some(KeywordCommand::List { format }),
            }) => assert_eq!(format, ListFormat::Json),
            _ => panic!("expected keyword list command"),
        }
    }

    #[test]
    fn cli_uses_replace_command() {
        let cli = Cli::try_parse_from(["ostt", "replace"]).expect("parse cli");
        match cli.command {
            Some(Commands::Replace) => {}
            _ => panic!("expected replace command"),
        }
    }

    #[test]
    fn cli_accepts_paste_for_output_commands() {
        let cli = Cli::try_parse_from(["ostt", "--paste"]).expect("parse cli");
        assert!(cli.paste);

        let cli = Cli::try_parse_from(["ostt", "record", "--paste"]).expect("parse cli");
        match cli.command {
            Some(Commands::Record { paste, .. }) => assert!(paste),
            _ => panic!("expected record command"),
        }

        let cli =
            Cli::try_parse_from(["ostt", "transcribe", "audio.ogg", "--paste"]).expect("parse cli");
        match cli.command {
            Some(Commands::Transcribe { paste, .. }) => assert!(paste),
            _ => panic!("expected transcribe command"),
        }

        let cli = Cli::try_parse_from(["ostt", "retry", "--paste"]).expect("parse cli");
        match cli.command {
            Some(Commands::Retry { paste, .. }) => assert!(paste),
            _ => panic!("expected retry command"),
        }

        let cli = Cli::try_parse_from(["ostt", "process", "clean", "--paste"]).expect("parse cli");
        match cli.command {
            Some(Commands::Process { paste, .. }) => assert!(paste),
            _ => panic!("expected process command"),
        }
    }

    #[test]
    fn cli_forwards_paste_for_launch() {
        let cli =
            Cli::try_parse_from(["ostt", "launch", "--paste", "-p", "clean"]).expect("parse cli");

        match cli.command {
            Some(Commands::Launch { args }) => {
                assert_eq!(args, ["--paste", "-p", "clean"]);
            }
            _ => panic!("expected launch command"),
        }
    }

    #[test]
    fn output_mode_validation_rejects_ambiguous_modes() {
        assert!(ensure_single_output_mode(true, true, None).is_err());
        assert!(ensure_single_output_mode(false, true, Some(&"out.txt".to_string())).is_err());
        assert!(ensure_single_output_mode(true, false, Some(&"out.txt".to_string())).is_err());
        assert!(ensure_single_output_mode(false, true, None).is_ok());
    }

    #[test]
    fn cli_moves_device_listing_under_config() {
        assert!(Cli::try_parse_from(["ostt", "list-devices"]).is_err());

        let cli = Cli::try_parse_from(["ostt", "config", "list-devices"]).expect("parse cli");
        match cli.command {
            Some(Commands::Config {
                command: Some(ConfigCommand::ListDevices),
            }) => {}
            _ => panic!("expected config list-devices command"),
        }
    }

    #[test]
    fn cli_uses_subcommands_for_process_and_completion_listing_actions() {
        assert!(Cli::try_parse_from(["ostt", "process", "--list"]).is_err());
        assert!(Cli::try_parse_from(["ostt", "completions", "bash", "--install"]).is_err());

        let cli = Cli::try_parse_from(["ostt", "process", "list", "--format", "json"])
            .expect("parse cli");
        match cli.command {
            Some(Commands::Process {
                index_or_action,
                format,
                ..
            }) => {
                assert_eq!(index_or_action.as_deref(), Some("list"));
                assert_eq!(format, ListFormat::Json);
            }
            _ => panic!("expected process list command"),
        }

        let cli =
            Cli::try_parse_from(["ostt", "completions", "install", "bash"]).expect("parse cli");
        match cli.command {
            Some(Commands::Completions {
                command: Some(CompletionsCommand::Install { shell }),
                ..
            }) => assert_eq!(shell, Shell::Bash),
            _ => panic!("expected completions install command"),
        }
    }

    #[test]
    fn cli_accepts_repeatable_params_for_default_record() {
        let cli = Cli::try_parse_from([
            "ostt",
            "--param",
            "detect_language=false",
            "--param",
            "smart_format=true",
        ])
        .expect("parse cli");

        assert_eq!(
            cli.params,
            vec!["detect_language=false", "smart_format=true"]
        );
    }

    #[test]
    fn cli_accepts_params_for_transcribe() {
        let cli =
            Cli::try_parse_from(["ostt", "transcribe", "audio.ogg", "--param", "language=sv"])
                .expect("parse cli");

        assert_eq!(cli.params, vec!["language=sv"]);
        match cli.command {
            Some(Commands::Transcribe { .. }) => {}
            _ => panic!("expected transcribe command"),
        }
    }

    #[test]
    fn cli_collects_params_once_for_subcommands() {
        let cli = Cli::try_parse_from([
            "ostt",
            "transcribe",
            "audio.ogg",
            "--param",
            "diarize=true",
            "--param",
            "language=sv",
        ])
        .expect("parse cli");

        assert_eq!(cli.params, vec!["diarize=true", "language=sv"]);
    }

    #[test]
    fn cli_accepts_params_for_launch() {
        let cli =
            Cli::try_parse_from(["ostt", "launch", "--param", "language=sv"]).expect("parse cli");

        assert_eq!(cli.params, vec!["language=sv"]);
    }

    #[test]
    fn cli_accepts_global_params_before_subcommand() {
        let cli =
            Cli::try_parse_from(["ostt", "--param", "language=sv", "transcribe", "audio.ogg"])
                .expect("parse cli");

        assert_eq!(cli.params, vec!["language=sv"]);
    }
}
