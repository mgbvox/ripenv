"""Ripenv benchmark suite."""

from __future__ import annotations

from pathlib import Path

from benchmark_ripenv import Command


class RipenvSuite:
    """Generates benchmark commands for ripenv."""

    def __init__(
        self,
        *,
        ripenv_path: str,
        name: str,
        working_dir: Path,
        pipfile_path: Path,
        cache_dir: Path,
    ) -> None:
        self.ripenv_path = ripenv_path
        self.name = name
        self.working_dir = working_dir
        self.pipfile_path = pipfile_path
        self.cache_dir = cache_dir

    def _env_prefix(self) -> str:
        """Return the environment variable prefix for ripenv commands."""
        return f"PIPENV_PIPFILE={self.pipfile_path}"

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
        """Build a ripenv command with environment."""
        return [
            "env",
            self._env_prefix(),
            self.ripenv_path,
            *args,
        ]

    def lock_cold(self) -> Command:
        """Lock with no cache (cold resolution)."""
        prepare = f"{self._rm_lockfile()} && {self._rm_cache()}"
        return Command(
            name=f"ripenv lock-cold ({self.name})",
            prepare=prepare,
            command=self._command("lock"),
        )

    def lock_warm(self) -> Command:
        """Lock with cached metadata."""
        prepare = self._rm_lockfile()
        return Command(
            name=f"ripenv lock-warm ({self.name})",
            prepare=prepare,
            command=self._command("lock"),
        )

    def lock_noop(self) -> Command:
        """Lock when lockfile is already up to date."""
        return Command(
            name=f"ripenv lock-noop ({self.name})",
            prepare=None,
            command=self._command("lock"),
        )

    def sync_cold(self) -> Command:
        """Sync from lockfile with no cache."""
        prepare = f"{self._rm_venv()} && {self._rm_cache()}"
        return Command(
            name=f"ripenv sync-cold ({self.name})",
            prepare=prepare,
            command=self._command("sync"),
        )

    def sync_warm(self) -> Command:
        """Sync from lockfile with cached wheels."""
        prepare = self._rm_venv()
        return Command(
            name=f"ripenv sync-warm ({self.name})",
            prepare=prepare,
            command=self._command("sync"),
        )

    def install_cold(self) -> Command:
        """Full install (lock + sync) with no cache."""
        prepare = f"{self._rm_lockfile()} && {self._rm_venv()} && {self._rm_cache()}"
        return Command(
            name=f"ripenv install-cold ({self.name})",
            prepare=prepare,
            command=self._command("install"),
        )

    def install_warm(self) -> Command:
        """Full install (lock + sync) with cached metadata."""
        prepare = f"{self._rm_lockfile()} && {self._rm_venv()}"
        return Command(
            name=f"ripenv install-warm ({self.name})",
            prepare=prepare,
            command=self._command("install"),
        )
