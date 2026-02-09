//! ripenv: a pipenv-compatible CLI powered by uv.
//!
//! This crate provides the main entry point and command dispatch for the ripenv
//! binary. It parses CLI arguments, sets up a tokio runtime, and delegates to
//! command handlers that bridge Pipfile semantics onto uv's project machinery.

#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::ffi::OsString;
use std::process::ExitCode;

use anstream::eprintln;
use clap::Parser;
use owo_colors::OwoColorize;

use uv_configuration::min_stack_size;

use crate::cli::Cli;
use crate::commands::ExitStatus;
use crate::printer::Printer;

pub mod cli;
pub mod commands;
pub mod pipfile;
pub mod printer;

/// Entry point for the ripenv CLI.
///
/// Parses CLI arguments, sets up the tokio runtime on a dedicated thread
/// (see [`min_stack_size`]), and dispatches to the appropriate command handler.
pub fn main<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(err) => err.exit(),
    };

    let printer = Printer::new(cli.verbose, cli.quiet);

    // Run on a dedicated thread with a larger stack to match uv's convention.
    // See `min_stack_size` doc comment for rationale.
    let min_stack_size = min_stack_size();
    let run = move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_stack_size(min_stack_size)
            .build()
            .expect("failed to build tokio runtime");

        let result = runtime.block_on(commands::dispatch(
            cli.command,
            printer,
            cli.verbose,
            cli.quiet,
        ));

        runtime.shutdown_background();
        result
    };

    // These .expect() calls mirror uv's pattern — thread spawn/join failures
    // are unrecoverable and warrant a panic.
    let result = std::thread::Builder::new()
        .name("ripenv-main".to_owned())
        .stack_size(min_stack_size)
        .spawn(run)
        .expect("failed to spawn main thread")
        .join()
        .expect("main thread panicked");

    match result {
        Ok(code) => code.into(),
        Err(err) => {
            let mut causes = err.chain();
            // An anyhow::Error always has at least one cause (itself).
            printer.error(
                &causes
                    .next()
                    .expect("error chain is never empty")
                    .to_string(),
            );
            for cause in causes {
                eprintln!(
                    "  {}: {}",
                    "Caused by".red().bold(),
                    cause.to_string().trim()
                );
            }
            ExitStatus::Error.into()
        }
    }
}
