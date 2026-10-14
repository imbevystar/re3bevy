#!/usr/bin/env python3
"""Check source candidates for accidental data/configuration disclosure.

This is a file hygiene check, not a copyright or license clearance tool.
With --snapshot, check a history-free export instead of Git's candidate files.
With --export, copy checked candidates without Git or local reference inputs.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys


REQUIRED = (
    "LICENSE", "THIRD_PARTY_NOTICES.md",
    "crates/rustgta3_formats/native/LICENSE.pl_mpeg",
)
LOCAL_ROOTS = {
    ".git", ".local-git-remote", "target", "dist", "game-data",
    "original-game-data", "node_modules", ".gradle",
}
GAME_FILES = {
    "gta3.img", "gta3.dir", "main.scm", "sfx.raw", "sfx.sdt",
    "gta3.dat", "default.dat", "waterpro.dat", "timecyc.dat",
    "carcols.dat", "handling.cfg", "particle.cfg", "animgrp.dat",
}
GAME_SUFFIXES = {".dff", ".txd", ".ifp", ".gxt", ".scm", ".col"}
PACKAGE_SUFFIXES = {".apk", ".aab", ".exe", ".dll", ".so", ".pdb", ".zip", ".7z", ".rar"}
PRIVATE_SUFFIXES = {".pem", ".p12", ".pfx", ".keystore", ".jks"}
SECRET = re.compile(
    rb"-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY-----|"
    rb"\bgh[pousr]_[A-Za-z0-9]{36,}\b|\bAKIA[A-Z0-9]{16}\b"
)


def git(root: Path, *args: str, data: bytes | None = None, allowed: tuple[int, ...] = (0,)) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(root), *args], input=data, capture_output=True,
    )
    if result.returncode not in allowed:
        raise RuntimeError("Git could not enumerate the source candidates")
    return result.stdout


def candidates(root: Path, snapshot: bool) -> list[str]:
    if snapshot:
        paths = []
        for parent, directories, files in os.walk(root, followlinks=False):
            # Do not walk into metadata or linked directories in an export.
            for name in list(directories):
                directory = Path(parent, name)
                local_root = Path(parent) == root and name.lower() in LOCAL_ROOTS
                if local_root or name == ".git" or directory.is_symlink():
                    paths.append(directory.relative_to(root).as_posix())
                    directories.remove(name)
            paths.extend(Path(parent, name).relative_to(root).as_posix() for name in files)
        return sorted(paths)
    raw = git(root, "ls-files", "--cached", "--others", "--exclude-standard", "-z")
    return sorted({os.fsdecode(path) for path in raw.split(b"\0") if path})


def check(root: Path, snapshot: bool) -> list[str]:
    problems = []
    paths = candidates(root, snapshot)
    present = []
    for relative in paths:
        path = root / relative
        parts = PurePosixPath(relative).parts
        if not parts or ".." in parts or PurePosixPath(relative).is_absolute():
            problems.append(f"Unsafe source path: {relative}")
            continue
        if path.is_symlink():
            problems.append(f"Linked path needs manual review: {relative}")
            continue
        if parts[0].lower() in LOCAL_ROOTS or (snapshot and relative == ".gitmodules"):
            problems.append(f"Local/generated metadata in source: {relative}")
            continue
        if not path.is_file():
            # Gitlinks are pointers, not reference source. Deleted working-tree
            # files are absent from the prospective fresh source snapshot.
            continue
        if not path.resolve().is_relative_to(root):
            problems.append(f"Path escapes source root: {relative}")
            continue
        present.append(relative)
        name = path.name.lower()
        suffix = path.suffix.lower()
        if parts[0].lower() == "references":
            problems.append(f"Reference source in distribution: {relative}")
        if name in GAME_FILES or suffix in GAME_SUFFIXES:
            problems.append(f"Possible original game data; review its origin: {relative}")
        if suffix in PACKAGE_SUFFIXES or suffix in PRIVATE_SUFFIXES:
            problems.append(f"Binary/package/private key in source: {relative}")
        if name == ".env" or (name.startswith(".env.") and not name.endswith(".example")):
            problems.append(f"Local environment file in source: {relative}")
        if relative.lower() == "config/original_gta3_path.txt":
            problems.append(f"Local installation configuration in source: {relative}")
        if path.stat().st_size > 16 * 1024 * 1024:
            problems.append(f"Large file needs manual distribution review: {relative}")
            continue
        content = path.read_bytes()
        if content.startswith((b"MZ", b"PK\x03\x04", b"\x7fELF")):
            problems.append(f"Binary/archive signature in source: {relative}")
        if SECRET.search(content):
            # Never echo the secret itself into CI or chat output.
            problems.append(f"Possible embedded credential: {relative}")

    for required in REQUIRED:
        if required not in present:
            problems.append(f"Missing license/notice: {required}")
    if not snapshot and present:
        data = b"\0".join(os.fsencode(path) for path in present) + b"\0"
        ignored = git(root, "check-ignore", "--no-index", "--stdin", "-z", data=data, allowed=(0, 1))
        problems.extend(
            f"Ignored local/generated file tracked in Git: {os.fsdecode(path)}"
            for path in ignored.split(b"\0") if path
        )
    return sorted(set(problems))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--snapshot", type=Path, help="check a source export with no Git metadata")
    mode.add_argument("--export", type=Path, help="create a new source snapshot without Git or reference trees; does not clear publication rights")
    arguments = parser.parse_args()
    root = (arguments.snapshot or Path(__file__).resolve().parent.parent).resolve()
    if not root.is_dir():
        parser.error("source root is not a directory")
    problems = check(root, arguments.snapshot is not None)
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        return 1
    if arguments.export:
        destination = arguments.export.resolve()
        if destination.is_relative_to(root) and not destination.is_relative_to(root / "dist" / "source"):
            parser.error("exports inside the repository must be under dist/source")
        if destination.exists():
            parser.error("export destination must not already exist; existing work is never replaced")
        paths = candidates(root, False)
        destination.mkdir(parents=True)
        for relative in paths:
            source = root / relative
            if (relative == ".gitmodules"
                    or PurePosixPath(relative).parts[0] == "references"):
                continue
            if not source.is_file():
                continue
            if source.is_symlink() or not source.resolve().is_relative_to(root):
                raise RuntimeError(f"Source path changed during export: {relative}")
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        # Recheck copied bytes, including files changed during the export.
        problems = check(destination, True)
        if problems:
            for problem in problems:
                print(problem, file=sys.stderr)
            return 1
        print(f"History-free source snapshot created: {destination}")
    print("Source file hygiene checks passed. This does not clear redistribution rights.")
    print("Reference trees stay local; review THIRD_PARTY_NOTICES.md.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
