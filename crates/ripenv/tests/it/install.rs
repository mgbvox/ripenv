//! Integration tests for `ripenv install` behavior.

use crate::common::ripenv_command;

/// `ripenv install` in a directory with no Pipfile should auto-create one.
#[test]
fn install_creates_pipfile_when_missing() {
    let dir = tempfile::TempDir::new().unwrap();

    let mut cmd = ripenv_command();
    cmd.current_dir(dir.path());
    cmd.arg("install");

    let output = cmd.output().expect("Failed to execute ripenv");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The Pipfile should now exist
    let pipfile_path = dir.path().join("Pipfile");
    assert!(pipfile_path.is_file(), "Pipfile should be created");

    // Should contain the default PyPI source
    let content = fs_err::read_to_string(&pipfile_path).unwrap();
    assert!(content.contains("[packages]"));
    assert!(content.contains("[[source]]"));
    assert!(content.contains("pypi.org/simple"));

    // The stderr should mention creating the Pipfile
    assert!(
        stderr.contains("Created new Pipfile"),
        "Expected 'Created new Pipfile' in stderr, got: {stderr}"
    );
}

/// `ripenv install <package>` in a directory with no Pipfile should create one
/// and add the package to it.
#[test]
fn install_package_creates_pipfile_and_adds_package() {
    let dir = tempfile::TempDir::new().unwrap();

    let mut cmd = ripenv_command();
    cmd.current_dir(dir.path());
    cmd.args(["install", "--skip-lock", "requests"]);

    let output = cmd.output().expect("Failed to execute ripenv");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The Pipfile should exist with the package
    let pipfile_path = dir.path().join("Pipfile");
    assert!(pipfile_path.is_file(), "Pipfile should be created");

    let content = fs_err::read_to_string(&pipfile_path).unwrap();
    assert!(
        content.contains("requests"),
        "Pipfile should contain 'requests', got: {content}"
    );

    // Should mention creating the Pipfile
    assert!(
        stderr.contains("Created new Pipfile"),
        "Expected 'Created new Pipfile' in stderr, got: {stderr}"
    );
}

/// Other commands (e.g., `lock`) should NOT auto-create a Pipfile.
#[test]
fn lock_does_not_create_pipfile() {
    let dir = tempfile::TempDir::new().unwrap();

    let mut cmd = ripenv_command();
    cmd.current_dir(dir.path());
    cmd.arg("lock");

    let output = cmd.output().expect("Failed to execute ripenv");

    // Should fail — no Pipfile
    assert!(!output.status.success());

    // Should NOT create a Pipfile
    let pipfile_path = dir.path().join("Pipfile");
    assert!(
        !pipfile_path.is_file(),
        "Pipfile should NOT be created by lock"
    );
}
