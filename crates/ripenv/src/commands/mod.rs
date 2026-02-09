//! Command dispatch for ripenv.
//!
//! Each subcommand is dispatched here. Currently all commands except `check`
//! are stubs that return [`ExitStatus::Failure`] with a "not yet implemented"
//! message. As phases are implemented, each arm will delegate to the real
//! command handler.

use std::process::ExitCode;

use anyhow::Result;

use crate::cli;
use crate::printer::Printer;

/// Exit status for ripenv commands.
#[derive(Copy, Clone)]
pub enum ExitStatus {
    /// The command succeeded.
    Success,

    /// The command failed due to an error in the user input.
    Failure,

    /// The command failed with an unexpected error.
    Error,

    /// The command's exit status is propagated from an external command.
    External(u8),
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        match status {
            ExitStatus::Success => Self::from(0),
            ExitStatus::Failure => Self::from(1),
            ExitStatus::Error => Self::from(2),
            ExitStatus::External(code) => Self::from(code),
        }
    }
}

/// Dispatch a parsed CLI command to the appropriate handler.
///
/// This function is async to support delegation to uv's async project commands
/// (e.g., `sync`, `lock`, `add`, `remove`) in Phase 2.
#[expect(clippy::unused_async, reason = "will await uv commands in Phase 2")]
pub async fn dispatch(command: cli::Commands, printer: Printer) -> Result<ExitStatus> {
    match command {
        cli::Commands::Check(_) => {
            printer.warn("ripenv check is deprecated. Use `ripenv audit` instead.");
            Ok(ExitStatus::Failure)
        }

        // All other commands are stubs until their phase is implemented.
        command => {
            let name = command.name();
            printer.warn(&format!("ripenv {name} is not yet implemented."));
            Ok(ExitStatus::Failure)
        }
    }
}
