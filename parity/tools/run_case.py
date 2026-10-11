#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)


def write_blocked(case_dir: Path, out_dir: Path, reason: str, log: str) -> None:
    case_name = case_dir.name
    case_file = case_dir / "case.toml"
    if case_file.exists():
        for line in case_file.read_text(encoding="utf-8").splitlines():
            if line.strip().startswith("name") and "=" in line:
                case_name = line.split("=", 1)[1].strip().strip('"')
                break
    diff: dict[str, Any] = {
        "case": case_name,
        "status": "blocked",
        "reason": reason,
        "log": log[-8000:],
        "failures": [],
        "warnings": [],
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "diff.json").write_text(json.dumps(diff, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (out_dir / "summary.txt").write_text(
        f"case: {case_name}\nstatus: blocked\nreason: {reason}\n\n{log[-4000:]}\n",
        encoding="utf-8",
    )


def build_cpp(root: Path) -> tuple[Path | None, str]:
    cmake = shutil.which("cmake")
    if cmake is None:
        return None, "cmake not found"

    source = root / "parity" / "cpp_harness"
    build = source / "build"
    configure = run([cmake, "-S", str(source), "-B", str(build)], root)
    if configure.returncode != 0:
        return None, configure.stdout

    build_result = run([cmake, "--build", str(build), "--config", "Debug"], root)
    if build_result.returncode != 0:
        return None, configure.stdout + "\n" + build_result.stdout

    candidates = list(build.rglob("re3bevy_parity_cpp_harness.exe")) + list(build.rglob("re3bevy_parity_cpp_harness"))
    if not candidates:
        return None, configure.stdout + "\n" + build_result.stdout + "\nC++ harness executable not found"
    return candidates[0], configure.stdout + "\n" + build_result.stdout


def main() -> int:
    parser = argparse.ArgumentParser(description="Run one re3bevy parity case.")
    parser.add_argument("case_dir", type=Path)
    args = parser.parse_args()

    root = repo_root()
    case_dir = args.case_dir.resolve()
    case_file = case_dir / "case.toml"
    out_dir = case_dir / "out"
    cpp_out = out_dir / "cpp.json"
    rust_out = out_dir / "rust.json"

    if not case_file.exists():
        print(f"missing case file: {case_file}", file=sys.stderr)
        return 2

    out_dir.mkdir(parents=True, exist_ok=True)

    cpp_exe, cpp_log = build_cpp(root)
    if cpp_exe is None:
        write_blocked(case_dir, out_dir, "cpp_harness_build_failed", cpp_log)
        print((out_dir / "summary.txt").read_text(encoding="utf-8"))
        return 3

    cpp_run = run([str(cpp_exe), "--case-file", str(case_file), "--out", str(cpp_out)], root)
    if cpp_run.returncode != 0:
        write_blocked(case_dir, out_dir, "cpp_harness_run_failed", cpp_run.stdout)
        print((out_dir / "summary.txt").read_text(encoding="utf-8"))
        return 3

    rust_run = run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "re3bevy_parity_harness",
            "--",
            "--case-file",
            str(case_file),
            "--out",
            str(rust_out),
        ],
        root,
    )
    if rust_run.returncode != 0:
        write_blocked(case_dir, out_dir, "rust_harness_run_failed", rust_run.stdout)
        print((out_dir / "summary.txt").read_text(encoding="utf-8"))
        return 3

    compare = run([sys.executable, str(root / "parity" / "tools" / "compare_outputs.py"), str(case_dir)], root)
    print(compare.stdout)
    if (out_dir / "summary.txt").exists():
        print((out_dir / "summary.txt").read_text(encoding="utf-8"))
    return compare.returncode


if __name__ == "__main__":
    raise SystemExit(main())

