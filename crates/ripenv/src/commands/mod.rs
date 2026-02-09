//! Command dispatch for ripenv.
//!
//! Each subcommand is dispatched here. Phase 2 commands delegate to uv via
//! the [`uv_runner`] module; remaining stubs return
//! [`ExitStatus::Failure`] with a "not yet implemented" message.

use std::process::ExitCode;

use anyhow::Result;

use crate::cli;
use crate::printer::Printer;

pub mod install;
pub mod lock;
pub mod run;
pub mod sync;
pub mod uninstall;
pub mod update;
pub mod uv_runner;

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
#[expect(clippy::unused_async, reason = "will await uv commands in future")]
pub async fn dispatch(
    command: cli::Commands,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    match command {
        cli::Commands::Install(ref args) => install::execute(args, printer, verbosity, quiet),
        cli::Commands::Uninstall(ref args) => uninstall::execute(args, printer, verbosity, quiet),
        cli::Commands::Lock(ref args) => lock::execute(args, printer, verbosity, quiet),
        cli::Commands::Sync(ref args) => sync::execute(args, printer, verbosity, quiet),
        cli::Commands::Update(ref args) => update::execute(args, printer, verbosity, quiet),
        cli::Commands::Run(ref args) => run::execute(args, printer, verbosity, quiet),
        cli::Commands::Check(_) => {
            printer.warn("ripenv check is deprecated. Use `ripenv audit` instead.");
            Ok(ExitStatus::Failure)
        }

        // Remaining stubs (Phase 3+)
        command => {
            let name = command.name();
            printer.warn(&format!("ripenv {name} is not yet implemented."));
            Ok(ExitStatus::Failure)
        }
    }
}
