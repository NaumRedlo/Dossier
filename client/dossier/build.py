import asyncio
import shutil
from typing import Optional

from dossier.settings import DOSSIER_BIN
from dossier.log import get_logger

logger = get_logger("build")

UNKNOWN = "unknown"

_cached: Optional[str] = None

async def local(*, refresh: bool = False) -> Optional[str]:
    global _cached
    if _cached is not None and not refresh:
        return _cached

    binary = shutil.which(DOSSIER_BIN) or DOSSIER_BIN
    try:
        process = await asyncio.create_subprocess_exec(
            binary,
            "--version",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.DEVNULL,
        )
        out, _ = await asyncio.wait_for(process.communicate(), 10)
    except (OSError, asyncio.TimeoutError):
        logger.warning("engine build: %s could not be asked its version", binary)
        return None
    if process.returncode != 0:

        logger.warning("engine build: %s does not answer --version", binary)
        return None

    _cached = out.decode(errors="replace").strip() or None
    return _cached

def build_of(version: Optional[str]) -> str:
    if not version:
        return UNKNOWN
    start = version.rfind("(")
    end = version.rfind(")")
    if start == -1 or end < start:
        return UNKNOWN
    return version[start + 1 : end].strip() or UNKNOWN

def agree(ours: Optional[str], theirs: Optional[str]) -> tuple[bool, str]:
    mine, yours = build_of(ours), build_of(theirs)
    if mine == UNKNOWN or yours == UNKNOWN:
        return True, "одна из двух сборок не может назвать себя"

    if mine == yours:
        if mine.endswith("+"):

            return True, f"обе — {mine}, собраны из правленого дерева"
        return True, f"обе — {mine}"

    if mine.rstrip("+") == yours.rstrip("+"):

        edited = "воркер" if yours.endswith("+") else "бот"
        return False, (
            f"обе на {mine.rstrip('+')}, но {edited} собрал свой двоичник "
            f"из правленого дерева — пересоберите там, прежде закоммитив "
            f"или отложив всё несохранённое в dossier/crates"
        )

    return False, (
        f"бот рисует сборкой {mine}, а воркер — {yours}: `git pull`, затем "
        f"`cargo build --release` на той стороне, что отстала"
    )

__all__ = ["local", "build_of", "agree", "UNKNOWN"]
