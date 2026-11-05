"""Shared local tool environment for desktop checks and preview builds."""
from __future__ import annotations

import os
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def environment(target: str = "desktop-target") -> dict[str, str]:
    env = dict(os.environ)
    paths: list[str] = []
    if platform.system() == "Darwin" and platform.machine() == "arm64":
        for relative in ("tools/rust-1.98.0-aarch64/bin", "tools/node/node-v24.22.1-darwin-arm64/bin"):
            candidate = ROOT / relative
            if candidate.is_dir():
                paths.append(str(candidate))
        env["TMPDIR"] = "/private/tmp"
    env["PATH"] = os.pathsep.join(paths + [env.get("PATH", "")])
    env["CARGO_HOME"] = str(ROOT / "tools/cargo-home")
    env["CARGO_TARGET_DIR"] = str(ROOT / "artifacts" / target)
    env["npm_config_cache"] = str(ROOT / "tools/npm-cache")
    return env


def run(arguments: list[str | Path], *, env: dict[str, str], cwd: Path = ROOT) -> None:
    subprocess.run([str(value) for value in arguments], cwd=cwd, env=env, check=True)
