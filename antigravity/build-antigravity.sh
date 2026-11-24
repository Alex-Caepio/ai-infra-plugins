#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

exec python3 - "${REPO_ROOT}" "$@" <<'PY'
import argparse
import hashlib
import os
import pathlib
import shutil
import sys
import zipfile

PLUGIN_BINS = {
    "ai-infra-mod": (
        "plugins/ai-infra-mod/bin",
        [
            "ai-infra-mod",
            "graph-map",
            "orch-engine",
            "context-graph-engine",
            "skill-foundry",
            "guardrails",
            "skillcache-engine",
        ],
    ),
    "graph-map": ("plugins/graph-map/bin", ["graph-map"]),
    "system-orchestration": ("plugins/system-orchestration/bin", ["orch-engine"]),
    "context-graph": ("plugins/context-graph/bin", ["context-graph-engine"]),
    "skill-foundry": ("plugins/skill-foundry/bin", ["skill-foundry"]),
    "agent-guardrails": ("plugins/agent-guardrails/bin", ["guardrails"]),
    "skillcache": ("plugins/skillcache/bin", ["skillcache-engine"]),
}

FIXED_ZIP_TIME = (2026, 1, 1, 0, 0, 0)


def parse_args(argv):
    parser = argparse.ArgumentParser(
        prog="build-antigravity.sh",
        description="Install self-contained Antigravity Extensions and/or build reproducible release .zip archives.",
    )
    parser.add_argument(
        "--install",
        nargs="?",
        const="__DEFAULT__",
        default=None,
        metavar="DIR",
        help="Install extensions into DIR (defaults to ~/.gemini/config/plugins). Never modifies config.json.",
    )
    parser.add_argument(
        "--dist-dir",
        default=None,
        metavar="DIR",
        help="Write reproducible <name>-antigravity.zip archives into DIR.",
    )
    parser.add_argument(
        "--plugin",
        action="append",
        choices=sorted(PLUGIN_BINS.keys()),
        help="Optional plugin filter (repeatable; defaults to all 7 plugins).",
    )
    args = parser.parse_args(argv)
    if args.install is None and args.dist_dir is None:
        parser.error("Specify --install [DIR] and/or --dist-dir DIR.")
    return args


def collect_plugin_entries(repo_root: pathlib.Path, name: str):
    src_dir = repo_root / "antigravity" / name
    if not src_dir.is_dir():
        raise FileNotFoundError(f"Missing Antigravity extension directory: {src_dir}")
    bin_rel_dir, bin_names = PLUGIN_BINS[name]
    bin_dir = repo_root / bin_rel_dir

    entries = []
    for path in sorted(src_dir.rglob("*")):
        if path.is_file():
            rel = path.relative_to(src_dir).as_posix()
            if rel.startswith("bin/"):
                continue
            entries.append((rel, path, False))

    for bname in sorted(bin_names):
        bpath = bin_dir / bname
        if not bpath.is_file():
            raise FileNotFoundError(f"Missing binary for {name}: {bpath}")
        entries.append((f"bin/{bname}", bpath, True))

    entries.sort(key=lambda item: item[0])
    return entries


def install_plugin(dest_root: pathlib.Path, name: str, entries):
    dest_root.mkdir(parents=True, exist_ok=True)
    target = dest_root / name
    staging = dest_root / f".{name}.staging.{os.getpid()}"
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir(parents=True)
    try:
        for rel, src_path, is_exec in entries:
            dst = staging / rel
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src_path, dst)
            os.chmod(dst, 0o755 if is_exec else 0o644)
        if target.exists():
            shutil.rmtree(target)
        staging.rename(target)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    print(f"Installed {name} -> {target}")


def build_zip(dist_dir: pathlib.Path, name: str, entries):
    dist_dir.mkdir(parents=True, exist_ok=True)
    archive = dist_dir / f"{name}-antigravity.zip"
    staging = dist_dir / f".{name}-antigravity.zip.tmp.{os.getpid()}"
    try:
        with zipfile.ZipFile(staging, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as zf:
            for rel, src_path, is_exec in entries:
                zinfo = zipfile.ZipInfo(rel, date_time=FIXED_ZIP_TIME)
                zinfo.create_system = 3  # Unix permissions
                mode = 0o100755 if is_exec else 0o100644
                zinfo.external_attr = mode << 16
                zinfo.compress_type = zipfile.ZIP_DEFLATED
                zf.writestr(zinfo, src_path.read_bytes())
        staging.replace(archive)
    finally:
        if staging.exists():
            staging.unlink()

    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    size = archive.stat().st_size
    print(f"{digest}  {archive.name} ({size} bytes)")
    return archive, digest, size


def main():
    repo_root = pathlib.Path(sys.argv[1])
    args = parse_args(sys.argv[2:])
    selected = args.plugin if args.plugin else list(PLUGIN_BINS.keys())

    install_dir = None
    if args.install is not None:
        install_dir = (
            pathlib.Path.home() / ".gemini" / "config" / "plugins"
            if args.install == "__DEFAULT__"
            else pathlib.Path(args.install).expanduser()
        )
    dist_dir = pathlib.Path(args.dist_dir).expanduser() if args.dist_dir else None

    for name in selected:
        entries = collect_plugin_entries(repo_root, name)
        if install_dir is not None:
            install_plugin(install_dir, name, entries)
        if dist_dir is not None:
            build_zip(dist_dir, name, entries)


if __name__ == "__main__":
    main()
PY
