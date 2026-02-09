use crate::common::ripenv_command;

#[test]
fn quiet_suppresses_stub_warning() {
    let mut cmd = ripenv_command();
    cmd.args(["--quiet", "install"]);

    let output = cmd.output().expect("Failed to execute ripenv");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.is_empty(),
        "Expected no output with --quiet, got: {stderr}"
    );
}

#[test]
fn quiet_suppresses_check_deprecation() {
    let mut cmd = ripenv_command();
    cmd.args(["--quiet", "check"]);

    let output = cmd.output().expect("Failed to execute ripenv");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.is_empty(),
        "Expected no output with --quiet, got: {stderr}"
    );
}

#[test]
fn verbose_flag_accepted() {
    let mut cmd = ripenv_command();
    cmd.args(["--verbose", "install"]);

    let output = cmd.output().expect("Failed to execute ripenv");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Command still fails (stub) but -v is accepted without error
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.contains("not yet implemented"),
        "Expected stub warning with --verbose, got: {stderr}"
    );
}

#[test]
fn double_verbose_accepted() {
    let mut cmd = ripenv_command();
    cmd.args(["-vv", "install"]);

    let output = cmd.output().expect("Failed to execute ripenv");

    // -vv is accepted without error
    assert_eq!(output.status.code(), Some(1));
}
