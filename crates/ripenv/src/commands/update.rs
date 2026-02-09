//! `ripenv update` — update packages (re-lock then sync).

use anyhow::Result;

use crate::cli::UpdateArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv update`.
pub fn execute(
    args: &UpdateArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let ctx = UvContext::discover(printer, verbosity, quiet)?;

    // Build lock args
    let mut lock_args = vec!["lock", "--upgrade"];

    // If specific packages, only upgrade those
    let upgrade_packages: Vec<String> = args
        .packages
        .iter()
        .map(|p| format!("--upgrade-package={p}"))
        .collect();
    if !args.packages.is_empty() {
        // Remove the blanket --upgrade, use per-package instead
        lock_args.pop();
        for pkg in &upgrade_packages {
            lock_args.push(pkg);
        }
    }

    if args.dry_run {
        lock_args.push("--dry-run");
    }

    // Re-lock
    let result = ctx.run_uv(&lock_args)?;
    if !result.success() {
        return Ok(ExitStatus::External(result.exit_code));
    }

    // Sync (unless --lock-only or --dry-run)
    if !args.lock_only && !args.dry_run {
        let result = ctx.run_uv(&["sync"])?;
        if !result.success() {
            return Ok(ExitStatus::External(result.exit_code));
        }
    }

    ctx.printer.info("Update complete.");
    Ok(ExitStatus::Success)
}
