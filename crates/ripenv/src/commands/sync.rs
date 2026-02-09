//! `ripenv sync` — sync the virtualenv with the lockfile.

use anyhow::Result;

use crate::cli::SyncArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv sync`.
pub fn execute(
    args: &SyncArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let ctx = UvContext::discover(printer, verbosity, quiet)?;

    let mut uv_args = vec!["sync"];

    if args.no_dev {
        uv_args.push("--no-group=dev");
    }
    if args.system {
        uv_args.push("--python-preference=system");
    }

    let result = ctx.run_uv(&uv_args)?;

    if result.success() {
        ctx.printer.info("Sync complete.");
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::External(result.exit_code))
    }
}
