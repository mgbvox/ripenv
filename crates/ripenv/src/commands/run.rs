//! `ripenv run` — run a command in the virtualenv, or a Pipfile script.

use std::ffi::OsString;

use anyhow::{Context, Result};
use uv_configuration::{DependencyGroups, EditableMode, EnvFile, ExtrasSpecification};

use crate::cli::RunArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv run`.
pub async fn execute(
    args: &RunArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
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

    // Build a RunCommand::External for uv
    let command_os = OsString::from(&command);
    let extra_args_os: Vec<OsString> = extra_args.iter().map(OsString::from).collect();
    let run_command = uv::commands::project::run::RunCommand::External(command_os, extra_args_os);

    let cache = ctx.cache()?;

    let result = Box::pin(uv::commands::project::run::run(
        &ctx.project_dir,
        None, // script (PEP 723)
        Some(run_command),
        vec![], // requirements
        false,  // show_resolution
        uv::settings::LockCheck::Disabled,
        None,  // frozen
        None,  // active
        false, // no_sync
        false, // isolated
        false, // all_packages
        None,  // package
        false, // no_project
        false, // no_config
        ExtrasSpecification::default(),
        DependencyGroups::default(),
        Some(EditableMode::default()),
        uv::commands::pip::operations::Modifications::Sufficient,
        None, // python
        None, // python_platform
        ctx.install_mirrors(),
        ctx.resolver_installer_settings(),
        ctx.client_builder(),
        ctx.python_preference(),
        ctx.python_downloads(),
        false, // installer_metadata
        ctx.concurrency(),
        cache,
        ctx.uv_printer(),
        EnvFile::default(),
        ctx.preview(),
        50, // max_recursion_depth
    ))
    .await?;

    Ok(result)
}
