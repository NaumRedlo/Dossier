#!/usr/bin/env python3

import argparse
import hashlib
import json
import os
import pathlib
import io
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
import zipfile

MAX_BEATMAP_BYTES = 50 * 1024 * 1024
MIRRORS_MD5 = (
    "https://osu.direct/api/v2/md5/{hash}",
)

MIRRORS_OSZ = (
    "https://api.nerinyan.moe/d/{set_id}",
    "https://beatconnect.io/b/{set_id}",
    "https://osu.direct/d/{set_id}",
)

MAX_SET_BYTES = 200 * 1024 * 1024

BROWSER_UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
)

SOLO_REPLAY = re.compile(r"solo-replay-\w+_(\d+)_\d+")
TIMEOUT = 20

SET_TIMEOUT = 120
TRIES = 4
WAIT_MOST = 5.0
SPACING = 0.5
INSPECT_BATCH = 400

answers = {}
last_asked = {}

def wait_for(error):
    if not isinstance(error, urllib.error.HTTPError) or error.code != 429:
        return None
    try:
        asked = float(error.headers.get("Retry-After", "1"))
    except (TypeError, ValueError):
        asked = 1.0
    return min(max(asked, 1.0), WAIT_MOST)

def spaced(url):
    host = url.split("/")[2]
    gap = SPACING - (time.monotonic() - last_asked.get(host, float("-inf")))
    if gap > 0:
        time.sleep(gap)
    last_asked[host] = time.monotonic()

def patiently(url, open_once):
    for attempt in range(TRIES):
        spaced(url)
        try:
            return open_once()
        except urllib.error.HTTPError as error:
            wait = wait_for(error)
            if wait is None or attempt == TRIES - 1:
                raise
            time.sleep(wait)

def get(url):
    if url in answers:
        return answers[url]

    def once():
        request = urllib.request.Request(url, headers={"User-Agent": "dossier/0.1"})
        with urllib.request.urlopen(request, timeout=TIMEOUT) as response:
            return response.read(MAX_BEATMAP_BYTES + 1)

    body = patiently(url, once)
    answers[url] = body
    return body

def get_set(url):
    def once():
        request = urllib.request.Request(url, headers={"User-Agent": BROWSER_UA})
        with urllib.request.urlopen(request, timeout=SET_TIMEOUT) as response:
            return response.read(MAX_SET_BYTES + 1)

    return patiently(url, once)

def set_id_of(want_hash):
    why = "no mirror knows this hash"
    for mirror in MIRRORS_MD5:
        host = mirror.split("/")[2]
        try:
            entry = json.loads(get(mirror.format(hash=want_hash)))
        except urllib.error.HTTPError as error:
            if error.code != 404:
                why = f"{host}: {error}"
            continue
        except (urllib.error.URLError, OSError, TimeoutError) as error:
            why = f"{host}: {error}"
            continue
        except ValueError:
            continue
        if isinstance(entry, dict) and entry.get("beatmapset_id"):
            return int(entry["beatmapset_id"]), None
    return None, why

def keep_set(content, want_hash, into):
    if not content:
        return "empty"
    if len(content) > MAX_SET_BYTES:
        return f"over {MAX_SET_BYTES // (1024 * 1024)}MB"
    try:
        archive = zipfile.ZipFile(io.BytesIO(content))
        names = archive.namelist()
    except (zipfile.BadZipFile, OSError):
        return "not an archive"
    found = False
    for name in names:
        if not name.lower().endswith(".osu"):
            continue
        try:
            if hashlib.md5(archive.read(name)).hexdigest() == want_hash:
                found = True
                break
        except (zipfile.BadZipFile, OSError):
            continue
    if not found:
        return "the set does not hold this revision"
    temporary = into.with_suffix(".osz.part")
    temporary.write_bytes(content)
    os.replace(temporary, into)

    bare = into.with_suffix(".osu")
    if bare.exists():
        bare.unlink()
    return None

def keep_bare(content, want_hash, songs):
    if not content:
        return "empty", set()
    if len(content) > MAX_SET_BYTES:
        return f"over {MAX_SET_BYTES // (1024 * 1024)}MB", set()
    try:
        archive = zipfile.ZipFile(io.BytesIO(content))
        names = archive.namelist()
    except (zipfile.BadZipFile, OSError):
        return "not an archive", set()
    maps = {}
    for name in names:
        if not name.lower().endswith(".osu"):
            continue
        try:
            body = archive.read(name)
        except (zipfile.BadZipFile, OSError):
            continue
        if len(body) <= MAX_BEATMAP_BYTES:
            maps[hashlib.md5(body).hexdigest()] = body
    if want_hash not in maps:
        return "the set does not hold this revision", set()
    for digest, body in maps.items():
        target = songs / f"{digest}.osu"
        if target.exists():
            continue
        temporary = target.with_suffix(".osu.part")
        temporary.write_bytes(body)
        os.replace(temporary, target)
    return None, set(maps)

def hashes_in(path):
    try:
        archive = zipfile.ZipFile(path)
        return {hashlib.md5(archive.read(name)).hexdigest() for name in archive.namelist() if name.lower().endswith(".osu")}
    except (zipfile.BadZipFile, OSError):
        return set()

def fetch_set(want_hash, songs, bare=False):
    set_id, why = set_id_of(want_hash)
    if set_id is None:
        return why, set()
    why = "no mirror answered"
    for mirror in MIRRORS_OSZ:
        host = mirror.split("/")[2]
        try:
            content = get_set(mirror.format(set_id=set_id))
        except (urllib.error.URLError, OSError, TimeoutError) as error:
            why = f"{host}: {error}"
            continue
        if bare:
            problem, kept = keep_bare(content, want_hash, songs)
        else:
            target = songs / f"{want_hash}.osz"
            problem = keep_set(content, want_hash, target)
            kept = hashes_in(target) if problem is None else set()
        if problem is None:
            return None, kept
        why = f"{host}: {problem}"
    return f"set {set_id}: {why}", set()

def headers_of(dossier, replays):
    out = []
    for start in range(0, len(replays), INSPECT_BATCH):
        out.extend(subprocess.run(
            [dossier, "inspect", "--json", *[str(r) for r in replays[start:start + INSPECT_BATCH]]],
            capture_output=True,
            text=True,
        ).stdout.splitlines())
    wanted = {}
    for line in out:
        try:
            entry = json.loads(line)
        except ValueError:
            continue
        if entry.get("beatmap_hash"):
            wanted.setdefault(entry["beatmap_hash"], []).append(entry["replay"])
    return wanted

def on_disk(songs):
    have = set()
    for path in songs.rglob("*"):
        if path.suffix.lower() == ".osu":
            have.add(hashlib.md5(path.read_bytes()).hexdigest())
        elif path.suffix.lower() == ".osz":
            try:
                archive = zipfile.ZipFile(path)
            except (zipfile.BadZipFile, OSError):
                continue
            for name in archive.namelist():
                if name.lower().endswith(".osu"):
                    try:
                        have.add(hashlib.md5(archive.read(name)).hexdigest())
                    except (zipfile.BadZipFile, OSError):
                        continue
    return have

def beatmap_id(want_hash, replays):
    for replay in replays:
        found = SOLO_REPLAY.search(pathlib.Path(replay).name)
        if found:
            return int(found.group(1)), "name"
    unknown = True
    for mirror in MIRRORS_MD5:
        host = mirror.split("/")[2]
        try:
            body = get(mirror.format(hash=want_hash))
        except urllib.error.HTTPError as error:

            if error.code != 404:
                unknown = False
            continue
        except (urllib.error.URLError, OSError, TimeoutError):
            unknown = False
            continue
        try:
            entry = json.loads(body)
        except ValueError:
            continue
        if isinstance(entry, dict) and entry.get("id"):
            return int(entry["id"]), host
    return None, "hash on no mirror" if unknown else "mirrors unreachable"

def read_manifest(path):
    rows = []
    for line in path.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) == 7:
            rows.append(fields)
    return rows

def wanted_from_manifest(rows):
    wanted = {}
    pinned = {}
    for replay_md5, beatmap_md5, beatmap_id, _error, _combo, _score, name in rows:
        wanted.setdefault(beatmap_md5, []).append(name)
        if beatmap_id != "-":
            pinned[beatmap_md5] = int(beatmap_id)
    return wanted, pinned

def write_manifest(path, resolved):
    lines = []
    filled = 0
    for line in path.read_text().splitlines():
        fields = line.split("\t")
        if line.startswith("#") or len(fields) != 7 or fields[2] != "-":
            lines.append(line)
            continue
        found = resolved.get(fields[1])
        if found is None:
            lines.append(line)
            continue
        fields[2] = str(found)
        filled += 1
        lines.append("\t".join(fields))
    path.write_text("\n".join(lines) + "\n")
    return filled

def main():
    parser = argparse.ArgumentParser(
        description="Recover missing maps from mirror beatmapset archives, verified by MD5.",
        add_help=True,
    )
    parser.add_argument("replays", nargs="*", type=pathlib.Path)
    parser.add_argument(
        "--manifest",
        type=pathlib.Path,
        help="take the map list from the corpus manifest instead of from replay files, "
        "and pin back any beatmap id resolved along the way",
    )
    parser.add_argument(
        "--songs",
        type=pathlib.Path,
        default=os.environ.get("DOSSIER_SONGS_DIR"),
        help="where maps live, and where fetched ones are put",
    )
    parser.add_argument(
        "--dossier",
        default=os.environ.get("DOSSIER_BIN", "target/release/dossier"),
        help="the binary to read replay headers with",
    )
    parser.add_argument(
        "--with-audio",
        action="store_true",
        help="also replace existing bare .osu files with mirror beatmapsets containing audio. "
        "Missing maps are always fetched as .osz archives from mirrors.",
    )
    parser.add_argument(
        "--bare",
        action="store_true",
        help="keep only the .osu files of each fetched set, named by their MD5, and drop the archive",
    )
    parser.add_argument("--dry-run", action="store_true", help="say what is missing, fetch nothing")
    options = parser.parse_args()
    answers.clear()
    if not options.songs:
        parser.error("--songs is required (or set DOSSIER_SONGS_DIR)")
    songs = pathlib.Path(options.songs).expanduser()
    songs.mkdir(parents=True, exist_ok=True)

    pinned = {}
    if options.manifest:
        rows = read_manifest(options.manifest)
        wanted, pinned = wanted_from_manifest(rows)
        print(f"{len(rows)} rows name {len(wanted)} maps, {len(pinned)} with an id already")
    else:
        if not options.replays:
            parser.error("give replay paths, or --manifest")
        wanted = headers_of(options.dossier, options.replays)
        if not wanted:
            print("no replay headers could be read — is --dossier right?", file=sys.stderr)
            return 1
        replays = sum(len(r) for r in wanted.values())
        print(f"{replays} replays want {len(wanted)} maps")
    have = on_disk(songs)
    missing = {h: r for h, r in wanted.items() if h not in have}
    if options.with_audio:

        silent = {
            h: r
            for h, r in wanted.items()
            if h in have and not (songs / f"{h}.osz").exists()
        }
        missing.update(silent)
        print(f"{len(missing)} of {len(wanted)} are missing or silent")
    else:
        print(f"{len(missing)} of {len(wanted)} are missing")

    resolved = {}

    unpinned = {h: r for h, r in wanted.items() if h not in pinned}
    if options.manifest and not options.dry_run:
        for want_hash, names in sorted(unpinned.items()):
            if want_hash in missing:
                continue
            found, _how = beatmap_id(want_hash, names)
            if found is not None:
                resolved[want_hash] = found

    fetched, failed = 0, []
    for want_hash, for_replays in sorted(missing.items()):
        name = pathlib.Path(for_replays[0]).name
        if want_hash in have and not options.with_audio:
            fetched += 1
            print(f"  ok {want_hash}  came with an earlier set")
            continue
        if options.dry_run:
            print(f"  ?? {want_hash}  {name[:56]}")
            continue
        if want_hash in pinned:
            found, how = pinned[want_hash], "pinned"
        else:
            found, how = beatmap_id(want_hash, for_replays)
            if found is not None:
                resolved[want_hash] = found
        why, kept = fetch_set(want_hash, songs, options.bare)
        have |= kept
        if why:
            failed.append((want_hash, name, why))
            print(f"  !! {want_hash}  {why}", flush=True)
        else:
            fetched += 1
            print(f"  ok {want_hash}  {'mirror set, maps only' if options.bare else 'mirror set with audio'}", flush=True)

    if options.dry_run:
        return 0
    if options.manifest and resolved:
        filled = write_manifest(options.manifest, resolved)
        print(f"{filled} row(s) pinned to {len(resolved)} beatmap id(s) in {options.manifest}")
    print(f"{fetched} fetched, {len(failed)} still missing")
    return 0 if fetched or not failed else 1

if __name__ == "__main__":
    sys.exit(main())
