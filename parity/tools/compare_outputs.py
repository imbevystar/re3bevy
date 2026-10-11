#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import tomllib
from typing import Any


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_case(case_dir: Path) -> dict[str, Any]:
    with (case_dir / "case.toml").open("rb") as handle:
        return tomllib.load(handle)


def tolerance_for(path: str, case: dict[str, Any]) -> float:
    tolerance = case.get("tolerance", {})
    if path in tolerance:
        return float(tolerance[path])
    tail = path.rsplit(".", 1)[-1]
    tail_base = tail.split("[", 1)[0]
    if tail in tolerance:
        return float(tolerance[tail])
    if tail_base in tolerance:
        return float(tolerance[tail_base])
    return float(tolerance.get("default", 0))


def is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def compare_values(
    path: str,
    cpp: Any,
    rust: Any,
    case: dict[str, Any],
    failures: list[dict[str, Any]],
    warnings: list[dict[str, Any]],
) -> None:
    if path in {"engine", "reference_files"}:
        return

    if isinstance(cpp, dict) and isinstance(rust, dict):
        for key in sorted(cpp.keys() - rust.keys()):
            failures.append({"path": f"{path}.{key}" if path else key, "cpp": cpp[key], "rust": None, "reason": "missing_in_rust"})
        for key in sorted(rust.keys() - cpp.keys()):
            warnings.append({"path": f"{path}.{key}" if path else key, "cpp": None, "rust": rust[key], "reason": "extra_in_rust"})
        for key in sorted(cpp.keys() & rust.keys()):
            compare_values(f"{path}.{key}" if path else key, cpp[key], rust[key], case, failures, warnings)
        return

    if isinstance(cpp, list) and isinstance(rust, list):
        if len(cpp) != len(rust):
            failures.append({"path": path, "cpp": len(cpp), "rust": len(rust), "reason": "list_length"})
            return
        for index, (cpp_item, rust_item) in enumerate(zip(cpp, rust)):
            compare_values(f"{path}[{index}]", cpp_item, rust_item, case, failures, warnings)
        return

    if is_number(cpp) and is_number(rust):
        tolerance = tolerance_for(path, case)
        diff = float(rust) - float(cpp)
        if math.isnan(diff) or abs(diff) > tolerance:
            failures.append({"path": path, "cpp": cpp, "rust": rust, "diff": diff, "tolerance": tolerance})
        return

    if cpp != rust:
        failures.append({"path": path, "cpp": cpp, "rust": rust, "reason": "exact_mismatch"})


def write_summary(path: Path, diff: dict[str, Any]) -> None:
    lines = [
        f"case: {diff['case']}",
        f"status: {diff['status']}",
        f"failures: {len(diff['failures'])}",
        f"warnings: {len(diff['warnings'])}",
    ]
    for failure in diff["failures"][:50]:
        lines.append(f"FAIL {failure}")
    for warning in diff["warnings"][:50]:
        lines.append(f"WARN {warning}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Compare C++ re3 and Rust re3bevy parity JSON outputs.")
    parser.add_argument("case_dir", type=Path)
    parser.add_argument("--cpp", type=Path)
    parser.add_argument("--rust", type=Path)
    parser.add_argument("--diff", type=Path)
    parser.add_argument("--summary", type=Path)
    args = parser.parse_args()

    case_dir = args.case_dir
    out_dir = case_dir / "out"
    cpp_path = args.cpp or out_dir / "cpp.json"
    rust_path = args.rust or out_dir / "rust.json"
    diff_path = args.diff or out_dir / "diff.json"
    summary_path = args.summary or out_dir / "summary.txt"

    case = load_case(case_dir)
    cpp = load_json(cpp_path)
    rust = load_json(rust_path)
    failures: list[dict[str, Any]] = []
    warnings: list[dict[str, Any]] = []

    compare_values("", cpp, rust, case, failures, warnings)

    status = "fail" if failures else "warn" if warnings else "pass"
    diff = {
        "case": case.get("name", case_dir.name),
        "status": status,
        "failures": failures,
        "warnings": warnings,
    }

    diff_path.parent.mkdir(parents=True, exist_ok=True)
    diff_path.write_text(json.dumps(diff, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_summary(summary_path, diff)

    return 0 if status in {"pass", "warn"} else 1


if __name__ == "__main__":
    raise SystemExit(main())
