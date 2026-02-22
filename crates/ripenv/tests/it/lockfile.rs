//! Integration tests for `Pipfile.lock` generation.

use std::path::{Path, PathBuf};

use crate::common::ripenv_command;

/// Create a project directory inside a temp dir with a valid Python package name.
///
/// Temp dirs often start with `.tmp` which is not a valid package name,
/// so we create a subdirectory with a clean name.
fn project_dir(tmp: &tempfile::TempDir) -> PathBuf {
    let dir = tmp.path().join("test-project");
    fs_err::create_dir_all(&dir).unwrap();
    dir
}

/// Write a Pipfile to a directory.
fn write_pipfile(dir: &Path, content: &str) {
    fs_err::write(dir.join("Pipfile"), content).unwrap();
}

/// Parse the generated `Pipfile.lock` from a directory.
fn read_pipfile_lock(dir: &Path) -> serde_json::Value {
    let content = fs_err::read_to_string(dir.join("Pipfile.lock")).unwrap();
    serde_json::from_str(&content).unwrap()
}

const MINIMAL_PIPFILE: &str = r#"[[source]]
url = "https://pypi.org/simple"
verify_ssl = true
name = "pypi"

[packages]
six = "==1.17.0"

[dev-packages]

[requires]
python_version = "3.12"
"#;

const DEV_PIPFILE: &str = r#"[[source]]
url = "https://pypi.org/simple"
verify_ssl = true
name = "pypi"

[packages]
six = "==1.17.0"

[dev-packages]
iniconfig = "==2.1.0"

[requires]
python_version = "3.12"
"#;

/// `ripenv lock` should generate a `Pipfile.lock` alongside `uv.lock`.
#[test]
fn lock_generates_pipfile_lock() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = project_dir(&tmp);
    write_pipfile(&dir, MINIMAL_PIPFILE);

    let output = ripenv_command()
        .current_dir(&dir)
        .arg("lock")
        .output()
        .expect("Failed to execute ripenv");

    assert!(
        output.status.success(),
        "ripenv lock failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Both lockfiles should exist
    assert!(dir.join("uv.lock").is_file(), "uv.lock missing");
    assert!(dir.join("Pipfile.lock").is_file(), "Pipfile.lock missing");

    // Pipfile.lock should be valid JSON with expected top-level keys
    let lock = read_pipfile_lock(&dir);
    assert!(lock.get("_meta").is_some(), "missing _meta key");
    assert!(lock.get("default").is_some(), "missing default key");
    assert!(lock.get("develop").is_some(), "missing develop key");
}

/// `_meta` should have correct structure: hash, pipfile-spec, requires, sources.
#[test]
fn pipfile_lock_has_correct_meta() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = project_dir(&tmp);
    write_pipfile(&dir, MINIMAL_PIPFILE);

    let output = ripenv_command()
        .current_dir(&dir)
        .arg("lock")
        .output()
        .expect("Failed to execute ripenv");

    assert!(
        output.status.success(),
        "ripenv lock failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lock = read_pipfile_lock(&dir);
    let meta = lock.get("_meta").unwrap();

    // pipfile-spec should be 6
    assert_eq!(meta.get("pipfile-spec").unwrap(), 6);

    // hash should be a non-empty SHA256
    let hash = meta
        .get("hash")
        .unwrap()
        .get("sha256")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(hash.len(), 64, "SHA256 hex should be 64 chars: {hash}");

    // sources should contain pypi
    let sources = meta.get("sources").unwrap().as_array().unwrap();
    assert!(!sources.is_empty(), "sources should not be empty");
    assert_eq!(sources[0].get("name").unwrap(), "pypi");
    assert_eq!(sources[0].get("url").unwrap(), "https://pypi.org/simple");

    // requires should mention python_version
    let requires = meta.get("requires").unwrap();
    assert_eq!(requires.get("python_version").unwrap(), "3.12");
}

/// Packages in `[packages]` should appear in `default`; packages only
/// reachable from `[dev-packages]` should appear in `develop`.
#[test]
fn pipfile_lock_default_develop_split() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = project_dir(&tmp);
    write_pipfile(&dir, DEV_PIPFILE);

    let output = ripenv_command()
        .current_dir(&dir)
        .arg("lock")
        .output()
        .expect("Failed to execute ripenv");

    assert!(
        output.status.success(),
        "ripenv lock failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lock = read_pipfile_lock(&dir);
    let default = lock.get("default").unwrap().as_object().unwrap();
    let develop = lock.get("develop").unwrap().as_object().unwrap();

    // six should be in default
    assert!(default.contains_key("six"), "six should be in default");

    // iniconfig should be in develop (it has no deps that overlap with default)
    assert!(
        develop.contains_key("iniconfig"),
        "iniconfig should be in develop, got: {develop:?}"
    );

    // iniconfig should NOT be in default
    assert!(
        !default.contains_key("iniconfig"),
        "iniconfig should not be in default"
    );
}

/// Each locked package should have at least one SHA256 hash and a pinned version.
#[test]
fn pipfile_lock_packages_have_hashes_and_versions() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = project_dir(&tmp);
    write_pipfile(&dir, MINIMAL_PIPFILE);

    let output = ripenv_command()
        .current_dir(&dir)
        .arg("lock")
        .output()
        .expect("Failed to execute ripenv");

    assert!(
        output.status.success(),
        "ripenv lock failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lock = read_pipfile_lock(&dir);
    let default = lock.get("default").unwrap().as_object().unwrap();

    let six = default.get("six").expect("six should be in default");
    let hashes = six.get("hashes").unwrap().as_array().unwrap();
    assert!(!hashes.is_empty(), "six should have at least one hash");
    for hash in hashes {
        let hash_str = hash.as_str().unwrap();
        assert!(
            hash_str.starts_with("sha256:"),
            "hash should start with sha256: got {hash_str}"
        );
    }

    let version = six.get("version").unwrap().as_str().unwrap();
    assert!(
        version.starts_with("=="),
        "version should be pinned with ==, got: {version}"
    );
}

/// Locking twice should produce identical `Pipfile.lock` content.
#[test]
fn pipfile_lock_is_deterministic() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = project_dir(&tmp);
    write_pipfile(&dir, MINIMAL_PIPFILE);

    // First lock
    let output = ripenv_command()
        .current_dir(&dir)
        .arg("lock")
        .output()
        .expect("Failed to execute ripenv");

    assert!(
        output.status.success(),
        "first lock failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lock1 = fs_err::read_to_string(dir.join("Pipfile.lock")).unwrap();

    // Second lock
    let output = ripenv_command()
        .current_dir(&dir)
        .arg("lock")
        .output()
        .expect("Failed to execute ripenv");

    assert!(
        output.status.success(),
        "second lock failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lock2 = fs_err::read_to_string(dir.join("Pipfile.lock")).unwrap();

    assert_eq!(lock1, lock2, "Pipfile.lock should be identical across runs");
}

/// `ripenv install` (bare) should also generate `Pipfile.lock`.
#[test]
fn install_generates_pipfile_lock() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = project_dir(&tmp);
    write_pipfile(&dir, MINIMAL_PIPFILE);

    let output = ripenv_command()
        .current_dir(&dir)
        .arg("install")
        .output()
        .expect("Failed to execute ripenv");

    assert!(
        output.status.success(),
        "ripenv install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        dir.join("Pipfile.lock").is_file(),
        "Pipfile.lock should be generated by install"
    );

    let lock = read_pipfile_lock(&dir);
    assert!(lock.get("_meta").is_some());
    assert!(lock.get("default").is_some());
}
