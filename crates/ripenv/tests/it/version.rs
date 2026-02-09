use crate::common::ripenv_command;

#[test]
fn version_flag_shows_version() {
    let mut cmd = ripenv_command();
    cmd.arg("--version");

    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.starts_with("ripenv "),
        "Expected version string starting with 'ripenv ', got: {stdout}"
    );
}

#[test]
fn short_version_flag_works() {
    let mut cmd = ripenv_command();
    cmd.arg("-V");

    let output = cmd.output().expect("Failed to execute ripenv");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.starts_with("ripenv "),
        "Expected version string starting with 'ripenv ', got: {stdout}"
    );
}
