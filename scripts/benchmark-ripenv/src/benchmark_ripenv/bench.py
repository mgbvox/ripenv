"""CLI entry point for the ripenv benchmark suite."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from benchmark_ripenv import Hyperfine
from benchmark_ripenv.pipenv_suite import PipenvSuite
from benchmark_ripenv.ripenv_suite import RipenvSuite
from benchmark_ripenv.uv_suite import UvSuite, generate_pyproject_from_pipfile

FIXTURES_DIR = Path(__file__).parent / "fixtures"

ALL_BENCHMARKS = [
    "lock-cold",
    "lock-warm",
    "lock-noop",
    "sync-cold",
    "sync-warm",
    "install-cold",
    "install-warm",
]


def find_binary(name: str) -> str | None:
    """Find a binary on PATH, returning its full path or None.

    For pyenv shims, attempts to resolve the actual binary path via
    ``pyenv which`` to avoid shim failures in subprocess calls.
    """
    result = shutil.which(name)
    if result is None:
        return None

    # Resolve pyenv shims to the actual binary.
    if "pyenv/shims" in result:
        try:
            resolved = subprocess.run(
                ["pyenv", "which", name],
                capture_output=True,
                text=True,
                check=True,
            )
            return resolved.stdout.strip()
        except (subprocess.CalledProcessError, FileNotFoundError):
            pass

    return result


def setup_ripenv_dir(fixture: str, working_dir: Path, pipfile_path: Path) -> None:
    """Set up a ripenv working directory with the fixture Pipfile."""
    working_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy(pipfile_path, working_dir / "Pipfile")


def setup_pipenv_dir(fixture: str, working_dir: Path, pipfile_path: Path) -> None:
    """Set up a pipenv working directory with the fixture Pipfile."""
    working_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy(pipfile_path, working_dir / "Pipfile")


def setup_uv_dir(fixture: str, working_dir: Path, pipfile_path: Path) -> None:
    """Set up a uv working directory with a generated pyproject.toml."""
    working_dir.mkdir(parents=True, exist_ok=True)
    generate_pyproject_from_pipfile(pipfile_path, working_dir)


def initial_lock(
    tool: str,
    binary_path: str,
    working_dir: Path,
    cache_dir: Path,
    pipfile_path: Path | None = None,
) -> None:
    """Run an initial lock to create the lockfile for warm/noop benchmarks."""
    env = None
    if tool == "ripenv":
        cmd = [binary_path, "lock"]
        env = {
            **dict(__import__("os").environ),
            "PIPENV_PIPFILE": str(pipfile_path),
        }
    elif tool == "pipenv":
        cmd = [binary_path, "lock"]
        env = {
            **dict(__import__("os").environ),
            "PIPENV_CACHE_DIR": str(cache_dir),
            "WORKON_HOME": str(working_dir),
            "PIPENV_VENV_IN_PROJECT": "1",
            "PIPENV_YES": "1",
        }
    elif tool == "uv":
        cmd = [
            binary_path,
            "lock",
            "--cache-dir",
            str(cache_dir),
            "--directory",
            str(working_dir),
        ]
    else:
        msg = f"Unknown tool: {tool}"
        raise ValueError(msg)

    result = subprocess.run(
        cmd,
        cwd=working_dir,
        env=env,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"  initial lock failed (exit {result.returncode}):", file=sys.stderr)
        if result.stderr:
            print(f"  stderr: {result.stderr.strip()}", file=sys.stderr)
        if result.stdout:
            print(f"  stdout: {result.stdout.strip()}", file=sys.stderr)
        sys.exit(1)


def run_benchmarks(
    benchmarks: list[str],
    suites: list[tuple[str, object]],
    *,
    warmup: int,
    min_runs: int,
    runs: int | None,
    verbose: bool,
    json: bool,
) -> None:
    """Run the selected benchmarks across all suites."""
    benchmark_methods = {
        "lock-cold": "lock_cold",
        "lock-warm": "lock_warm",
        "lock-noop": "lock_noop",
        "sync-cold": "sync_cold",
        "sync-warm": "sync_warm",
        "install-cold": "install_cold",
        "install-warm": "install_warm",
    }

    for benchmark_name in benchmarks:
        method_name = benchmark_methods[benchmark_name]
        commands = []
        for _suite_name, suite in suites:
            method = getattr(suite, method_name, None)
            if method is not None:
                commands.append(method())

        if not commands:
            continue

        hyperfine = Hyperfine(
            name=benchmark_name,
            commands=commands,
            warmup=warmup,
            min_runs=min_runs,
            runs=runs,
            verbose=verbose,
            json=json,
        )

        print(f"\n{'=' * 60}")
        print(f"  Benchmark: {benchmark_name}")
        print(f"{'=' * 60}\n")

        hyperfine.run()


def main() -> None:
    """CLI entry point."""
    parser = argparse.ArgumentParser(
        prog="bench-ripenv",
        description="Benchmark ripenv vs pipenv vs uv",
    )

    parser.add_argument(
        "fixture",
        choices=os.listdir(Path(__file__).parent / "fixtures"),
        help="Fixture name (trio, flask, jupyter)",
    )

    # Tool selection.
    parser.add_argument(
        "--ripenv",
        action="store_true",
        help="Benchmark ripenv",
    )
    parser.add_argument(
        "--pipenv",
        action="store_true",
        help="Benchmark pipenv",
    )
    parser.add_argument(
        "--uv",
        action="store_true",
        help="Benchmark uv",
    )

    # Custom binary paths (repeatable).
    parser.add_argument(
        "--ripenv-path",
        action="append",
        default=[],
        help="Path to ripenv binary (repeatable for comparison)",
    )
    parser.add_argument(
        "--pipenv-path",
        action="append",
        default=[],
        help="Path to pipenv binary (repeatable for comparison)",
    )
    parser.add_argument(
        "--uv-path",
        action="append",
        default=[],
        help="Path to uv binary (repeatable for comparison)",
    )

    # Benchmark selection.
    parser.add_argument(
        "-b",
        "--benchmark",
        action="append",
        default=[],
        choices=ALL_BENCHMARKS,
        help="Specific benchmark(s) to run (repeatable)",
    )

    # Hyperfine options.
    parser.add_argument(
        "--warmup",
        type=int,
        default=3,
        help="Warmup runs (default: 3)",
    )
    parser.add_argument(
        "--min-runs",
        type=int,
        default=10,
        help="Minimum runs (default: 10)",
    )
    parser.add_argument(
        "--runs",
        type=int,
        default=None,
        help="Exact number of runs",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Export results to JSON",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show command output",
    )

    args = parser.parse_args()

    # Default to all tools if none specified.
    if not args.ripenv and not args.pipenv and not args.uv:
        args.ripenv = True
        args.pipenv = True
        args.uv = True

    # Default to all benchmarks if none specified.
    benchmarks = args.benchmark if args.benchmark else ALL_BENCHMARKS

    # Locate fixture Pipfile.
    pipfile_path = FIXTURES_DIR / args.fixture / "Pipfile"
    if not pipfile_path.exists():
        print(f"Error: fixture Pipfile not found: {pipfile_path}", file=sys.stderr)
        sys.exit(1)

    # Resolve binary paths.
    ripenv_paths: list[str] = args.ripenv_path
    pipenv_paths: list[str] = args.pipenv_path
    uv_paths: list[str] = args.uv_path

    if args.ripenv and not ripenv_paths:
        found = find_binary("ripenv")
        if found:
            ripenv_paths = [found]
        else:
            print("Warning: ripenv not found on PATH, skipping", file=sys.stderr)
            args.ripenv = False

    if args.pipenv and not pipenv_paths:
        found = find_binary("pipenv")
        if found:
            pipenv_paths = [found]
        else:
            print("Warning: pipenv not found on PATH, skipping", file=sys.stderr)
            args.pipenv = False

    if args.uv and not uv_paths:
        found = find_binary("uv")
        if found:
            uv_paths = [found]
        else:
            print("Warning: uv not found on PATH, skipping", file=sys.stderr)
            args.uv = False

    # Check hyperfine is available.
    if not shutil.which("hyperfine"):
        print(
            "Error: hyperfine is required. Install with: brew install hyperfine",
            file=sys.stderr,
        )
        sys.exit(1)

    # Create temp directory for isolated runs.
    with tempfile.TemporaryDirectory(prefix="bench-ripenv-") as tmpdir:
        tmpdir_path = Path(tmpdir)
        suites: list[tuple[str, object]] = []

        # Set up ripenv suites.
        if args.ripenv:
            for i, ripenv_path in enumerate(ripenv_paths):
                suffix = f"-{i}" if len(ripenv_paths) > 1 else ""
                label = Path(ripenv_path).name + suffix
                working_dir = tmpdir_path / f"ripenv{suffix}"
                cache_dir = tmpdir_path / f"ripenv-cache{suffix}"
                cache_dir.mkdir(parents=True, exist_ok=True)

                setup_ripenv_dir(args.fixture, working_dir, pipfile_path)
                print(f"Setting up ripenv ({label})...")
                initial_lock(
                    "ripenv",
                    ripenv_path,
                    working_dir,
                    cache_dir,
                    pipfile_path=working_dir / "Pipfile",
                )
                suites.append(
                    (
                        f"ripenv ({label})",
                        RipenvSuite(
                            ripenv_path=ripenv_path,
                            name=label,
                            working_dir=working_dir,
                            pipfile_path=working_dir / "Pipfile",
                            cache_dir=cache_dir,
                        ),
                    )
                )

        # Set up pipenv suites.
        if args.pipenv:
            for i, pipenv_path in enumerate(pipenv_paths):
                suffix = f"-{i}" if len(pipenv_paths) > 1 else ""
                label = Path(pipenv_path).name + suffix
                working_dir = tmpdir_path / f"pipenv{suffix}"
                cache_dir = tmpdir_path / f"pipenv-cache{suffix}"
                cache_dir.mkdir(parents=True, exist_ok=True)

                setup_pipenv_dir(args.fixture, working_dir, pipfile_path)
                print(f"Setting up pipenv ({label})...")
                initial_lock(
                    "pipenv",
                    pipenv_path,
                    working_dir,
                    cache_dir,
                )
                suites.append(
                    (
                        f"pipenv ({label})",
                        PipenvSuite(
                            pipenv_path=pipenv_path,
                            name=label,
                            working_dir=working_dir,
                            cache_dir=cache_dir,
                        ),
                    )
                )

        # Set up uv suites.
        if args.uv:
            for i, uv_path in enumerate(uv_paths):
                suffix = f"-{i}" if len(uv_paths) > 1 else ""
                label = Path(uv_path).name + suffix
                working_dir = tmpdir_path / f"uv{suffix}"
                cache_dir = tmpdir_path / f"uv-cache{suffix}"
                cache_dir.mkdir(parents=True, exist_ok=True)

                setup_uv_dir(args.fixture, working_dir, pipfile_path)
                print(f"Setting up uv ({label})...")
                initial_lock(
                    "uv",
                    uv_path,
                    working_dir,
                    cache_dir,
                )
                suites.append(
                    (
                        f"uv ({label})",
                        UvSuite(
                            uv_path=uv_path,
                            name=label,
                            working_dir=working_dir,
                            cache_dir=cache_dir,
                        ),
                    )
                )

        if not suites:
            print("Error: no tools available to benchmark", file=sys.stderr)
            sys.exit(1)

        print(f"\nBenchmarking fixture: {args.fixture}")
        print(f"Tools: {', '.join(name for name, _ in suites)}")
        print(f"Benchmarks: {', '.join(benchmarks)}")

        run_benchmarks(
            benchmarks,
            suites,
            warmup=args.warmup,
            min_runs=args.min_runs,
            runs=args.runs,
            verbose=args.verbose,
            json=args.json,
        )


if __name__ == "__main__":
    main()
