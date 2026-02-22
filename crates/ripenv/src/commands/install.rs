//! `ripenv install` — install packages or sync from the lockfile.
//!
//! Two modes:
//! - No packages: equivalent to `ripenv sync` (install from lockfile).
//! - With packages: add to Pipfile, then lock + sync.

use anyhow::{Result, bail};
use uv_cache::{Cache, Refresh};
use uv_cli::SyncFormat;
use uv_configuration::{
    DependencyGroups, DryRun, EditableMode, ExtrasSpecification, InstallOptions,
};
use uv_resolver::PrereleaseMode;

use crate::cli::InstallArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::pipfile::model::{PipfilePackage, PipfilePackageDetail};
use crate::printer::Printer;

/// Execute `ripenv install`.
pub async fn execute(
    args: &InstallArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    if args.packages.is_empty() && args.requirements.is_none() {
        return Box::pin(install_from_lockfile(args, printer, verbosity, quiet)).await;
    }

    Box::pin(install_packages(args, printer, verbosity, quiet)).await
}

async fn do_lock(
    ctx: &UvContext,
    cache: &Cache,
    lock_check: uv::settings::LockCheck,
) -> Result<ExitStatus> {
    let lock_exit_status = uv::commands::project::lock::lock(
        &ctx.project_dir,
        lock_check,
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
        cache,
        ctx.uv_printer(),
        ctx.preview(),
    )
    .await?;

    Ok(lock_exit_status)
}

/// `ripenv install` with no packages — sync from the lockfile.
async fn install_from_lockfile(
    args: &InstallArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let ctx = UvContext::discover_or_init(printer, verbosity, quiet)?;

    // If --deploy, verify the lockfile is up to date first
    if args.deploy {
        let cache = ctx.cache()?;
        let check_result = do_lock(
            &ctx,
            &cache,
            uv::settings::LockCheck::Enabled(uv::settings::LockCheckSource::Check),
        )
        .await?;

        if !matches!(check_result, ExitStatus::Success) {
            bail!("Lockfile is out of date (--deploy mode). Run `ripenv lock` first.");
        }
    }

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
        ctx.generate_pipfile_lock()?;
        ctx.printer.info("Install complete.");
    }

    Ok(result)
}

/// `ripenv install <PACKAGES>` — add packages to Pipfile, then lock + sync.
async fn install_packages(
    args: &InstallArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let mut ctx = UvContext::discover_or_init(printer, verbosity, quiet)?;

    // Parse and add each package to the Pipfile
    for spec in &args.packages {
        let (name, package) = parse_package_spec(spec, args);

        if args.dev_packages {
            ctx.pipfile.dev_packages.insert(name, package);
        } else {
            ctx.pipfile.packages.insert(name, package);
        }
    }

    // Handle -r requirements.txt
    if let Some(ref req_file) = args.requirements {
        let content = fs_err::read_to_string(req_file)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
                continue;
            }
            let (name, package) = parse_requirement_line(line);
            ctx.pipfile.packages.insert(name, package);
        }
    }

    // Write updated Pipfile
    ctx.pipfile.write_to(&ctx.pipfile_path)?;

    // Regenerate virtual pyproject.toml
    ctx.refresh()?;

    let cache = ctx.cache()?;

    // Lock (unless --skip-lock)
    if !args.skip_lock {
        let mut settings = ctx.resolver_settings();
        if args.pre {
            settings.prerelease = PrereleaseMode::Allow;
        }

        let result = uv::commands::project::lock::lock(
            &ctx.project_dir,
            uv::settings::LockCheck::Disabled,
            None, // frozen
            DryRun::default(),
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
    }

    // Sync
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
        ctx.generate_pipfile_lock()?;
        ctx.printer.info("Install complete.");
    }

    Ok(result)
}

/// Parse a package spec string like `"requests"`, `"requests>=2.0"`, or `"requests[security]"`.
fn parse_package_spec(spec: &str, args: &InstallArgs) -> (String, PipfilePackage) {
    let (name, version) = split_name_version(spec);

    if args.editable {
        return (
            name.to_owned(),
            PipfilePackage::Detailed(PipfilePackageDetail {
                path: Some(spec.to_owned()),
                editable: true,
                ..PipfilePackageDetail::default()
            }),
        );
    }

    if version.is_empty() {
        (name.to_owned(), PipfilePackage::Simple("*".to_owned()))
    } else {
        (name.to_owned(), PipfilePackage::Simple(version.to_owned()))
    }
}

/// Split a package spec into name and version parts.
///
/// Examples: `"requests>=2.0"` -> `("requests", ">=2.0")`,
///           `"flask"` -> `("flask", "")`,
///           `"requests[security]>=2.0"` -> `("requests[security]", ">=2.0")`.
fn split_name_version(spec: &str) -> (&str, &str) {
    for (i, c) in spec.char_indices() {
        if matches!(c, '>' | '<' | '=' | '!' | '~') {
            return (&spec[..i], &spec[i..]);
        }
    }
    (spec, "")
}

/// Parse a requirements.txt line into a Pipfile package entry.
fn parse_requirement_line(line: &str) -> (String, PipfilePackage) {
    let (name, version) = split_name_version(line);
    if version.is_empty() {
        (name.to_owned(), PipfilePackage::Simple("*".to_owned()))
    } else {
        (name.to_owned(), PipfilePackage::Simple(version.to_owned()))
    }
}
