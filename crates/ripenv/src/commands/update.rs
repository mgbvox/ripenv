//! `ripenv update` — update packages (re-lock then sync).

use std::str::FromStr;

use anyhow::Result;
use rustc_hash::FxHashMap;
use uv_cache::Refresh;
use uv_cli::SyncFormat;
use uv_configuration::{
    DependencyGroups, DryRun, EditableMode, ExtrasSpecification, InstallOptions, Upgrade,
};
use uv_normalize::PackageName;

use crate::cli::UpdateArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv update`.
pub async fn execute(
    args: &UpdateArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let ctx = UvContext::discover(printer, verbosity, quiet)?;

    // Build lock settings with upgrade
    let mut settings = ctx.resolver_settings();

    if args.packages.is_empty() {
        settings.upgrade = Upgrade::All;
    } else {
        let mut packages = FxHashMap::default();
        for p in &args.packages {
            let name = PackageName::from_str(p)?;
            packages.insert(name, vec![]);
        }
        settings.upgrade = Upgrade::Packages(packages);
    }

    let dry_run = if args.dry_run {
        DryRun::Enabled
    } else {
        DryRun::default()
    };

    let cache = ctx.cache()?;

    // Re-lock
    let result = uv::commands::project::lock::lock(
        &ctx.project_dir,
        uv::settings::LockCheck::Disabled,
        None, // frozen
        dry_run,
        Refresh::from_args(None, vec![]),
        None, // python
        ctx.install_mirrors(),
        settings,
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

    ctx.generate_pipfile_lock()?;

    // Sync (unless --lock-only or --dry-run)
    if !args.lock_only && !args.dry_run {
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

        if !matches!(result, ExitStatus::Success) {
            return Ok(result);
        }
    }

    ctx.printer.info("Update complete.");
    Ok(ExitStatus::Success)
}
