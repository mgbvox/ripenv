//! Parity tests between pipenv and ripenv CLIs.
//!
//! These tests verify that ripenv's implemented endpoints mirror pipenv's
//! behavior: same subcommand names, compatible flag names, similar help
//! structure, and matching exit codes for equivalent operations.

use crate::common::ripenv_command;

// ─── Help & version structure ────────────────────────────────────────────────

#[test]
fn help_lists_all_pipenv_subcommands() {
    // pipenv exposes these subcommands; ripenv must have all of them
    // (except `open`, `pylock`, and `activate` which are non-goals for MVP).
    let pipenv_commands = [
        "install",
        "uninstall",
        "lock",
        "sync",
        "update",
        "run",
        "shell",
        "graph",
        "requirements",
        "clean",
        "scripts",
        "verify",
        "check",
        "audit",
    ];

    let mut cmd = ripenv_command();
    cmd.arg("--help");
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for command in pipenv_commands {
        assert!(
            stdout.contains(command),
            "ripenv --help is missing pipenv subcommand: {command}\nOutput:\n{stdout}"
        );
    }
}

#[test]
fn version_output_format_matches_pipenv() {
    // pipenv: "pipenv, version 2026.0.3"
    // ripenv: "ripenv 0.1.0"
    // Both should start with the binary name and contain a version.
    let mut cmd = ripenv_command();
    cmd.arg("--version");

    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    assert!(
        stdout.starts_with("ripenv "),
        "Version output should start with 'ripenv ', got: {stdout}"
    );
    // Must contain a semver-like version
    assert!(
        stdout.contains('.'),
        "Version output should contain a version number, got: {stdout}"
    );
}

// ─── Global flags parity ─────────────────────────────────────────────────────

#[test]
fn quiet_flag_parity() {
    // pipenv: -q / --quiet suppresses output
    // ripenv: same behavior
    for flag in ["-q", "--quiet"] {
        let mut cmd = ripenv_command();
        cmd.args([flag, "install"]);

        let output = cmd.output().expect("Failed to execute ripenv");
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            stderr.is_empty(),
            "{flag} should suppress stderr, got: {stderr}"
        );
    }
}

#[test]
fn verbose_flag_parity() {
    // pipenv: -v / --verbose increases output
    // ripenv: same flags accepted
    for flag in ["-v", "--verbose"] {
        let mut cmd = ripenv_command();
        cmd.args([flag, "install"]);

        let output = cmd.output().expect("Failed to execute ripenv");

        assert_eq!(
            output.status.code(),
            Some(1),
            "{flag} should be accepted (exit 1 = stub, not 2 = parse error)"
        );
    }
}

// ─── install subcommand flag parity ──────────────────────────────────────────

#[test]
fn install_help_has_pipenv_compatible_flags() {
    // Flags present in pipenv install that ripenv must also support.
    let required_flags = [
        "--system",
        "--deploy",
        "--skip-lock",
        "--editable",
        "--requirements",
        "--index",
        "--dev-packages", // pipenv uses -d/--dev; ripenv uses -d/--dev-packages
        "--pre",
        "--no-dev", // ripenv's equivalent of pipenv's default dev behavior
    ];

    let mut cmd = ripenv_command();
    cmd.args(["install", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for flag in required_flags {
        assert!(
            stdout.contains(flag),
            "ripenv install --help missing flag: {flag}\nOutput:\n{stdout}"
        );
    }
}

// ─── uninstall subcommand flag parity ────────────────────────────────────────

#[test]
fn uninstall_help_has_pipenv_compatible_flags() {
    let required_flags = ["--all", "--all-dev", "--skip-lock"];

    let mut cmd = ripenv_command();
    cmd.args(["uninstall", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for flag in required_flags {
        assert!(
            stdout.contains(flag),
            "ripenv uninstall --help missing flag: {flag}\nOutput:\n{stdout}"
        );
    }
}

// ─── lock subcommand flag parity ─────────────────────────────────────────────

#[test]
fn lock_help_has_pipenv_compatible_flags() {
    let required_flags = ["--pre", "--clear"];

    let mut cmd = ripenv_command();
    cmd.args(["lock", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for flag in required_flags {
        assert!(
            stdout.contains(flag),
            "ripenv lock --help missing flag: {flag}\nOutput:\n{stdout}"
        );
    }
}

// ─── update/upgrade parity ───────────────────────────────────────────────────

#[test]
fn upgrade_is_alias_for_update() {
    // pipenv has both `update` and `upgrade` as separate commands;
    // ripenv has `upgrade` as an alias for `update`.
    let mut cmd = ripenv_command();
    cmd.args(["upgrade", "--help"]);

    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains("Update packages"),
        "upgrade --help should show update's help text, got: {stdout}"
    );
}

#[test]
fn update_help_has_pipenv_compatible_flags() {
    let required_flags = ["--dry-run", "--lock-only"];

    let mut cmd = ripenv_command();
    cmd.args(["update", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for flag in required_flags {
        assert!(
            stdout.contains(flag),
            "ripenv update --help missing flag: {flag}\nOutput:\n{stdout}"
        );
    }
}

// ─── graph subcommand flag parity ────────────────────────────────────────────

#[test]
fn graph_help_has_pipenv_compatible_flags() {
    let required_flags = ["--bare", "--json", "--reverse"];

    let mut cmd = ripenv_command();
    cmd.args(["graph", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for flag in required_flags {
        assert!(
            stdout.contains(flag),
            "ripenv graph --help missing flag: {flag}\nOutput:\n{stdout}"
        );
    }
}

// ─── requirements subcommand flag parity ─────────────────────────────────────

#[test]
fn requirements_help_has_pipenv_compatible_flags() {
    let required_flags = ["--dev", "--dev-only", "--hash"];

    let mut cmd = ripenv_command();
    cmd.args(["requirements", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for flag in required_flags {
        assert!(
            stdout.contains(flag),
            "ripenv requirements --help missing flag: {flag}\nOutput:\n{stdout}"
        );
    }
}

// ─── check deprecation parity ────────────────────────────────────────────────

#[test]
fn check_shows_deprecation_like_pipenv() {
    // pipenv: "DEPRECATION WARNING: The 'check' command ... is deprecated"
    // ripenv: should also show deprecation warning pointing to audit
    let mut cmd = ripenv_command();
    cmd.arg("check");

    let output = cmd.output().expect("Failed to execute ripenv");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("deprecated") || stderr.contains("Deprecated"),
        "check should show deprecation warning, got: {stderr}"
    );
    assert!(
        stderr.contains("audit"),
        "check deprecation should mention 'audit', got: {stderr}"
    );
}

// ─── clean subcommand flag parity ────────────────────────────────────────────

#[test]
fn clean_help_has_dry_run_flag() {
    let mut cmd = ripenv_command();
    cmd.args(["clean", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("--dry-run"),
        "ripenv clean --help missing --dry-run flag\nOutput:\n{stdout}"
    );
}

// ─── run subcommand flag parity ──────────────────────────────────────────────

#[test]
fn run_help_has_pipenv_compatible_structure() {
    let mut cmd = ripenv_command();
    cmd.args(["run", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains("command") || stdout.contains("COMMAND"),
        "run --help should mention a command argument, got: {stdout}"
    );
}

// ─── sync subcommand flag parity ─────────────────────────────────────────────

#[test]
fn sync_help_has_pipenv_compatible_flags() {
    let required_flags = ["--no-dev", "--system"];

    let mut cmd = ripenv_command();
    cmd.args(["sync", "--help"]);
    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for flag in required_flags {
        assert!(
            stdout.contains(flag),
            "ripenv sync --help missing flag: {flag}\nOutput:\n{stdout}"
        );
    }
}

// ─── Exit code parity ───────────────────────────────────────────────────────

#[test]
fn unknown_command_exits_with_code_2() {
    // pipenv exits with code 2 for unknown commands (click convention).
    // ripenv should match (clap also uses exit code 2 for parse errors).
    let mut cmd = ripenv_command();
    cmd.arg("nonexistent");

    let output = cmd.output().expect("Failed to execute ripenv");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Unknown command should exit with code 2 (parse error)"
    );
}

#[test]
fn no_args_exits_with_code_2() {
    // Both pipenv and ripenv should exit with code 2 when no subcommand is given.
    let mut cmd = ripenv_command();

    let output = cmd.output().expect("Failed to execute ripenv");

    assert_eq!(
        output.status.code(),
        Some(2),
        "No args should exit with code 2 (missing subcommand)"
    );
}
