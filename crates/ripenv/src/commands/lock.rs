//! `ripenv lock` — generate or update the lockfile from the Pipfile.

use anyhow::Result;
use uv_cache::Refresh;
use uv_configuration::DryRun;
use uv_resolver::PrereleaseMode;

use crate::cli::LockArgs;
use crate::commands::ExitStatus;
use crate::commands::uv_runner::UvContext;
use crate::printer::Printer;

/// Execute `ripenv lock`.
pub async fn execute(
    args: &LockArgs,
    printer: Printer,
    verbosity: u8,
    quiet: bool,
) -> Result<ExitStatus> {
    let ctx = UvContext::discover(printer, verbosity, quiet)?;

    let mut settings = ctx.resolver_settings();

    if args.pre {
        settings.prerelease = PrereleaseMode::Allow;
    }

    let refresh = if args.clear {
        Refresh::from_args(Some(true), vec![])
    } else {
        Refresh::from_args(None, vec![])
    };

    let cache = ctx.cache()?.with_refresh(refresh.clone());

    if args.dev_only {
        // uv lock doesn't have --dev-only directly; lock always resolves everything.
        // This flag is a no-op for lock (it affects sync/install behavior).
        ctx.printer
            .debug("--dev-only has no effect on lock (all deps are always resolved)");
    }

    let result = uv::commands::project::lock::lock(
        &ctx.project_dir,
        uv::settings::LockCheck::Disabled,
        None, // frozen
        DryRun::default(),
        refresh,
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

    if matches!(result, ExitStatus::Success) {
        ctx.generate_pipfile_lock()?;
        ctx.printer.info("Locking successful.");
    }

    Ok(result)
}
