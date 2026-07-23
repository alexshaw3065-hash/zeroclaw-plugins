#!/usr/bin/env python3
"""Keep each plugin's vendored solana-core copy identical to the canonical one.

The real CI (tools/ci/validate_components.sh) builds every plugin from an
isolated snapshot of just that plugin's own directory plus wit/v0 -- nothing
else from the repo. That means solana-core cannot be a normal shared path
dependency (plugins/*/Cargo.toml pointing at ../../solana-core): it has to be
vendored, as a plain copy, inside every plugin directory that uses it.

Top-level solana-core/ stays the single canonical source a contributor edits.
This script propagates it (`sync`) or verifies no vendored copy has drifted
from it (`check`, the CI-friendly mode: no writes, non-zero exit on
mismatch).

One deliberate difference between the canonical crate and every vendored
copy: the canonical Cargo.toml ends in a `[workspace]` table so `cd
solana-core && cargo test` works standalone. A *vendored* copy sits inside a
plugin directory whose own Cargo.toml already declares `[workspace]` (making
the plugin the workspace root); Cargo rejects a path dependency that is
itself a second workspace root nested in the same tree ("multiple workspace
roots found in the same workspace"). So vendored copies have that trailing
block stripped -- `_strip_workspace_marker` below is the one place that
transform happens, and both `sync` and `check` apply it identically.

Usage:
    python3 tools/sync_solana_core.py sync
    python3 tools/sync_solana_core.py check
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CANONICAL = REPO_ROOT / "solana-core"

# Every plugin directory that vendors a copy of solana-core.
VENDORED_IN = [
    "token-risk-check",
    "solana-pay-request",
    "payment-watch",
    "sns-resolve",
]

# Build/lock artifacts that legitimately differ per vendored copy and must
# not be compared or copied.
IGNORE_NAMES = {"target", "Cargo.lock", ".gitignore"}


def vendor_dirs() -> list[Path]:
    return [REPO_ROOT / "plugins" / name / "solana-core" for name in VENDORED_IN]


def _strip_workspace_marker(cargo_toml_text: str) -> str:
    """Drop the trailing `# ...\\n[workspace]` block. See module docstring."""
    lines = cargo_toml_text.splitlines()
    if not lines or lines[-1].strip() != "[workspace]":
        return cargo_toml_text
    end = len(lines) - 1
    start = end
    while start > 0 and (lines[start - 1].startswith("#") or lines[start - 1].strip() == ""):
        start -= 1
    kept = lines[:start]
    while kept and kept[-1].strip() == "":
        kept.pop()
    return "\n".join(kept) + "\n"


def _source_files() -> list[Path]:
    """Canonical files to vendor, relative paths, deterministic order."""
    return sorted(
        p.relative_to(CANONICAL)
        for p in CANONICAL.rglob("*")
        if p.is_file() and not any(part in IGNORE_NAMES for part in p.relative_to(CANONICAL).parts)
    )


def _rendered_canonical(rel_path: Path) -> bytes:
    """Canonical file content as it should appear in a vendored copy."""
    src = CANONICAL / rel_path
    if rel_path.name == "Cargo.toml":
        return _strip_workspace_marker(src.read_text()).encode()
    return src.read_bytes()


def sync() -> int:
    files = _source_files()
    for dest_root in vendor_dirs():
        if dest_root.exists():
            for old in sorted(dest_root.rglob("*"), reverse=True):
                if old.is_file():
                    old.unlink()
            for old in sorted(dest_root.rglob("*"), reverse=True):
                if old.is_dir():
                    old.rmdir()
        for rel in files:
            dest = dest_root / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(_rendered_canonical(rel))
        print(f"synced {dest_root.relative_to(REPO_ROOT)} ({len(files)} files)")
    return 0


def check() -> int:
    if not CANONICAL.is_dir():
        print(f"error: canonical solana-core not found at {CANONICAL}", file=sys.stderr)
        return 1

    files = _source_files()
    overall = 0
    for dest_root in vendor_dirs():
        rel_root = dest_root.relative_to(REPO_ROOT)
        if not dest_root.is_dir():
            print(f"error: {rel_root} does not exist -- run `sync` first", file=sys.stderr)
            overall = 1
            continue

        errors = []
        for rel in files:
            dest = dest_root / rel
            if not dest.is_file():
                errors.append(f"missing: {rel}")
                continue
            if dest.read_bytes() != _rendered_canonical(rel):
                errors.append(f"content differs: {rel}")
        vendored_files = {
            p.relative_to(dest_root)
            for p in dest_root.rglob("*")
            if p.is_file() and not any(part in IGNORE_NAMES for part in p.relative_to(dest_root).parts)
        }
        for extra in sorted(vendored_files - set(files)):
            errors.append(f"unexpected extra file: {extra}")

        if errors:
            print(f"error: {rel_root} has drifted from solana-core/:", file=sys.stderr)
            for e in errors:
                print(f"  {e}", file=sys.stderr)
            overall = 1
        else:
            print(f"ok: {rel_root} matches solana-core/")
    return overall


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["sync", "check"])
    args = parser.parse_args(argv)
    return sync() if args.mode == "sync" else check()


if __name__ == "__main__":
    raise SystemExit(main())
