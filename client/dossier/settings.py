import os
import shutil
import sys

_HERE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

_ENGINE = "dossier.exe" if os.name == "nt" else "dossier"

def _next_to_the_program() -> str:
    if getattr(sys, "frozen", False):
        return os.path.dirname(os.path.abspath(sys.executable))

    main = getattr(sys.modules.get("__main__"), "__file__", None)
    return os.path.dirname(os.path.abspath(main)) if main else os.getcwd()

def _find_engine() -> str:
    beside = os.path.join(_HERE, "target", "release", _ENGINE)
    unpacked = os.path.join(_next_to_the_program(), _ENGINE)
    for candidate in (unpacked, beside):
        if os.path.isfile(candidate):
            return candidate
    return shutil.which(_ENGINE) or beside

DOSSIER_BIN = os.getenv("DOSSIER_BIN") or _find_engine()

def _find_font() -> str:
    here = os.path.join("assets", "fonts", "VarelaRound-Regular.ttf")
    for root in (_next_to_the_program(), _HERE):
        found = os.path.join(root, here)
        if os.path.isfile(found):
            return found
    return ""

DOSSIER_FONT = os.getenv("DOSSIER_FONT") or _find_font()

DOSSIER_FFMPEG = os.getenv("DOSSIER_FFMPEG", "ffmpeg")
DOSSIER_CRF = os.getenv("DOSSIER_CRF", "20")
DOSSIER_PRESET = os.getenv("DOSSIER_PRESET", "veryfast")

DOSSIER_ENCODER_THREADS = os.getenv("DOSSIER_ENCODER_THREADS", "")

DOSSIER_SKIN = os.getenv("DOSSIER_SKIN", "classic")

DOSSIER_GAME_SOUNDS = os.getenv("DOSSIER_GAME_SOUNDS", "")

BEATMAP_STORE_DIR = os.getenv(
    "BEATMAP_STORE_DIR",
    os.getenv("DANSER_SONGS_DIR", os.path.expanduser("~/.osu/Songs")),
)

SKIN_STORE_DIR = os.getenv("SKIN_STORE_DIR", os.path.expanduser("~/.dossier/skins"))

MAX_SKIN_MB = int(os.getenv("MAX_SKIN_MB", "128"))

__all__ = [name for name in dir() if name.isupper() and not name.startswith("_")]
