//! Output formatting for ripenv commands.
//!
//! The [`Printer`] controls whether messages are emitted to stderr based on
//! the user's `--quiet` and `--verbose` flags. Errors are always printed
//! regardless of quiet mode (matching uv's behavior).

use anstream::eprintln;
use owo_colors::OwoColorize;

/// Controls output formatting for ripenv commands.
#[derive(Copy, Clone)]
pub struct Printer {
    /// Verbosity level: 0 = normal, 1+ = verbose.
    verbosity: u8,
    /// Whether output is suppressed.
    quiet: bool,
}

impl Printer {
    /// Create a new printer with the given verbosity and quiet settings.
    pub fn new(verbosity: u8, quiet: bool) -> Self {
        Self { verbosity, quiet }
    }

    /// Print an informational message to stderr.
    pub fn info(&self, message: &str) {
        if !self.quiet {
            eprintln!("{}", message);
        }
    }

    /// Print a warning message to stderr.
    pub fn warn(&self, message: &str) {
        if !self.quiet {
            eprintln!("{}: {}", "warning".yellow().bold(), message);
        }
    }

    /// Print an error message to stderr.
    ///
    /// Errors are always printed, even in quiet mode, because suppressing
    /// error output would hide actionable failures from the user.
    pub fn error(&self, message: &str) {
        eprintln!("{}: {}", "error".red().bold(), message);
    }

    /// Print a debug message (only at verbosity >= 1).
    pub fn debug(&self, message: &str) {
        if self.verbosity >= 1 && !self.quiet {
            eprintln!("{}: {}", "debug".dimmed(), message);
        }
    }
}
