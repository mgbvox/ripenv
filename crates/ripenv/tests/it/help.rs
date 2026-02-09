use crate::common::{INSTA_FILTERS, ripenv_help};
use crate::ripenv_snapshot;

#[test]
fn help_shows_all_commands() {
    ripenv_snapshot!(&INSTA_FILTERS, ripenv_help(), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    A pipenv-compatible CLI powered by uv.

    Usage: ripenv [OPTIONS] <COMMAND>

    Commands:
      install       Install packages from the lockfile, or add new packages to the Pipfile
      uninstall     Remove packages from the Pipfile and virtualenv
      lock          Generate or update the lockfile (uv.lock) from the Pipfile
      sync          Sync the virtualenv with the lockfile
      update        Update packages (re-lock then sync)
      run           Run a command in the virtualenv, or a Pipfile script
      shell         Spawn a shell with the virtualenv activated
      graph         Display the dependency tree
      requirements  Export locked dependencies as requirements.txt
      clean         Remove packages not in the lockfile from the virtualenv
      scripts       List scripts defined in the Pipfile
      verify        Verify the lockfile is up to date with the Pipfile
      check         Deprecated: use `ripenv audit` instead
      audit         Audit installed packages for known vulnerabilities
      help          Print this message or the help of the given subcommand(s)

    Options:
      -v, --verbose...  Increase logging verbosity
      -q, --quiet       Suppress all output
      -h, --help        Print help
      -V, --version     Print version

    Use `ripenv help <command>` for more information on a specific command.
    ----- stderr -----
    "#);
}

#[test]
fn help_install() {
    let mut cmd = crate::common::ripenv_command();
    cmd.args(["help", "install"]);

    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Install packages"));
    assert!(stdout.contains("--no-dev"));
    assert!(stdout.contains("--deploy"));
}

#[test]
fn upgrade_alias_works() {
    let mut cmd = crate::common::ripenv_command();
    cmd.arg("upgrade");
    cmd.arg("--help");

    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Update packages"));
}

#[test]
fn stub_commands_return_failure() {
    // These commands are still stubs (Phase 3+).
    // Implemented commands (install, lock, sync, etc.) now fail with exit 2
    // when no Pipfile is present, which is correct behavior.
    for subcommand in ["audit", "verify", "scripts"] {
        let mut cmd = crate::common::ripenv_command();
        cmd.arg(subcommand);

        let output = cmd.output().expect("Failed to execute ripenv");

        assert_eq!(
            output.status.code(),
            Some(1),
            "{subcommand} should return exit code 1 (not yet implemented)"
        );
    }
}

#[test]
fn implemented_commands_fail_without_pipfile() {
    // Implemented commands require a Pipfile; without one they exit with code 2.
    for subcommand in ["install", "lock", "sync", "update", "uninstall"] {
        let mut cmd = crate::common::ripenv_command();
        cmd.arg(subcommand);

        let output = cmd.output().expect("Failed to execute ripenv");

        assert_eq!(
            output.status.code(),
            Some(2),
            "{subcommand} should return exit code 2 (no Pipfile found)"
        );
    }
}

#[test]
fn check_shows_deprecation_warning() {
    let mut cmd = crate::common::ripenv_command();
    cmd.arg("check");

    let output = cmd.output().expect("Failed to execute ripenv");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("deprecated"));
    assert!(stderr.contains("ripenv audit"));
}

#[test]
fn unknown_command_errors() {
    let mut cmd = crate::common::ripenv_command();
    cmd.arg("nonexistent");

    let output = cmd.output().expect("Failed to execute ripenv");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn no_args_shows_help() {
    let mut cmd = crate::common::ripenv_command();

    let output = cmd.output().expect("Failed to execute ripenv");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // clap errors with "requires a subcommand" when no subcommand given
    assert!(!output.status.success());
    assert!(
        stderr.contains("Usage") || stderr.contains("subcommand"),
        "Expected usage info in stderr, got: {stderr}"
    );
}
