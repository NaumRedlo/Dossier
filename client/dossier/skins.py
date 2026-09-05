import hashlib
import json
import os
import re
import shutil
import subprocess
import time
import zipfile

from dossier.settings import DOSSIER_FFMPEG, MAX_SKIN_MB, SKIN_STORE_DIR
from dossier.log import get_logger

logger = get_logger("skins")

_preview = None

def draws_previews(hook) -> None:
    global _preview
    _preview = hook

class SkinRejected(RuntimeError):
    pass

MAX_UNPACKED_BYTES = MAX_SKIN_MB * 2 * 1024 * 1024

MAX_FILES = 2000

KEPT_SUFFIXES = (".png", ".jpg", ".wav", ".mp3", ".ogg", ".ini")

EXTRACT_VERSION = 1

STAMP = ".dossier-import.json"

def store_dir() -> str:
    return os.path.expanduser(SKIN_STORE_DIR)

def _safe_name(name: str) -> str:
    stem = os.path.splitext(os.path.basename(name or ""))[0]
    cleaned = re.sub(r"[^A-Za-z0-9 _.-]", "", stem).strip(" .")
    return (cleaned or "skin")[:48]

def available() -> list[str]:
    try:
        return sorted(
            entry.name
            for entry in os.scandir(store_dir())
            if entry.is_dir() and not entry.name.startswith(".")
        )
    except OSError:
        return []

def folder_of(name: str) -> str | None:
    if name in available():
        return os.path.join(store_dir(), name)
    return None

def forget(name: str) -> bool:
    folder = folder_of(name)
    if not folder:
        return False
    shutil.rmtree(folder, ignore_errors=True)
    return True

def import_osk(archive_path: str, filename: str, owner: int | None = None) -> str:
    os.makedirs(store_dir(), exist_ok=True)
    name = _safe_name(filename)
    destination = os.path.join(store_dir(), name)

    try:
        with zipfile.ZipFile(archive_path) as archive:
            entries = [item for item in archive.infolist() if not item.is_dir()]
            if len(entries) > MAX_FILES:
                raise SkinRejected(
                    f"в архиве {len(entries)} файлов — это не скин"
                )
            declared = sum(item.file_size for item in entries)
            if declared > MAX_UNPACKED_BYTES:
                raise SkinRejected(
                    f"скин распакуется в {declared // 1024 // 1024} МБ, "
                    f"а больше {MAX_UNPACKED_BYTES // 1024 // 1024} МБ мы не берём"
                )

            staging = destination + ".incoming"
            shutil.rmtree(staging, ignore_errors=True)
            os.makedirs(staging, exist_ok=True)
            try:
                written = _extract(archive, entries, staging)
                _to_wav(staging)
            except Exception:
                shutil.rmtree(staging, ignore_errors=True)
                raise
    except zipfile.BadZipFile as exc:
        raise SkinRejected(f"файл не читается как архив: {exc}") from exc

    if written == 0:
        shutil.rmtree(staging, ignore_errors=True)
        raise SkinRejected("в архиве нет ничего, что движок умеет читать")

    _write_stamp(staging, filename, written, owner)

    shutil.rmtree(destination, ignore_errors=True)
    os.replace(staging, destination)
    logger.info("imported skin %s: %d file(s)", name, written)

    if _preview is not None:
        try:
            _preview(name, rebuild=True)
        except Exception as exc:
            logger.warning("no preview for %s: %s", name, exc)
    return name

def _write_stamp(folder: str, filename: str, written: int, owner: int | None = None) -> None:
    body = {
        "extract_version": EXTRACT_VERSION,
        "source": filename,
        "files": written,
        "at": int(time.time()),
    }
    if owner is not None:
        body["owner"] = owner
    try:
        with open(os.path.join(folder, STAMP), "w", encoding="utf-8") as handle:
            json.dump(body, handle, ensure_ascii=False, indent=1)
    except OSError:

        logger.warning("could not stamp %s", folder)

def stamp_of(folder: str) -> dict:
    try:
        with open(os.path.join(folder, STAMP), encoding="utf-8") as handle:
            body = json.load(handle)
    except (OSError, ValueError):
        return {"extract_version": 0}
    if not isinstance(body, dict):
        return {"extract_version": 0}
    body.setdefault("extract_version", 0)
    return body

def owner_of(name: str) -> int | None:
    folder = folder_of(name)
    if not folder:
        return None
    owner = stamp_of(folder).get("owner")
    return owner if isinstance(owner, int) else None

def by_owner(tg_id: int | None) -> tuple[list[str], list[str]]:
    mine, shared = [], []
    for name in available():
        (mine if tg_id is not None and owner_of(name) == tg_id else shared).append(name)
    return mine, shared

def is_stale(name: str) -> bool:
    folder = folder_of(name)

    if folder is None or not os.path.isdir(folder):
        return False
    return int(stamp_of(folder).get("extract_version") or 0) < EXTRACT_VERSION

def stale() -> list[str]:
    return [name for name in available() if is_stale(name)]

def _extract(archive: zipfile.ZipFile, entries, into: str) -> int:
    root = os.path.realpath(into)
    total = 0
    written = 0

    ordered = sorted(entries, key=lambda item: item.filename.replace("\\", "/").count("/"))
    for item in ordered:
        leaf = os.path.basename(item.filename.replace("\\", "/"))
        if not leaf or not leaf.lower().endswith(KEPT_SUFFIXES):
            continue
        nested = "/" in item.filename.replace("\\", "/")
        if nested and os.path.exists(os.path.join(root, leaf)):

            continue
        target = os.path.realpath(os.path.join(root, leaf))
        if os.path.commonpath([root, target]) != root:

            raise SkinRejected("в архиве путь, ведущий за пределы папки")

        with archive.open(item) as source, open(target, "wb") as sink:
            while chunk := source.read(1 << 16):
                total += len(chunk)
                if total > MAX_UNPACKED_BYTES:

                    raise SkinRejected("архив распаковывается больше, чем обещал")
                sink.write(chunk)
        written += 1
    return written

FOREIGN_AUDIO = (".ogg", ".mp3")

def _readable_wav(path: str) -> bool:
    try:
        with open(path, "rb") as handle:
            head = handle.read(12)
            if len(head) < 12 or head[0:4] != b"RIFF" or head[8:12] != b"WAVE":
                return False

            while True:
                header = handle.read(8)
                if len(header) < 8:
                    break
                size = int.from_bytes(header[4:8], "little")
                if header[0:4] == b"fmt " and size >= 16:
                    body = handle.read(size + (size & 1))
                    if len(body) < 16:
                        return False
                    tag = int.from_bytes(body[0:2], "little")
                    channels = int.from_bytes(body[2:4], "little")
                    bits = int.from_bytes(body[14:16], "little")
                    return tag == 1 and channels in (1, 2) and bits in (8, 16)
                handle.seek(size + (size & 1), os.SEEK_CUR)
    except OSError:
        return False
    return False

_AUDIO_MAGIC = {
    ".ogg": (b"OggS",),
    ".oga": (b"OggS",),
    ".mp3": (b"ID3",),
}

_SHORTEST_AUDIO = 27

def _has_bytes(path: str) -> bool:
    try:
        size = os.path.getsize(path)
        if size < _SHORTEST_AUDIO:
            return False
        wanted = _AUDIO_MAGIC.get(os.path.splitext(path)[1].lower())
        if wanted is None:
            return True
        with open(path, "rb") as handle:
            head = handle.read(4)
        if any(head.startswith(magic) for magic in wanted):
            return True

        return (
            os.path.splitext(path)[1].lower() == ".mp3"
            and len(head) >= 2
            and head[0] == 0xFF
            and (head[1] & 0xE0) == 0xE0
        )
    except OSError:
        return False

def _unconverted(folder: str) -> list[tuple[str, str]]:
    work = []
    try:
        leaves = sorted(os.listdir(folder))
    except OSError:
        return work
    for leaf in leaves:
        path = os.path.join(folder, leaf)
        lower = leaf.lower()
        if lower.endswith(FOREIGN_AUDIO):
            target = os.path.join(folder, os.path.splitext(leaf)[0] + ".wav")

            if not os.path.exists(target) and _has_bytes(path):
                work.append((path, target))
        elif lower.endswith(".wav") and _has_bytes(path) and not _readable_wav(path):
            work.append((path, path))
    return work

def _to_wav(folder: str) -> None:
    refused: list[tuple[str, str]] = []
    for source, target in _unconverted(folder):
        leaf = os.path.basename(source)

        in_place = source == target
        written = target + ".converting" if in_place else target
        try:
            done = subprocess.run(
                [DOSSIER_FFMPEG, "-nostdin", "-v", "error", "-y", "-i", source,
                 "-ac", "2", "-ar", "44100", "-c:a", "pcm_s16le", "-f", "wav",
                 written],
                capture_output=True,
                timeout=30,
            )
        except (OSError, subprocess.SubprocessError) as exc:
            logger.warning("could not convert %s: %s", leaf, exc)
            return
        if done.returncode == 0 and in_place:
            os.replace(written, target)
        if done.returncode != 0:

            said = done.stderr.decode("utf-8", "replace").strip().splitlines()
            refused.append((leaf, said[0][:160] if said else "no reason given"))

            if os.path.exists(written) and not (in_place and written == target):
                os.remove(written)

    if refused:

        names = ", ".join(leaf for leaf, _ in refused[:6])
        more = f" and {len(refused) - 6} more" if len(refused) > 6 else ""
        logger.warning(
            "ffmpeg would not take %d sample(s) in %s — %s%s: %s",
            len(refused), os.path.basename(folder), names, more, refused[0][1],
        )

def convert_folder(folder: str) -> int:
    work = _unconverted(folder)
    if work:
        _to_wav(folder)
    return len(work)

def convert_stored() -> int:
    store = store_dir()
    if not os.path.isdir(store):
        return 0
    touched = 0
    for name in sorted(os.listdir(store)):
        folder = os.path.join(store, name)
        if not os.path.isdir(folder):
            continue

        if not _unconverted(folder):
            continue
        _to_wav(folder)
        touched += 1
    if touched:
        logger.info("converted samples in %d stored skin(s)", touched)
    return touched

def packed(name: str) -> tuple[str, str] | None:
    folder = folder_of(name)
    if not folder:
        return None
    files = sorted(
        (entry.name, entry.stat().st_mtime)
        for entry in os.scandir(folder)
        if entry.is_file()
    )
    if not files:
        return None

    archive = os.path.join(store_dir(), f".{name}.zip")
    newest = max(mtime for _, mtime in files)
    if not os.path.exists(archive) or os.path.getmtime(archive) < newest:
        staging = archive + ".building"
        with zipfile.ZipFile(staging, "w", zipfile.ZIP_DEFLATED) as out:
            for leaf, _ in files:
                out.write(os.path.join(folder, leaf), leaf)
        os.replace(staging, archive)

    digest = hashlib.sha256()
    with open(archive, "rb") as handle:
        while chunk := handle.read(1 << 20):
            digest.update(chunk)
    return archive, digest.hexdigest()[:16]
