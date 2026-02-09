// The `unreachable_pub` is to silence false positives in RustRover.
#![allow(dead_code, unreachable_pub)]

use std::path::PathBuf;
use std::process::Command;

/// Insta snapshot filters shared across ripenv tests.
pub const INSTA_FILTERS: &[(&str, &str)] = &[
    // Operation times
    (r"(\s|\()(\d+m )?(\d+\.)?\d+(ms|s)", "$1[TIME]"),
    // Rewrite Windows output to Unix output
    (r"\\([\w\d]|\.)", "/$1"),
    (r"ripenv\.exe", "ripenv"),
    // ripenv version display
    (
        r"ripenv \d+\.\d+\.\d+(-(alpha|beta|rc)\.\d+)?(\+\d+)?",
        r"ripenv [VERSION]",
    ),
    // Trim end-of-line whitespaces
    (r"([^\s])[ \t]+(\r?\n)", "$1$2"),
];

/// Returns the ripenv binary that cargo built before launching the tests.
pub fn get_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ripenv"))
}

/// Create a `ripenv` command for testing.
pub fn ripenv_command() -> Command {
    let mut command = Command::new(get_bin());
    // Clear environment variables that might interfere with tests.
    command.env_remove("PIPENV_VENV_IN_PROJECT");
    command.env_remove("PIPENV_PYTHON");
    command.env_remove("PIPENV_PIPFILE");
    command.env_remove("PIPENV_CACHE_DIR");
    command.env_remove("PIPENV_YES");
    command.env_remove("PIPENV_SKIP_LOCK");
    command.env_remove("PIPENV_DEFAULT_PYTHON_VERSION");
    command.env_remove("PIPENV_PYPI_MIRROR");
    command
}

/// Create a `ripenv help` command.
pub fn ripenv_help() -> Command {
    let mut command = ripenv_command();
    command.arg("help");
    command
}

/// Snapshot test helper macro. Runs a command and asserts against an insta snapshot.
#[macro_export]
macro_rules! ripenv_snapshot {
    ($filters:expr, $command:expr, @$expected:literal) => {{
        let output = $command.output().expect("Failed to execute ripenv");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut combined = format!(
            "success: {:?}\nexit_code: {}\n----- stdout -----\n{}\n----- stderr -----\n{}",
            output.status.success(),
            output.status.code().unwrap_or(-1),
            stdout.trim(),
            stderr.trim(),
        );

        // Apply filters
        for (pattern, replacement) in $filters.iter() {
            let re = regex::Regex::new(pattern).expect("Invalid filter regex");
            combined = re.replace_all(&combined, *replacement).to_string();
        }

        insta::assert_snapshot!(combined, @$expected);
    }};
}
