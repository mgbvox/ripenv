//! `ripenv install` — install packages or sync from the lockfile.
//!
//! Two modes:
//! - No packages: equivalent to `ripenv sync` (install from lockfile).
//! - With packages: add to Pipfile, then lock + sync.

use anyhow::{Result, bail};

use crate::cli::InstallArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::pipfile::model::{PipfilePackage, PipfilePackageDetail};
use crate::printer::Printer;

/// Execute `ripenv install`.
pub fn execute(
    args: &InstallArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    if args.packages.is_empty() && args.requirements.is_none() {
        return install_from_lockfile(args, printer, verbosity, quiet);
    }

    install_packages(args, printer, verbosity, quiet)
}

/// `ripenv install` with no packages — sync from the lockfile.
fn install_from_lockfile(
    args: &InstallArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let ctx = UvContext::discover(printer, verbosity, quiet)?;

    // If --deploy, verify the lockfile is up to date first
    if args.deploy {
        let check = ctx.run_uv(&["lock", "--check"])?;
        if !check.success() {
            bail!("Lockfile is out of date (--deploy mode). Run `ripenv lock` first.");
        }
    }

    let mut uv_args = vec!["sync"];

    if args.no_dev {
        uv_args.push("--no-group=dev");
    }
    if args.system {
        uv_args.push("--python-preference=system");
    }

    let result = ctx.run_uv(&uv_args)?;

    if result.success() {
        ctx.printer.info("Install complete.");
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::External(result.exit_code))
    }
}

/// `ripenv install <PACKAGES>` — add packages to Pipfile, then lock + sync.
fn install_packages(
    args: &InstallArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let mut ctx = UvContext::discover(printer, verbosity, quiet)?;

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

    // Lock (unless --skip-lock)
    if !args.skip_lock {
        let mut lock_args = vec!["lock"];
        if args.pre {
            lock_args.push("--prerelease=allow");
        }
        let result = ctx.run_uv(&lock_args)?;
        if !result.success() {
            return Ok(ExitStatus::External(result.exit_code));
        }
    }

    // Sync
    let mut sync_args = vec!["sync"];
    if args.no_dev {
        sync_args.push("--no-group=dev");
    }

    let result = ctx.run_uv(&sync_args)?;

    if result.success() {
        ctx.printer.info("Install complete.");
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::External(result.exit_code))
    }
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
