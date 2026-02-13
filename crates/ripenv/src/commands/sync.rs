//! `ripenv sync` — sync the virtualenv with the lockfile.

use anyhow::Result;
use uv_cli::SyncFormat;
use uv_configuration::{
    DependencyGroups, DryRun, EditableMode, ExtrasSpecification, InstallOptions,
};

use crate::cli::SyncArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv sync`.
pub async fn execute(
    args: &SyncArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let ctx = UvContext::discover(printer, verbosity, quiet)?;

    let groups = DependencyGroups::from_args(
        false,       // dev
        args.no_dev, // no_dev
        false,       // only_dev
        vec![],      // group
        vec![],      // no_group
        false,       // no_default_groups
        vec![],      // only_group
        false,       // all_groups
    );

    let python_preference = if args.system {
        uv_python::PythonPreference::System
    } else {
        ctx.python_preference()
    };

    let cache = ctx.cache()?;

    let result = Box::pin(uv::commands::project::sync::sync(
        &ctx.project_dir,
        uv::settings::LockCheck::Disabled,
        None, // frozen
        DryRun::default(),
        None,   // active
        false,  // all_packages
        vec![], // package
        ExtrasSpecification::default(),
        groups,
        Some(EditableMode::default()),
        InstallOptions::default(),
        uv::commands::pip::operations::Modifications::Exact,
        None, // python
        None, // python_platform
        ctx.install_mirrors(),
        python_preference,
        ctx.python_downloads(),
        ctx.resolver_installer_settings(),
        ctx.client_builder(),
        None,  // script
        false, // installer_metadata
        ctx.concurrency(),
        false, // no_config
        &cache,
        ctx.uv_printer(),
        ctx.preview(),
        SyncFormat::default(),
    ))
    .await?;

    if matches!(result, ExitStatus::Success) {
        ctx.printer.info("Sync complete.");
    }

    Ok(result)
}
