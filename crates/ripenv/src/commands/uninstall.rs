//! `ripenv uninstall` — remove packages from the Pipfile and virtualenv.

use anyhow::{Result, bail};
use uv_cache::Refresh;
use uv_cli::SyncFormat;
use uv_configuration::{
    DependencyGroups, DryRun, EditableMode, ExtrasSpecification, InstallOptions,
};

use crate::cli::UninstallArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv uninstall`.
pub async fn execute(
    args: &UninstallArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let mut ctx = UvContext::discover(printer, verbosity, quiet)?;

    // Determine which packages to remove
    if args.all {
        ctx.pipfile.packages.clear();
        ctx.pipfile.dev_packages.clear();
        ctx.printer.info("Removed all packages from Pipfile.");
    } else if args.all_dev {
        ctx.pipfile.dev_packages.clear();
        ctx.printer.info("Removed all dev packages from Pipfile.");
    } else if args.packages.is_empty() {
        bail!("No packages specified. Use --all or --all-dev to remove all packages.");
    } else {
        for name in &args.packages {
            // Without --dev: try removing from [packages] first, then [dev-packages].
            // With --dev: only remove from [dev-packages].
            let removed_from_packages = if args.dev {
                false
            } else {
                ctx.pipfile.packages.remove(name).is_some()
            };
            let removed_from_dev = ctx.pipfile.dev_packages.remove(name).is_some();

            if !removed_from_packages && !removed_from_dev {
                ctx.printer
                    .warn(&format!("Package '{name}' not found in Pipfile."));
            } else {
                ctx.printer.info(&format!("Removed '{name}' from Pipfile."));
            }
        }
    }

    // Write updated Pipfile
    ctx.pipfile.write_to(&ctx.pipfile_path)?;

    // Regenerate virtual pyproject.toml
    ctx.refresh()?;

    let cache = ctx.cache()?;

    // Re-lock (unless --skip-lock)
    if !args.skip_lock {
        let result = uv::commands::project::lock::lock(
            &ctx.project_dir,
            uv::settings::LockCheck::Disabled,
            None, // frozen
            DryRun::default(),
            Refresh::from_args(None, vec![]),
            None, // python
            ctx.install_mirrors(),
            ctx.resolver_settings(),
            ctx.client_builder(),
            None, // script
            ctx.python_preference(),
            ctx.python_downloads(),
            ctx.concurrency(),
            false, // no_config
            &cache,
            ctx.uv_printer(),
            ctx.preview(),
        )
        .await?;

        if !matches!(result, ExitStatus::Success) {
            return Ok(result);
        }
    }

    // Sync to remove unneeded packages from venv
    let result = Box::pin(uv::commands::project::sync::sync(
        &ctx.project_dir,
        uv::settings::LockCheck::Disabled,
        None, // frozen
        DryRun::default(),
        None,   // active
        false,  // all_packages
        vec![], // package
        ExtrasSpecification::default(),
        DependencyGroups::default(),
        Some(EditableMode::default()),
        InstallOptions::default(),
        uv::commands::pip::operations::Modifications::Exact,
        None, // python
        None, // python_platform
        ctx.install_mirrors(),
        ctx.python_preference(),
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
        ctx.printer.info("Uninstall complete.");
    }

    Ok(result)
}
