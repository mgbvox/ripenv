//! Command dispatch for ripenv.
//!
//! Each subcommand is dispatched here. Commands call uv's project functions
//! directly as library calls via the [`uv_runner`] module.

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

/// Re-export uv's `ExitStatus` as our own.
pub use uv::commands::ExitStatus;

/// Dispatch a parsed CLI command to the appropriate handler.
pub async fn dispatch(
    command: cli::Commands,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    match command {
        cli::Commands::Install(ref args) => {
            Box::pin(install::execute(args, printer, verbosity, quiet)).await
        }
        cli::Commands::Uninstall(ref args) => {
            Box::pin(uninstall::execute(args, printer, verbosity, quiet)).await
        }
        cli::Commands::Lock(ref args) => lock::execute(args, printer, verbosity, quiet).await,
        cli::Commands::Sync(ref args) => {
            Box::pin(sync::execute(args, printer, verbosity, quiet)).await
        }
        cli::Commands::Update(ref args) => {
            Box::pin(update::execute(args, printer, verbosity, quiet)).await
        }
        cli::Commands::Run(ref args) => {
            Box::pin(run::execute(args, printer, verbosity, quiet)).await
        }
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
