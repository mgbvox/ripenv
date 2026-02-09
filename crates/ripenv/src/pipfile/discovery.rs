//! Pipfile discovery: walk up directories to find the nearest `Pipfile`.
//!
//! Respects `PIPENV_PIPFILE` (explicit path) and `PIPENV_MAX_DEPTH`
//! (maximum parent directories to traverse).

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

/// Default maximum directory traversal depth.
const DEFAULT_MAX_DEPTH: usize = 3;

/// The filename we're looking for.
const PIPFILE_NAME: &str = "Pipfile";

/// Discover the Pipfile by walking up from the given directory.
///
/// Resolution order:
/// 1. `PIPENV_PIPFILE` environment variable (explicit path)
/// 2. Walk up from `start_dir` looking for `Pipfile`, up to `PIPENV_MAX_DEPTH`
///    parent directories (default: 3).
pub fn find_pipfile(start_dir: &Path) -> Result<PathBuf> {
    // 1. Check PIPENV_PIPFILE env var
    if let Ok(explicit) = env::var("PIPENV_PIPFILE") {
        let path = PathBuf::from(&explicit);
        if path.is_file() {
            return Ok(path);
        }
        bail!("PIPENV_PIPFILE is set to '{explicit}' but the file does not exist");
    }

    // 2. Walk up directories
    let max_depth = env::var("PIPENV_MAX_DEPTH")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_DEPTH);

    let mut current = start_dir.to_path_buf();
    for _ in 0..=max_depth {
        let candidate = current.join(PIPFILE_NAME);
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !current.pop() {
            break;
        }
    }

    bail!(
        "No Pipfile found (searched up to {} parent directories from {})",
        max_depth,
        start_dir.display()
    );
}

/// Return the project root directory (parent of the Pipfile).
pub fn project_root(pipfile_path: &Path) -> Option<&Path> {
    pipfile_path.parent()
}

/// Derive a project name from the project root directory.
///
/// Falls back to `"project"` if the directory name can't be determined.
pub fn project_name_from_dir(project_dir: &Path) -> String {
    project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn find_pipfile_in_current_dir() {
        let dir = TempDir::new().unwrap();
        let pipfile = dir.path().join("Pipfile");
        fs_err::write(&pipfile, "# empty").unwrap();

        let found = find_pipfile(dir.path()).unwrap();
        assert_eq!(found, pipfile);
    }

    #[test]
    fn find_pipfile_in_parent_dir() {
        let dir = TempDir::new().unwrap();
        let pipfile = dir.path().join("Pipfile");
        fs_err::write(&pipfile, "# empty").unwrap();

        let subdir = dir.path().join("src");
        fs_err::create_dir(&subdir).unwrap();

        let found = find_pipfile(&subdir).unwrap();
        assert_eq!(found, pipfile);
    }

    #[test]
    fn no_pipfile_returns_error() {
        let dir = TempDir::new().unwrap();
        let result = find_pipfile(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No Pipfile found"));
    }

    #[test]
    fn project_name_from_directory() {
        let name = project_name_from_dir(Path::new("/home/user/my-project"));
        assert_eq!(name, "my-project");
    }

    #[test]
    fn project_name_fallback() {
        let name = project_name_from_dir(Path::new("/"));
        assert_eq!(name, "project");
    }
}
