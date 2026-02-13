"""uv benchmark suite."""

from __future__ import annotations

import re
from pathlib import Path

from benchmark_ripenv import Command


def _parse_pipfile_section(content: str, section: str) -> list[str]:
    """Extract package specs from a Pipfile section.

    Returns a list of PEP 508 dependency strings (e.g. ``["flask", "numpy"]``).
    """
    dependencies: list[str] = []
    in_section = False
    for line in content.splitlines():
        stripped = line.strip()
        if stripped == f"[{section}]":
            in_section = True
            continue
        if stripped.startswith("["):
            in_section = False
            continue
        if not in_section or not stripped or stripped.startswith("#"):
            continue

        # Parse `name = "version"` lines.
        match = re.match(r'^(\S+)\s*=\s*"([^"]*)"', stripped)
        if match:
            name, version = match.groups()
            if version == "*":
                dependencies.append(name)
            else:
                dependencies.append(f"{name}{version}")
    return dependencies


def generate_pyproject_from_pipfile(pipfile_path: Path, output_dir: Path) -> None:
    """Parse a Pipfile and generate an equivalent pyproject.toml for uv.

    Handles both ``[packages]`` (mapped to ``[project].dependencies``) and
    ``[dev-packages]`` (mapped to ``[dependency-groups].dev``).
    """
    content = pipfile_path.read_text()

    packages = _parse_pipfile_section(content, "packages")
    dev_packages = _parse_pipfile_section(content, "dev-packages")

    deps_str = ", ".join(f'"{dep}"' for dep in packages)
    dev_str = ", ".join(f'"{dep}"' for dep in dev_packages)

    pyproject = f"""\
[project]
name = "bench-fixture"
version = "0.0.1"
requires-python = ">=3.12"
dependencies = [{deps_str}]

[dependency-groups]
dev = [{dev_str}]
"""
    (output_dir / "pyproject.toml").write_text(pyproject)


class UvSuite:
    """Generates benchmark commands for uv."""

    def __init__(
        self,
        *,
        uv_path: str,
        name: str,
        working_dir: Path,
        cache_dir: Path,
    ) -> None:
        self.uv_path = uv_path
        self.name = name
        self.working_dir = working_dir
        self.cache_dir = cache_dir

    def _rm_lockfile(self) -> str:
        """Command to remove the lockfile."""
        return f"rm -f {self.working_dir / 'uv.lock'}"

    def _rm_venv(self) -> str:
        """Command to remove the virtualenv."""
        return f"rm -rf {self.working_dir / '.venv'}"

    def _rm_cache(self) -> str:
        """Command to remove the cache directory."""
        return f"rm -rf {self.cache_dir}"

    def _command(self, *args: str) -> list[str]:
        """Build a uv command."""
        return [
            self.uv_path,
            *args,
            "--cache-dir",
            str(self.cache_dir),
            "--directory",
            str(self.working_dir),
        ]

    def lock_cold(self) -> Command:
        """Lock with no cache (cold resolution)."""
        prepare = f"{self._rm_lockfile()} && {self._rm_cache()}"
        return Command(
            name=f"uv lock-cold ({self.name})",
            prepare=prepare,
            command=self._command("lock"),
        )

    def lock_warm(self) -> Command:
        """Lock with cached metadata."""
        prepare = self._rm_lockfile()
        return Command(
            name=f"uv lock-warm ({self.name})",
            prepare=prepare,
            command=self._command("lock"),
        )

    def lock_noop(self) -> Command:
        """Lock when lockfile is already up to date."""
        return Command(
            name=f"uv lock-noop ({self.name})",
            prepare=None,
            command=self._command("lock"),
        )

    def sync_cold(self) -> Command:
        """Sync from lockfile with no cache."""
        prepare = f"{self._rm_venv()} && {self._rm_cache()}"
        return Command(
            name=f"uv sync-cold ({self.name})",
            prepare=prepare,
            command=self._command("sync"),
        )

    def sync_warm(self) -> Command:
        """Sync from lockfile with cached wheels."""
        prepare = self._rm_venv()
        return Command(
            name=f"uv sync-warm ({self.name})",
            prepare=prepare,
            command=self._command("sync"),
        )

    def install_cold(self) -> Command:
        """Full lock + sync with no cache."""
        prepare = f"{self._rm_lockfile()} && {self._rm_venv()} && {self._rm_cache()}"
        # uv doesn't have a single "install" command; chain lock + sync.
        lock_cmd = " ".join(self._command("lock"))
        sync_cmd = " ".join(self._command("sync"))
        return Command(
            name=f"uv install-cold ({self.name})",
            prepare=prepare,
            command=["sh", "-c", f"{lock_cmd} && {sync_cmd}"],
        )

    def install_warm(self) -> Command:
        """Full lock + sync with cached metadata."""
        prepare = f"{self._rm_lockfile()} && {self._rm_venv()}"
        lock_cmd = " ".join(self._command("lock"))
        sync_cmd = " ".join(self._command("sync"))
        return Command(
            name=f"uv install-warm ({self.name})",
            prepare=prepare,
            command=["sh", "-c", f"{lock_cmd} && {sync_cmd}"],
        )
