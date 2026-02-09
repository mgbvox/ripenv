//! CLI argument definitions for ripenv.
//!
//! All clap derive structs live here. The [`Cli`] struct is the top-level
//! parser; [`Commands`] enumerates every subcommand.

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};

/// Clap v3-style help menu colors, matching uv's convention.
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

/// A pipenv-compatible CLI powered by uv.
#[derive(Parser, Debug)]
#[command(
    name = "ripenv",
    author,
    version,
    about = "A pipenv-compatible CLI powered by uv.",
    styles = STYLES,
    after_help = "Use `ripenv help <command>` for more information on a specific command."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Increase logging verbosity.
    #[arg(global = true, short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output.
    #[arg(global = true, short, long)]
    pub quiet: bool,
}

/// Top-level subcommands for ripenv.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install packages from the lockfile, or add new packages to the Pipfile.
    Install(InstallArgs),

    /// Remove packages from the Pipfile and virtualenv.
    Uninstall(UninstallArgs),

    /// Generate or update the lockfile (uv.lock) from the Pipfile.
    Lock(LockArgs),

    /// Sync the virtualenv with the lockfile.
    Sync(SyncArgs),

    /// Update packages (re-lock then sync).
    #[command(alias = "upgrade")]
    Update(UpdateArgs),

    /// Run a command in the virtualenv, or a Pipfile script.
    Run(RunArgs),

    /// Spawn a shell with the virtualenv activated.
    Shell(ShellArgs),

    /// Display the dependency tree.
    Graph(GraphArgs),

    /// Export locked dependencies as requirements.txt.
    Requirements(RequirementsArgs),

    /// Remove packages not in the lockfile from the virtualenv.
    Clean(CleanArgs),

    /// List scripts defined in the Pipfile.
    Scripts(ScriptsArgs),

    /// Verify the lockfile is up to date with the Pipfile.
    Verify(VerifyArgs),

    /// Deprecated: use `ripenv audit` instead.
    Check(CheckArgs),

    /// Audit installed packages for known vulnerabilities.
    Audit(AuditArgs),
}

impl Commands {
    /// Return the subcommand name as a static string (for diagnostics).
    pub fn name(&self) -> &'static str {
        match self {
            Self::Install(_) => "install",
            Self::Uninstall(_) => "uninstall",
            Self::Lock(_) => "lock",
            Self::Sync(_) => "sync",
            Self::Update(_) => "update",
            Self::Run(_) => "run",
            Self::Shell(_) => "shell",
            Self::Graph(_) => "graph",
            Self::Requirements(_) => "requirements",
            Self::Clean(_) => "clean",
            Self::Scripts(_) => "scripts",
            Self::Verify(_) => "verify",
            Self::Check(_) => "check",
            Self::Audit(_) => "audit",
        }
    }
}

/// Arguments for `ripenv install`.
#[derive(Parser, Debug)]
pub struct InstallArgs {
    /// Packages to install. If omitted, syncs from the lockfile.
    pub packages: Vec<String>,

    /// Exclude dev dependencies.
    ///
    /// By default, dev dependencies are included (matching pipenv behavior).
    /// Pass `--no-dev` to exclude them (e.g., in CI/production).
    #[arg(long = "no-dev")]
    pub no_dev: bool,

    /// Install into the system Python instead of a virtualenv.
    #[arg(long)]
    pub system: bool,

    /// Fail if the lockfile is out of date.
    #[arg(long)]
    pub deploy: bool,

    /// Install from a requirements file.
    #[arg(short = 'r', long = "requirements")]
    pub requirements: Option<String>,

    /// Add packages to dev-packages instead of packages.
    #[arg(short = 'd', long = "dev-packages")]
    pub dev_packages: bool,

    /// Allow pre-release versions.
    #[arg(long)]
    pub pre: bool,

    /// Install as editable.
    #[arg(short, long)]
    pub editable: bool,

    /// Skip locking after adding packages.
    #[arg(long)]
    pub skip_lock: bool,

    /// Specify the package index to use.
    #[arg(long)]
    pub index: Option<String>,
}

impl InstallArgs {
    /// Whether dev dependencies should be included (default: true).
    pub fn include_dev(&self) -> bool {
        !self.no_dev
    }
}

/// Arguments for `ripenv uninstall`.
#[derive(Parser, Debug)]
pub struct UninstallArgs {
    /// Packages to remove.
    pub packages: Vec<String>,

    /// Remove from dev-packages.
    #[arg(long)]
    pub dev: bool,

    /// Remove all packages.
    #[arg(long)]
    pub all: bool,

    /// Remove all dev packages.
    #[arg(long)]
    pub all_dev: bool,

    /// Skip re-locking after removal.
    #[arg(long)]
    pub skip_lock: bool,
}

/// Arguments for `ripenv lock`.
#[derive(Parser, Debug)]
pub struct LockArgs {
    /// Only lock dev dependencies.
    #[arg(long)]
    pub dev_only: bool,

    /// Allow pre-release versions.
    #[arg(long)]
    pub pre: bool,

    /// Clear resolver caches.
    #[arg(long)]
    pub clear: bool,
}

/// Arguments for `ripenv sync`.
#[derive(Parser, Debug)]
pub struct SyncArgs {
    /// Exclude dev dependencies.
    ///
    /// By default, dev dependencies are included (matching pipenv behavior).
    #[arg(long = "no-dev")]
    pub no_dev: bool,

    /// Install into the system Python.
    #[arg(long)]
    pub system: bool,
}

impl SyncArgs {
    /// Whether dev dependencies should be included (default: true).
    pub fn include_dev(&self) -> bool {
        !self.no_dev
    }
}

/// Arguments for `ripenv update`.
#[derive(Parser, Debug)]
pub struct UpdateArgs {
    /// Packages to update. If omitted, updates all.
    pub packages: Vec<String>,

    /// Show what would change without applying.
    #[arg(long)]
    pub dry_run: bool,

    /// Include dev dependencies.
    #[arg(long)]
    pub dev: bool,

    /// Only update the lockfile, do not sync.
    #[arg(long)]
    pub lock_only: bool,
}

/// Arguments for `ripenv run`.
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// The command (or Pipfile script name) to run.
    pub command: String,

    /// Arguments to pass to the command.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,

    /// Run in the system Python.
    #[arg(long)]
    pub system: bool,
}

/// Arguments for `ripenv shell`.
#[derive(Parser, Debug)]
pub struct ShellArgs;

/// Arguments for `ripenv graph`.
#[derive(Parser, Debug)]
pub struct GraphArgs {
    /// Output as bare package names.
    #[arg(long)]
    pub bare: bool,

    /// Output as JSON.
    #[arg(long)]
    pub json: bool,

    /// Show reverse dependencies.
    #[arg(long)]
    pub reverse: bool,
}

/// Arguments for `ripenv requirements`.
#[derive(Parser, Debug)]
pub struct RequirementsArgs {
    /// Include dev dependencies.
    #[arg(long)]
    pub dev: bool,

    /// Only dev dependencies.
    #[arg(long)]
    pub dev_only: bool,

    /// Include hashes.
    #[arg(long)]
    pub hash: bool,
}

/// Arguments for `ripenv clean`.
#[derive(Parser, Debug)]
pub struct CleanArgs {
    /// Show what would be removed without removing.
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for `ripenv scripts`.
#[derive(Parser, Debug)]
pub struct ScriptsArgs;

/// Arguments for `ripenv verify`.
#[derive(Parser, Debug)]
pub struct VerifyArgs;

/// Arguments for `ripenv check`.
#[derive(Parser, Debug)]
pub struct CheckArgs;

/// Arguments for `ripenv audit`.
#[derive(Parser, Debug)]
pub struct AuditArgs;
