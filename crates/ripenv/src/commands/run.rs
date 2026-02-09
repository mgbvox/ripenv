//! `ripenv run` — run a command in the virtualenv, or a Pipfile script.

use anyhow::{Context, Result};

use crate::cli::RunArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv run`.
pub fn execute(args: &RunArgs, printer: Printer, verbosity: u8, quiet: bool) -> Result<ExitStatus> {
    let ctx = UvContext::discover(printer, verbosity, quiet)?;

    // Check if the command is a Pipfile script
    let (command, extra_args) = if let Some(script) = ctx.pipfile.scripts.get(&args.command) {
        ctx.printer.debug(&format!(
            "Expanding script '{}' -> '{script}'",
            args.command
        ));

        // Split the script into command + args
        let mut parts = script.split_whitespace();
        let cmd = parts.next().context("script is empty")?.to_owned();
        let mut script_args: Vec<String> = parts.map(String::from).collect();
        // Append any extra args passed on the command line
        script_args.extend(args.args.clone());
        (cmd, script_args)
    } else {
        (args.command.clone(), args.args.clone())
    };

    // Build uv run command
    let mut uv_args: Vec<String> = vec!["run".to_owned(), "--".to_owned(), command];
    uv_args.extend(extra_args);

    let uv_args_refs: Vec<&str> = uv_args.iter().map(String::as_str).collect();
    let result = ctx.run_uv(&uv_args_refs)?;

    Ok(ExitStatus::External(result.exit_code))
}
