//! Pipfile parsing and data model for ripenv.
//!
//! This module provides Rust types that mirror the Pipfile TOML schema and
//! conversion logic to transform a `Pipfile` into uv's `PyProjectToml`.
//!
//! ## Architecture
//!
//! The central function is [`pipfile_to_project`] (Phase 1), which maps:
//!
//! - `[packages]` -> `[project.dependencies]`
//! - `[dev-packages]` -> `[dependency-groups.dev]`
//! - `[[source]]` -> `[[tool.uv.index]]`
//! - `[requires] python_version` -> `requires-python`
//!
//! This allows ripenv to reuse uv's project machinery unmodified.

pub mod model;

pub use model::Pipfile;
