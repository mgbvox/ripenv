#!/usr/bin/env bash
# Run ripenv vs pipenv vs uv benchmarks and generate a comparison plot.
#
# Usage:
#   ./run-benchmarks.sh [FIXTURE]           # default: trio
#   ./run-benchmarks.sh trio flask jupyter   # run multiple fixtures
#
# Prerequisites:
#   - hyperfine: brew install hyperfine
#   - pipenv:    pip install pipenv
#   - ripenv:    cargo build --release -p ripenv
#   - uv:        available on PATH or built locally
#
# Options via environment variables:
#   RIPENV_PATH  - path to ripenv binary (default: auto-detect)
#   PIPENV_PATH  - path to pipenv binary (default: auto-detect)
#   UV_PATH      - path to uv binary (default: auto-detect)
#   BENCHMARKS   - comma-separated list of benchmarks to run
#                  (default: all 7 benchmarks)
#   WARMUP       - warmup runs (default: 3)
#   MIN_RUNS     - minimum runs (default: 10)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="${SCRIPT_DIR}/results"

# Default fixtures if none provided.
FIXTURES=("${@:-flask}")

# Tool path flags.
RIPENV_FLAG=()
PIPENV_FLAG=()
UV_FLAG=()

if [[ -n "${RIPENV_PATH:-}" ]]; then
    RIPENV_FLAG=(--ripenv-path "$RIPENV_PATH")
elif command -v ripenv &>/dev/null; then
    RIPENV_FLAG=(--ripenv)
elif [[ -x "${SCRIPT_DIR}/../../target/release/ripenv" ]]; then
    RIPENV_FLAG=(--ripenv-path "${SCRIPT_DIR}/../../target/release/ripenv")
else
    echo "Warning: ripenv not found, skipping ripenv benchmarks" >&2
fi

if [[ -n "${PIPENV_PATH:-}" ]]; then
    PIPENV_FLAG=(--pipenv-path "$PIPENV_PATH")
elif command -v pipenv &>/dev/null; then
    PIPENV_FLAG=(--pipenv)
else
    echo "Warning: pipenv not found, skipping pipenv benchmarks" >&2
fi

if [[ -n "${UV_PATH:-}" ]]; then
    UV_FLAG=(--uv-path "$UV_PATH")
elif command -v uv &>/dev/null; then
    UV_FLAG=(--uv)
else
    echo "Warning: uv not found, skipping uv benchmarks" >&2
fi

if [[ ${#RIPENV_FLAG[@]} -eq 0 && ${#PIPENV_FLAG[@]} -eq 0 && ${#UV_FLAG[@]} -eq 0 ]]; then
    echo "Error: no benchmark tools found" >&2
    exit 1
fi

# Benchmark selection.
BENCH_FLAGS=()
if [[ -n "${BENCHMARKS:-}" ]]; then
    IFS=',' read -ra BENCH_NAMES <<< "$BENCHMARKS"
    for bench in "${BENCH_NAMES[@]}"; do
        BENCH_FLAGS+=(-b "$bench")
    done
fi

# Hyperfine tuning.
WARMUP_FLAG=(--warmup "${WARMUP:-3}")
MIN_RUNS_FLAG=(--min-runs "${MIN_RUNS:-10}")

for fixture in "${FIXTURES[@]}"; do
    echo ""
    echo "================================================================"
    echo "  Running benchmarks: ${fixture}"
    echo "================================================================"
    echo ""

    fixture_results="${RESULTS_DIR}/${fixture}"
    mkdir -p "$fixture_results"

    # Run benchmarks with JSON export into the fixture results directory.
    (
        cd "$fixture_results"
        uv run --project "$SCRIPT_DIR" bench-ripenv \
            ${RIPENV_FLAG[@]+"${RIPENV_FLAG[@]}"} \
            ${PIPENV_FLAG[@]+"${PIPENV_FLAG[@]}"} \
            ${UV_FLAG[@]+"${UV_FLAG[@]}"} \
            ${BENCH_FLAGS[@]+"${BENCH_FLAGS[@]}"} \
            "${WARMUP_FLAG[@]}" \
            "${MIN_RUNS_FLAG[@]}" \
            --json \
            "$fixture"
    )

    # Generate plot.
    echo ""
    echo "Generating plot for ${fixture}..."
    uv run --project "$SCRIPT_DIR" python -m benchmark_ripenv.plot \
        "$fixture_results" \
        -o "$fixture_results/benchmark-results.png"
done

echo ""
echo "================================================================"
echo "  Results saved to: ${RESULTS_DIR}/"
echo "================================================================"
ls -la "${RESULTS_DIR}"/*/*.png 2>/dev/null || true
