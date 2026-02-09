# ripenv -- TODO

A thin Rust wrapper around uv that provides a pipenv-compatible CLI. All work is organized into
phases ordered by dependency and priority.

Legend: **S** = small (< 1 day), **M** = medium (1--3 days), **L** = large (3--5 days), **XL** =
extra-large (5+ days)

## Non-Goals for MVP

- **Pipfile.lock write support** -- MVP uses `uv.lock` as the lockfile. Pipfile.lock compatibility
  is a follow-on phase (see Phase 5).
- **Full `PIPENV_*` env-var parity** -- only 8 critical variables are supported initially.
- **Custom categories** -- only `packages` / `dev-packages`; custom categories are post-MVP.
- **PEP 751 / `pylock`** -- deferred until spec stabilizes.
- **`ripenv open`** -- low usage; deferred.
- **`--three` / `--two` legacy flags** -- Python 2 is EOL; not worth implementing.
- **Pipenv-style output formatting** -- ripenv uses uv's output conventions.
- **Compatibility test suite against pipenv** -- useful but post-MVP.

---

## Phase 0: Crate Setup

Bootstrap the `crates/ripenv/` crate inside the uv workspace.

- [x] **Create `crates/ripenv/Cargo.toml`** -- S
  - Dependencies: none
  - Reuse: copy structure from `crates/uv/Cargo.toml`; use `{ workspace = true }` for all shared
    deps
  - New code: Cargo.toml manifest only
  - Notes: workspace uses `members = ["crates/*"]` so the crate auto-registers; set edition = 2024,
    use workspace version (0.0.20)

- [x] **Create `crates/ripenv/src/bin/ripenv.rs`** -- S
  - Dependencies: Cargo.toml exists
  - Reuse: mirror `crates/uv/src/bin/uv.rs` pattern (thin wrapper that calls `lib::main()`)
  - New code: entry point only

- [x] **Create `crates/ripenv/src/lib.rs` with main()** -- S
  - Dependencies: bin/ripenv.rs exists
  - Reuse: pattern from `crates/uv/src/lib.rs`
  - New code: tokio runtime setup, top-level error handling

- [x] **Define clap CLI skeleton with all subcommands** -- M
  - Dependencies: lib.rs exists
  - Reuse: `uv-cli` patterns for clap derive structs
  - New code: `RipenvCommand` enum with variants: install, uninstall, lock, sync, update, run,
    shell, graph, requirements, clean, scripts, verify, check, audit
  - Notes: `upgrade` is `.alias("upgrade")` on `update`, not a separate variant

- [x] **Set up integration test harness** -- M
  - Dependencies: CLI skeleton compiles
  - Reuse: `crates/uv/tests/` patterns and `uv-test` utilities
  - New code: test helpers for creating temp directories with Pipfiles, running ripenv, asserting
    output
  - Notes: place tests at `crates/ripenv/tests/` or `it/` per project convention

- [ ] **CI configuration** -- S
  - Dependencies: crate compiles
  - Reuse: existing workspace CI; ripenv compiles as part of `cargo build`
  - New code: any ripenv-specific test jobs

---

## Phase 0.II: Hardening & Phase 1 Readiness

Improvements identified during code review to solidify the skeleton before Phase 1.

- [ ] **Add `--verbose` / `--quiet` integration tests** -- S
  - Test that `--quiet` suppresses warnings, `--verbose` enables debug output
  - Test that errors are always printed even with `--quiet`

- [ ] **Add Pipfile test fixtures directory** -- S
  - Create `crates/ripenv/tests/fixtures/` with sample Pipfiles for Phase 1 integration tests
  - Include: minimal Pipfile, Pipfile with dev deps, Pipfile with multiple sources, Pipfile with
    VCS/editable deps

- [ ] **Add `ripenv version` integration test** -- S
  - Verify `ripenv --version` outputs the expected version string

- [ ] **Add `Pipfile` module stub** -- S
  - Create empty `crates/ripenv/src/pipfile/mod.rs` module with data model structs placeholder
  - Wire into `lib.rs` so Phase 1 can begin immediately

- [ ] **Windows cross-compilation check** -- S
  - Run `cargo xwin clippy -p ripenv` to verify no platform-specific issues
  - Fix any path-handling issues before they compound in later phases

---

## Phase 1: Pipfile Bridge (CRITICAL PATH)

The core insight: instead of reimplementing uv's project commands, we construct a **virtual
`PyProjectToml`** from a Pipfile and feed it to uv's existing project machinery. The central
function is:

```rust
fn pipfile_to_project(pipfile: &Pipfile) -> Result<(PyProjectToml, Vec<Source>)>
```

This maps:

- `[packages]` -> `[project.dependencies]`
- `[dev-packages]` -> `[dependency-groups.dev]`
- `[[source]]` -> `[[tool.uv.index]]`
- `[requires] python_version` -> `requires-python`

### 1A. Pipfile Parser & Virtual PyProjectToml Bridge

- [ ] **Define Pipfile data model structs** -- M
  - Dependencies: Phase 0 complete
  - Reuse: `uv-pep508` (dependency specifiers), `uv-pep440` (version specifiers), `uv-normalize`
    (package names)
  - New code: Rust structs mirroring Pipfile TOML schema:
    - `Pipfile` (top-level)
    - `PipfileSource` (name, url, verify_ssl)
    - `PipfilePackage` (version, extras, markers, git/path/editable, index)
    - `PipfileScripts` (name -> command mapping)
    - `PipfileRequires` (python_version, python_full_version)
  - Notes: use `serde` + `toml` for deserialization; packages can be a version string `"*"` or a
    table `{version = ">=1.0", extras = ["security"]}`

- [ ] **Implement Pipfile reader (TOML -> structs)** -- M
  - Dependencies: data model structs
  - Reuse: `toml` crate (already in workspace)
  - New code: `Pipfile::from_path(path: &Path) -> Result<Pipfile>`, handle both string and table
    package specs
  - Tests: parse real-world Pipfile examples; round-trip fidelity

- [ ] **Implement Pipfile writer (structs -> TOML)** -- M
  - Dependencies: data model structs
  - Reuse: `toml` crate serialization
  - New code: `Pipfile::write(&self, path: &Path) -> Result<()>`; preserve section ordering
  - Notes: needed for `ripenv install <pkg>` which modifies Pipfile

- [ ] **Implement `pipfile_to_project` bridge** -- L
  - Dependencies: Pipfile reader works
  - Reuse: `uv-workspace::PyProjectToml`, `uv-workspace::Project`, `uv-workspace::ToolUv`,
    `uv-distribution-types::Index`
  - New code: conversion function that builds a virtual `PyProjectToml`:
    - `PipfilePackage` -> `uv_pep508::Requirement` (handle `"*"`, exact, range, VCS, path, editable,
      extras, markers, index pinning)
    - `PipfileSource` -> `uv_distribution_types::Index` (name, url, verify_ssl -> trust-host)
    - `[requires] python_version` -> `RequiresPython`
    - Assemble into `PyProjectToml` with `Project { dependencies, dependency_groups }` and
      `ToolUv { index, sources }`
  - Tests: cover all pipenv dep spec variants; verify uv accepts the virtual pyproject

- [ ] **Pipfile discovery (walk up directories)** -- S
  - Dependencies: Phase 0
  - Reuse: `uv-workspace` project discovery patterns
  - New code: walk from CWD upward looking for `Pipfile`; respect `PIPENV_MAX_DEPTH` and
    `PIPENV_PIPFILE`

---

## Phase 2: Core Commands

Each command reads the Pipfile, builds a virtual `PyProjectToml` via the bridge, then delegates to
the corresponding uv project command. **uv.lock is the lockfile.**

### install

- [ ] **`ripenv install` (no args) -- sync from lock** -- M
  - Dependencies: Pipfile bridge
  - Reuse: `uv sync` logic (`crates/uv/src/commands/project/sync.rs`)
  - New code: build virtual pyproject, pass to uv sync
  - Flags: `--dev` (include dev deps, default true), `--system`, `--deploy` (fail if lock is stale)

- [ ] **`ripenv install <PACKAGES>` -- add packages** -- M
  - Dependencies: Pipfile reader/writer, bridge
  - Reuse: `uv add` logic (`crates/uv/src/commands/project/add.rs`), `uv lock`, `uv sync`
  - New code: parse package specs, add to Pipfile `[packages]` or `[dev-packages]`, build virtual
    pyproject, delegate to uv add+lock+sync
  - Flags: `--dev`, `--editable`, `--pre`, `--skip-lock`, `--index`

- [ ] **`ripenv install -r requirements.txt`** -- S
  - Dependencies: install command
  - Reuse: `uv-requirements` (requirements.txt parser)
  - New code: parse requirements.txt, add each dep to Pipfile, delegate to install

### uninstall

- [ ] **`ripenv uninstall <PACKAGES>`** -- M
  - Dependencies: Pipfile reader/writer, bridge
  - Reuse: `uv remove` logic (`crates/uv/src/commands/project/remove.rs`)
  - New code: remove from Pipfile, rebuild virtual pyproject, delegate to uv remove+lock+sync
  - Flags: `--dev`, `--all`, `--all-dev`, `--skip-lock`

### lock

- [ ] **`ripenv lock`** -- M
  - Dependencies: Pipfile bridge
  - Reuse: `uv lock` logic (`crates/uv/src/commands/project/lock.rs`), `uv-resolver`
  - New code: build virtual pyproject, delegate to uv lock -> produces uv.lock
  - Flags: `--dev-only`, `--pre`, `--clear`

### sync

- [ ] **`ripenv sync`** -- M
  - Dependencies: Pipfile bridge
  - Reuse: `uv sync` logic, `uv-installer`
  - New code: build virtual pyproject, delegate to uv sync from uv.lock
  - Flags: `--dev`, `--system`

### update

- [ ] **`ripenv update [PACKAGES]`** -- M
  - Dependencies: lock + sync commands
  - Reuse: composes lock + sync
  - New code: orchestration; if packages specified, only update those (pass constraints to resolver)
  - Flags: `--dry-run`, `--dev`, `--lock-only`
  - Notes: `upgrade` is a clap alias for `update`

### run

- [ ] **`ripenv run <COMMAND>`** -- M
  - Dependencies: Phase 0, Pipfile reader (for `[scripts]`)
  - Reuse: `uv run` logic (`crates/uv/src/commands/project/run.rs`)
  - New code:
    - Look up command in Pipfile `[scripts]` section first
    - If not found, pass through as raw command
    - Activate virtualenv in subprocess environment
  - Flags: `--system`

---

## Phase 3: Environment & Configuration

- [ ] **Virtual environment management** -- M
  - Dependencies: Phase 0
  - Reuse: `uv-virtualenv`, `uv-python`
  - New code: venv location strategy:
    - Default: `.venv/` in project dir (matches uv convention)
    - `PIPENV_VENV_IN_PROJECT=0` or `WORKON_HOME` -> `~/.local/share/virtualenvs/<project>-<hash>/`
      (pipenv-style)

- [ ] **`PIPENV_*` environment variable support (core 8)** -- M
  - Dependencies: Phase 0
  - New code: read and honor these critical env vars:
    - `PIPENV_VENV_IN_PROJECT` -- venv in `.venv/` (default) or hash-based
    - `PIPENV_CACHE_DIR` -- cache location
    - `PIPENV_YES` -- assume yes
    - `PIPENV_PYTHON` -- python interpreter path
    - `PIPENV_DEFAULT_PYTHON_VERSION` -- default python version
    - `PIPENV_SKIP_LOCK` -- skip locking
    - `PIPENV_PIPFILE` -- custom Pipfile path
    - `PIPENV_PYPI_MIRROR` -- PyPI mirror URL

- [ ] **`.env` file loading** -- S
  - Dependencies: env var support
  - Reuse: `dotenvy` crate (or similar)
  - New code: auto-load `.env` from project root before command execution; respect
    `PIPENV_DOTENV_LOCATION`

---

## Phase 4: Secondary Commands (Post-MVP)

These commands round out pipenv compatibility but are not needed for the initial release.

### shell

- [ ] **`ripenv shell`** -- M
  - Dependencies: virtualenv location logic
  - Reuse: `uv-virtualenv` for venv path discovery
  - New code: spawn a new shell with virtualenv activated; detect shell type (bash/zsh/fish)

### graph

- [ ] **`ripenv graph`** -- M
  - Dependencies: uv.lock reader
  - Reuse: `uv tree` logic (`crates/uv/src/commands/project/tree.rs`)
  - New code: format output in pipenv style (tree with arrows)
  - Flags: `--bare`, `--json`, `--reverse`

### requirements

- [ ] **`ripenv requirements`** -- M
  - Dependencies: Pipfile bridge
  - Reuse: `uv export` logic (`crates/uv/src/commands/project/export.rs`)
  - New code: emit `requirements.txt` from uv.lock
  - Flags: `--dev`, `--dev-only`, `--hash`

### clean

- [ ] **`ripenv clean`** -- S
  - Dependencies: installed package inspection
  - Reuse: `uv-installer` for listing installed packages
  - New code: diff installed packages vs locked packages, remove unlisted ones

### scripts

- [ ] **`ripenv scripts`** -- S
  - Dependencies: Pipfile reader
  - New code: read `[scripts]` from Pipfile, print name -> command table

### verify

- [ ] **`ripenv verify`** -- S
  - Dependencies: uv.lock exists
  - New code: check that uv.lock is up to date with virtual pyproject; exit 0/1

### check / audit

- [ ] **`ripenv check`** -- S
  - Dependencies: none
  - New code: print deprecation warning pointing to `ripenv audit`

- [ ] **`ripenv audit`** -- S
  - Dependencies: uv.lock reader
  - New code: shell out to `pip-audit` with packages from uv.lock
  - Notes: same strategy as pipenv; avoids reimplementing vulnerability DB queries

---

## Phase 5: Pipfile.lock Compatibility (Follow-on)

For users who need interoperability with pipenv, this phase adds Pipfile.lock read/write support
alongside uv.lock.

- [ ] **Define Pipfile.lock data model structs** -- M
  - New code: Rust structs for Pipfile.lock JSON schema:
    - `PipfileLock` (top-level: `_meta`, `default`, `develop`)
    - `PipfileLockMeta` (hash, pipfile-spec, requires, sources)
    - `PipfileLockPackage` (hashes, index, version, markers, extras, editable, git_url, path,
      dependencies)

- [ ] **Implement Pipfile.lock reader (JSON -> structs)** -- M
  - Reuse: `serde_json` (already in workspace)
  - New code: `PipfileLock::from_path(path: &Path) -> Result<PipfileLock>`
  - Tests: parse real Pipfile.lock files

- [ ] **Implement Pipfile.lock writer (structs -> JSON)** -- L
  - New code: `PipfileLock::write(&self, path: &Path) -> Result<()>`
    - Generate `_meta.hash` from Pipfile content (SHA256 of Pipfile as JSON-normalized)
    - Populate `_meta.sources` from Pipfile sources
    - Map resolved packages into `default` / `develop` sections
    - Include hashes for each package
    - Deterministic output (sorted keys) for reproducibility
  - Notes: must produce a Pipfile.lock that `pipenv install --deploy` would accept

- [ ] **Map uv resolution output -> Pipfile.lock packages** -- XL
  - Reuse: `uv-resolver` resolution graph, `uv-distribution-types`
  - New code: walk uv's resolution result and emit `PipfileLockPackage` entries with correct
    version, hashes, dependency list, and default/develop categorization

- [ ] **Pipfile.lock hash verification** -- S
  - New code: compute Pipfile content hash, compare to `_meta.hash.sha256` in lock

- [ ] **`--pipfile-lock` flag on core commands** -- M
  - New code: add `--pipfile-lock` flag to install/lock/sync/verify to opt into Pipfile.lock format
    instead of uv.lock

---

## Phase 6: Polish (Post-MVP)

- [ ] **Shell completions** -- S
  - Reuse: clap's built-in completion generation
  - New code: generate completions for bash, zsh, fish

- [ ] **`--python <VERSION>` global flag** -- S
  - Reuse: `uv-python` for python discovery and installation
  - New code: `ripenv --python 3.12 install` creates venv with specified python

- [ ] **Categories support** -- M
  - New code: pipenv supports custom categories beyond `packages`/`dev-packages`; handle
    `--categories` flag across commands

- [ ] **`ripenv activate`** -- S
  - New code: print activation command for current shell (e.g., `source /path/to/venv/bin/activate`)
  - Notes: user evals the output: `eval $(ripenv activate)`

---

## Dependency Graph (Critical Path)

```
Phase 0: Crate Setup
    |
    v
Phase 1: Pipfile Parser + Virtual PyProjectToml Bridge
    |
    +---------------------------+
    |                           |
    v                           v
Phase 2: Core Commands     Phase 3: Env & Config
    |
    v
Phase 4: Secondary Commands (Post-MVP)
    |
    v
Phase 5: Pipfile.lock Compatibility (Follow-on)
    |
    v
Phase 6: Polish (Post-MVP)
```

## Key Technical Decisions

1. **Resolution strategy**: Translate Pipfile to a virtual `PyProjectToml` and reuse uv's resolver
   and project commands unmodified. This is the single most important architectural decision -- it
   means ripenv is a thin translation layer, not a fork.

2. **Categories**: Start with `default` + `develop` only. Custom categories are post-MVP.

3. **Virtual environment location**: Default to `.venv/` (matches uv convention) with opt-in
   pipenv-style hash-based location via `PIPENV_VENV_IN_PROJECT=0` or `WORKON_HOME`.

4. **Audit implementation**: Shell out to `pip-audit` (like pipenv does) rather than reimplementing
   vulnerability DB queries. Size: S instead of L.

5. **Categories support**: Only `default` + `develop` for MVP; custom categories post-MVP.

6. **Lockfile format**: ripenv uses `uv.lock` as its native lockfile. This eliminates the hardest
   piece of the project (Pipfile.lock write fidelity) from the critical path. Pipfile.lock
   compatibility is available as an opt-in follow-on (Phase 5).

## UV Crate Dependency Summary

| UV Crate                | Used For                                                |
| ----------------------- | ------------------------------------------------------- |
| `uv-cli`                | Clap patterns, argument types                           |
| `uv-resolver`           | Dependency resolution engine                            |
| `uv-installer`          | Package installation                                    |
| `uv-virtualenv`         | Virtual environment creation                            |
| `uv-python`             | Python version discovery and management                 |
| `uv-workspace`          | `PyProjectToml`, `Project`, `ToolUv`, project discovery |
| `uv-requirements`       | requirements.txt parsing                                |
| `uv-configuration`      | Index and settings types                                |
| `uv-pep440`             | Version specifier types                                 |
| `uv-pep508`             | Dependency specifier types                              |
| `uv-normalize`          | Package name normalization                              |
| `uv-distribution-types` | `Index`, `IndexLocations`, distribution types           |
| `uv-cache`              | Download and resolution caching                         |
| `uv-fs`                 | Filesystem utilities                                    |

## Estimated MVP Size

~1,600 lines of new Rust code (Phases 0--3), primarily:

- ~400 lines: Pipfile data model + parser + writer
- ~300 lines: `pipfile_to_project` bridge
- ~500 lines: Core command dispatch (install, uninstall, lock, sync, update, run)
- ~200 lines: CLI skeleton, env vars, .env loading
- ~200 lines: Pipfile discovery, venv management

Timeline: ~2--3 weeks for a single developer familiar with the uv codebase.
