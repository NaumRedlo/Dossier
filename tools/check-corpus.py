#!/usr/bin/env python3
"""Check every replay below a corpus directory without rewriting expectations."""

import argparse
from pathlib import Path
import subprocess
import sys


def main():
    repo = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path, help="directory containing maps and replays")
    parser.add_argument("--songs", type=Path, help="separate map directory, if needed")
    parser.add_argument("--binary", type=Path, default=repo / "target/release/dossier")
    parser.add_argument("--manifest", type=Path, default=repo / "tools/corpus.tsv")
    parser.add_argument("--threads", type=int, default=2)
    args = parser.parse_args()
    root = args.root.expanduser().resolve()
    binary = args.binary.expanduser().resolve()
    songs = (args.songs or root).expanduser().resolve()
    manifest = args.manifest.expanduser().resolve()
    for directory in (root, songs):
        if not directory.is_dir():
            parser.error(f"directory not found: {directory}")
    if not binary.is_file():
        parser.error("build the checker first: cargo build --locked --release -p dossier-cli")
    if not manifest.is_file():
        parser.error(f"expectation file not found: {manifest}")
    if args.threads < 1:
        parser.error("--threads must be positive")
    replays = sorted(p for p in root.rglob("*") if p.is_file() and p.suffix.lower() == ".osr")
    if not replays:
        parser.error(f"no replays found below {root}")
    command = [str(binary), "corpus", "--songs", str(songs), "--expect", str(manifest),
               "--strict", "--threads", str(args.threads), *map(str, replays)]
    try:
        return subprocess.run(command, cwd=repo, check=False).returncode
    except OSError as error:
        print(f"could not run corpus check: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
