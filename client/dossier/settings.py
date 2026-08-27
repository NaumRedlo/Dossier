"""What this package needs to know, read from where it can read it.

Ten values, all environment variables, all with a default that is right for
somebody who has set nothing. This is what let the bridge leave the bot: it
used to read `config.settings` — the bot's own module, fifty-odd values of
which ten were these — so a render client on a laptop took the bot's whole
configuration to learn where ffmpeg is.

The bot re-exports these rather than declaring them again. One definition,
because two would disagree, and the disagreement would be about which binary
to run.
"""

import os
import shutil
import sys

# Where this file is, up three: `client/dossier/settings.py` to the checkout
# it lives in, which is also where cargo writes. So a checkout that has been
# built is a checkout that works, with nothing to configure.
_HERE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# `.exe` on Windows, because cargo writes one and looking for the other is how
# a successful build reports itself as missing.
_ENGINE = "dossier.exe" if os.name == "nt" else "dossier"

def _find_engine() -> str:
    """Where the engine is, for somebody who has not said where.

    A built checkout is the render client's case, and it should need no
    configuration at all — so the binary beside this package wins. Installed
    with `pip` there is no checkout above it, only the virtual environment, and
    `.../venv/target/release/dossier` is a path that has never existed; `PATH`
    is asked instead.

    If neither answers, the checkout path is returned anyway. A message that
    says the engine is not at `<checkout>/target/release/dossier` tells
    somebody to build it; one that says the engine is nowhere tells them
    nothing.
    """
    beside = os.path.join(_HERE, "target", "release", _ENGINE)
    if os.path.isfile(beside):
        return beside
    return shutil.which(_ENGINE) or beside


# The compiled engine. Set explicitly when it lives somewhere else — which is
# the ordinary case for the bot, whose checkout is not this one.
DOSSIER_BIN = os.getenv("DOSSIER_BIN") or _find_engine()

# The encoder the engine shells out to.
DOSSIER_FFMPEG = os.getenv("DOSSIER_FFMPEG", "ffmpeg")
DOSSIER_CRF = os.getenv("DOSSIER_CRF", "20")
DOSSIER_PRESET = os.getenv("DOSSIER_PRESET", "veryfast")
# Empty leaves the thread count to ffmpeg, which sizes it from the machine.
DOSSIER_ENCODER_THREADS = os.getenv("DOSSIER_ENCODER_THREADS", "")

# Which look the engine renders in when nobody chose a skin.
DOSSIER_SKIN = os.getenv("DOSSIER_SKIN", "classic")
# osu!'s own hit sounds, for what a skin leaves out. Harmless unset.
DOSSIER_GAME_SOUNDS = os.getenv("DOSSIER_GAME_SOUNDS", "")

# Where maps are kept. `DANSER_SONGS_DIR` is honoured for deployments that
# predate this project's own name for it.
BEATMAP_STORE_DIR = os.getenv(
    "BEATMAP_STORE_DIR",
    os.getenv("DANSER_SONGS_DIR", os.path.expanduser("~/.osu/Songs")),
)

# Where imported skins are unpacked, one folder each.
SKIN_STORE_DIR = os.getenv("SKIN_STORE_DIR", os.path.expanduser("~/.dossier/skins"))
# The largest `.osk` this deployment will take, in megabytes.
MAX_SKIN_MB = int(os.getenv("MAX_SKIN_MB", "128"))

__all__ = [name for name in dir() if name.isupper() and not name.startswith("_")]

del sys
