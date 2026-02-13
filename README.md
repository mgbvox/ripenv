# ripenv

A blazing-fast, pipenv-compatible CLI powered by [uv](https://github.com/astral-sh/uv).

ripenv is a drop-in replacement for [pipenv](https://pipenv.pypa.io/) that uses uv's resolver and
installer under the hood. It reads your existing `Pipfile`, translates it into uv's project model,
and calls uv's library functions directly — no subprocess overhead, no shelling out.

> **Note:** This repository is a fork of [astral-sh/uv](https://github.com/astral-sh/uv). The
> upstream uv crate is preserved in full; ripenv lives at `crates/ripenv/` as an additional
> workspace member.

## Why ripenv?

- **pipenv CLI, uv speed** — keep your `Pipfile` workflow with 10-100x faster resolution and
  installs
- **Zero migration** — point ripenv at your existing `Pipfile` and go
- **Direct library calls** — ripenv calls uv's Rust APIs directly, not via subprocess
- **Familiar commands** — `ripenv install`, `ripenv lock`, `ripenv sync`, `ripenv run`, etc.

## Installation

### From source (recommended during development)

```bash
# Clone the repo
git clone https://github.com/mgbvox/ripenv.git
cd ripenv

# Build release binary
cargo build --release -p ripenv

# The binary is at:
./target/release/ripenv --help
```

### Install to cargo bin

```bash
cargo install --path crates/ripenv
```

## Usage

ripenv reads your `Pipfile` and manages dependencies through uv.

```bash
# Install all dependencies (lock + sync)
ripenv install

# Add a package
ripenv install requests

# Add a dev dependency
ripenv install --dev pytest

# Remove a package
ripenv uninstall requests

# Generate/update the lockfile
ripenv lock

# Sync the virtualenv to match the lockfile
ripenv sync

# Run a command in the virtualenv
ripenv run python my_script.py

# Update all dependencies
ripenv update

# Update a specific package
ripenv update requests
```

### Environment variables

- `PIPENV_PIPFILE` — path to the Pipfile (default: auto-discovered in current/parent directories)

## Building

Requires Rust 1.91+ (edition 2024).

```bash
# Debug build (faster compile, slower runtime)
cargo build -p ripenv

# Release build (slower compile, optimized runtime)
cargo build --release -p ripenv

# Run clippy
cargo clippy -p ripenv

# Run tests
cargo test -p ripenv
```

## Benchmarking

A hyperfine-based benchmark suite lives in `scripts/benchmark-ripenv/` for comparing ripenv, pipenv,
and uv across lock, sync, and install operations.

### Prerequisites

- [hyperfine](https://github.com/sharkdp/hyperfine): `brew install hyperfine`
- pipenv: `pip install pipenv`
- A release build of ripenv: `cargo build --release -p ripenv`

### Quick start

```bash
cd scripts/benchmark-ripenv

# Run all benchmarks for the flask fixture (ripenv vs pipenv vs uv)
RIPENV_PATH=../../target/release/ripenv ./run-benchmarks.sh flask

# Results and plots are saved to scripts/benchmark-ripenv/results/flask/
```

### Detailed usage

```bash
# Run specific benchmarks
BENCHMARKS=lock-cold,lock-warm ./run-benchmarks.sh flask

# Run on multiple fixtures
./run-benchmarks.sh trio flask jupyter

# Custom binary paths
RIPENV_PATH=../../target/release/ripenv \
UV_PATH=../../target/release/uv \
  ./run-benchmarks.sh trio

# Use bench-ripenv directly for fine-grained control
uv run bench-ripenv --help
uv run bench-ripenv --ripenv --uv -b lock-cold -b lock-warm --json trio

# Compare two ripenv builds
uv run bench-ripenv \
    --ripenv-path ../../target/release/ripenv \
    --ripenv-path ../../target/release/ripenv-baseline \
    flask
```

### Fixtures

| Fixture   | Packages | Description               |
| --------- | -------- | ------------------------- |
| `trio`    | trio     | Lightweight async library |
| `flask`   | flask    | Common web framework      |
| `jupyter` | jupyter  | Heavy dependency tree     |

### Benchmark scenarios

| Benchmark      | Measures                        | Prepare step            |
| -------------- | ------------------------------- | ----------------------- |
| `lock-cold`    | Resolution, no cache            | Delete lockfile + cache |
| `lock-warm`    | Resolution, cached metadata     | Delete lockfile only    |
| `lock-noop`    | Lockfile already up to date     | Nothing                 |
| `sync-cold`    | Install from lockfile, no cache | Delete venv + cache     |
| `sync-warm`    | Install from lockfile, cached   | Delete venv only        |
| `install-cold` | Full lock+sync, no cache        | Delete all              |
| `install-warm` | Full lock+sync, cached          | Delete lockfile + venv  |

## Project structure

```
crates/ripenv/          # The ripenv crate
  src/
    bin/ripenv.rs       # Binary entry point
    lib.rs              # Library entry point
    cli.rs              # CLI argument parsing (clap)
    commands/           # Command implementations
    pipfile/            # Pipfile parser and bridge
  tests/it/             # Integration tests

scripts/benchmark-ripenv/   # Benchmark suite
  run-benchmarks.sh         # One-command benchmark runner + plot
  src/benchmark_ripenv/
    bench.py                # CLI entry point
    plot.py                 # Result visualization
    ripenv_suite.py         # ripenv benchmark commands
    pipenv_suite.py         # pipenv benchmark commands
    uv_suite.py             # uv benchmark commands
    fixtures/               # Pipfile fixtures (trio, flask, jupyter)
```

## Acknowledgements

ripenv is built on top of [uv](https://github.com/astral-sh/uv) by [Astral](https://astral.sh). This
project would not be possible without their excellent work on fast, correct Python package
management in Rust.

uv's dependency resolver uses [PubGrub](https://github.com/pubgrub-rs/pubgrub) under the hood.

## License

ripenv is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project by you, as defined in the Apache-2.0 license, shall be dually licensed as above,
without any additional terms or conditions.
