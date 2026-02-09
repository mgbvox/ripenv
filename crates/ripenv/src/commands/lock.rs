//! `ripenv lock` — generate or update the lockfile from the Pipfile.

use anyhow::Result;

use crate::cli::LockArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv lock`.
pub fn execute(
    args: &LockArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let ctx = UvContext::discover(printer, verbosity, quiet)?;

    let mut uv_args = vec!["lock"];

    if args.pre {
        uv_args.push("--prerelease=allow");
    }
    if args.clear {
        uv_args.push("--no-cache");
    }
    if args.dev_only {
        // uv lock doesn't have --dev-only directly; lock always resolves everything.
        // This flag is a no-op for lock (it affects sync/install behavior).
        ctx.printer
            .debug("--dev-only has no effect on lock (all deps are always resolved)");
    }

    let result = ctx.run_uv(&uv_args)?;

    if result.success() {
        ctx.printer.info("Locking successful.");
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::External(result.exit_code))
    }
}
