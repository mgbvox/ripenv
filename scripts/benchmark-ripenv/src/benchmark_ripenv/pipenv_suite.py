"""Pipenv benchmark suite."""

from __future__ import annotations

from pathlib import Path

from benchmark_ripenv import Command


class PipenvSuite:
    """Generates benchmark commands for pipenv."""

    def __init__(
        self,
        *,
        pipenv_path: str,
        name: str,
        working_dir: Path,
        cache_dir: Path,
    ) -> None:
        self.pipenv_path = pipenv_path
        self.name = name
        self.working_dir = working_dir
        self.cache_dir = cache_dir

    def _env_str(self) -> str:
        """Return environment variable exports for shell commands."""
        return (
            f"PIPENV_CACHE_DIR={self.cache_dir} "
            f"WORKON_HOME={self.working_dir} "
            f"PIPENV_VENV_IN_PROJECT=1 "
            f"PIPENV_YES=1"
        )

    def _rm_lockfile(self) -> str:
        """Command to remove the lockfile."""
        return f"rm -f {self.working_dir / 'Pipfile.lock'}"

    def _rm_venv(self) -> str:
        """Command to remove the virtualenv."""
        return f"rm -rf {self.working_dir / '.venv'}"

    def _rm_cache(self) -> str:
        """Command to remove the cache directory."""
        return f"rm -rf {self.cache_dir}"

    def _command(self, *args: str) -> list[str]:
        """Build a pipenv command that runs in the working directory."""
        args_str = " ".join(args)
        return [
            "sh",
            "-c",
            f"cd {self.working_dir} && {self._env_str()} {self.pipenv_path} {args_str}",
        ]

    def lock_cold(self) -> Command:
        """Lock with no cache (cold resolution)."""
        prepare = f"{self._rm_lockfile()} && {self._rm_cache()}"
        return Command(
            name=f"pipenv lock-cold ({self.name})",
            prepare=prepare,
            command=self._command("lock"),
        )

    def lock_warm(self) -> Command:
        """Lock with cached metadata."""
        prepare = self._rm_lockfile()
        return Command(
            name=f"pipenv lock-warm ({self.name})",
            prepare=prepare,
            command=self._command("lock"),
        )

    def lock_noop(self) -> Command:
        """Lock when lockfile is already up to date."""
        return Command(
            name=f"pipenv lock-noop ({self.name})",
            prepare=None,
            command=self._command("lock"),
        )

    def sync_cold(self) -> Command:
        """Sync from lockfile with no cache."""
        prepare = f"{self._rm_venv()} && {self._rm_cache()}"
        return Command(
            name=f"pipenv sync-cold ({self.name})",
            prepare=prepare,
            command=self._command("sync"),
        )

    def sync_warm(self) -> Command:
        """Sync from lockfile with cached wheels."""
        prepare = self._rm_venv()
        return Command(
            name=f"pipenv sync-warm ({self.name})",
            prepare=prepare,
            command=self._command("sync"),
        )

    def install_cold(self) -> Command:
        """Full install (lock + sync) with no cache."""
        prepare = (
            f"{self._rm_lockfile()} && {self._rm_venv()} && {self._rm_cache()}"
        )
        return Command(
            name=f"pipenv install-cold ({self.name})",
            prepare=prepare,
            command=self._command("install"),
        )

    def install_warm(self) -> Command:
        """Full install (lock + sync) with cached metadata."""
        prepare = f"{self._rm_lockfile()} && {self._rm_venv()}"
        return Command(
            name=f"pipenv install-warm ({self.name})",
            prepare=prepare,
            command=self._command("install"),
        )
