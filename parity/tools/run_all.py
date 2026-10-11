#!/usr/bin/env python3
from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    cases = sorted((root / "parity" / "cases").glob("*/*"))
    status = 0
    for case_dir in cases:
        if not (case_dir / "case.toml").exists():
            continue
        result = subprocess.run([sys.executable, str(root / "parity" / "tools" / "run_case.py"), str(case_dir)], cwd=root)
        if result.returncode != 0:
            status = result.returncode
    return status


if __name__ == "__main__":
    raise SystemExit(main())

