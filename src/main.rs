mod commands;
mod conflict;
mod id;
mod identity;
mod store;
mod symbols;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "grits", about = "Intent WAL for parallel AI agent coordination")]
struct Cli {
    /// Output as JSON (for agent consumption)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Declare intent to modify a file:symbol
    Claim {
        /// Target in file:symbol format (symbol optional)
        target: String,
    },

    /// Check if a file:symbol is safe to modify
    Check {
        /// Target in file:symbol format (symbol optional)
        target: String,
    },

    /// Release a claim after work is done
    Release {
        /// Claim ID (e.g., gs-a3f8)
        id: String,

        /// Git commit SHA
        #[arg(long)]
        commit: String,
    },

    /// Show all active claims
    Status,

    /// Show who last modified a file:symbol
    Blame {
        /// Target in file:symbol format
        target: String,
    },

    /// Show history for a file:symbol or agent
    Log {
        /// Target in file:symbol format
        target: Option<String>,

        /// Filter by agent type
        #[arg(long)]
        agent: Option<String>,
    },

    /// Print the agent primer block
    Prime,

    /// Initialize grits in the current repository
    Init {
        /// Reinitialize even if .grits/ already exists
        #[arg(long)]
        force: bool,
    },

    /// Manage agent workflow guidance in AGENTS.md / CLAUDE.md
    Agents {
        /// Add grits blurb to agent file
        #[arg(long, conflicts_with = "remove")]
        add: bool,

        /// Remove grits blurb from agent file
        #[arg(long, conflicts_with = "add")]
        remove: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Claim { target } => commands::claim::run(&target, cli.json),
        Command::Check { target } => commands::check::run(&target, cli.json),
        Command::Release { id, commit } => commands::release::run(&id, &commit, cli.json),
        Command::Status => commands::status::run(cli.json),
        Command::Blame { target } => commands::blame::run(&target, cli.json),
        Command::Log { target, agent } => commands::log::run(target.as_deref(), agent.as_deref(), cli.json),
        Command::Prime => commands::prime::run(),
        Command::Init { force } => commands::init::run(force, cli.json),
        Command::Agents { add, remove, force } => {
            let mode = if add {
                commands::agents::Mode::Add
            } else if remove {
                commands::agents::Mode::Remove
            } else {
                commands::agents::Mode::Check
            };
            commands::agents::run(mode, force, cli.json)
        }
    };

    if let Err(e) = result {
        if cli.json {
            let err = serde_json::json!({
                "error": e.code,
                "message": e.message,
                "hint": e.hint,
                "retryable": e.retryable,
            });
            eprintln!("{}", serde_json::to_string(&err).unwrap());
        } else {
            eprintln!("error: {}", e.message);
            if let Some(hint) = &e.hint {
                eprintln!("hint: {}", hint);
            }
        }
        std::process::exit(e.exit_code);
    }
}

/// Parse a "file:symbol" target string into (file, optional symbol).
pub fn parse_target(target: &str) -> (String, Option<String>) {
    match target.split_once(':') {
        Some((file, symbol)) => (file.to_string(), Some(symbol.to_string())),
        None => (target.to_string(), None),
    }
}

/// Find the project root by looking for `.git` directory walking up from cwd.
/// Falls back to cwd if no `.git` found.
pub fn find_root() -> Result<std::path::PathBuf, GritsError> {
    let cwd = std::env::current_dir()
        .map_err(|e| GritsError::io(format!("failed to get cwd: {e}")))?;

    let mut dir = cwd.as_path();
    loop {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return Ok(cwd),
        }
    }
}

/// Structured error type matching the design doc envelope.
#[derive(Debug)]
pub struct GritsError {
    pub code: &'static str,
    pub message: String,
    pub hint: Option<String>,
    pub retryable: bool,
    pub exit_code: i32,
}

impl GritsError {
    pub fn conflict(message: String, hint: String) -> Self {
        Self { code: "CONFLICT", message, hint: Some(hint), retryable: true, exit_code: 1 }
    }

    pub fn invalid_input(message: String) -> Self {
        Self { code: "INVALID_INPUT", message, hint: None, retryable: true, exit_code: 2 }
    }

    pub fn invalid_input_with_hint(message: String, hint: String) -> Self {
        Self { code: "INVALID_INPUT", message, hint: Some(hint), retryable: true, exit_code: 2 }
    }

    pub fn io(message: String) -> Self {
        Self { code: "IO_ERROR", message, hint: None, retryable: false, exit_code: 3 }
    }
}
