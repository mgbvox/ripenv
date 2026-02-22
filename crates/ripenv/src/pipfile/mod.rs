//! Pipfile parsing and data model for ripenv.
//!
//! This module provides Rust types that mirror the Pipfile TOML schema and
//! conversion logic to transform a `Pipfile` into uv's `PyProjectToml`.
//!
//! ## Architecture
//!
//! The central function is [`bridge::pipfile_to_pyproject_toml`], which maps:
//!
//! - `[packages]` -> `[project.dependencies]`
//! - `[dev-packages]` -> `[dependency-groups.dev]`
//! - `[[source]]` -> `[[tool.uv.index]]`
//! - `[requires] python_version` -> `requires-python`
//!
//! This allows ripenv to reuse uv's project machinery unmodified.

pub mod bridge;
pub mod discovery;
pub mod lockfile;
pub mod model;
mod writer;

pub use bridge::pipfile_to_pyproject_toml;
pub use discovery::{find_pipfile, project_name_from_dir, project_root};
pub use model::Pipfile;
