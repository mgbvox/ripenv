//! Integration tests for ripenv.
//!
//! Following the single-integration-test pattern from:
//! <https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html>

pub(crate) mod common;

mod help;
mod install;
mod lockfile;
mod parity;
mod pipfile_parse;
mod verbosity;
mod version;
