//! `ripenv uninstall` — remove packages from the Pipfile and virtualenv.

use anyhow::{Result, bail};

use crate::cli::UninstallArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv uninstall`.
pub fn execute(
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

    // Re-lock (unless --skip-lock)
    if !args.skip_lock {
        let result = ctx.run_uv(&["lock"])?;
        if !result.success() {
            return Ok(ExitStatus::External(result.exit_code));
        }
    }

    // Sync to remove unneeded packages from venv
    let result = ctx.run_uv(&["sync"])?;

    if result.success() {
        ctx.printer.info("Uninstall complete.");
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::External(result.exit_code))
    }
}
