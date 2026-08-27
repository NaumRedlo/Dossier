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

def _next_to_the_program() -> str:
    """The directory the running program is in, whatever kind of program it is.

    A release is a folder somebody unzipped: the engine and the client sit side
    by side in it, and there is no checkout, no virtual environment and no
    `PATH` entry. Frozen into one executable, `__file__` points inside a
    temporary directory that gets deleted, so `sys.executable` is the only
    thing that still names where the user actually put this.
    """
    if getattr(sys, "frozen", False):
        return os.path.dirname(os.path.abspath(sys.executable))
    # Run as a script from a checkout: `client/worker.py` or `python -m`.
    main = getattr(sys.modules.get("__main__"), "__file__", None)
    return os.path.dirname(os.path.abspath(main)) if main else os.getcwd()


def _find_engine() -> str:
    """Where the engine is, for somebody who has not said where.

    Three places, in the order of how sure each one is:

    **Next to the program.** This is a release — a folder somebody unzipped,
    with `dossier` and the client in it together. Nothing is configured and
    nothing should have to be; it is the case with the least room to guess
    wrong, so it is asked first.

    **The checkout.** `<repo>/target/release/dossier`, which is where cargo
    writes. A built clone works with no configuration either.

    **`PATH`.** Installed with `pip` there is no checkout above the package,
    only the virtual environment, and `.../venv/target/release/dossier` is a
    path that has never existed anywhere.

    If none of them answers, the checkout path is returned regardless. A
    message that says the engine is not at `<checkout>/target/release/dossier`
    tells somebody to build it; one that says the engine is nowhere tells them
    nothing.
    """
    beside = os.path.join(_HERE, "target", "release", _ENGINE)
    unpacked = os.path.join(_next_to_the_program(), _ENGINE)
    for candidate in (unpacked, beside):
        if os.path.isfile(candidate):
            return candidate
    return shutil.which(_ENGINE) or beside


# The compiled engine. Set explicitly when it lives somewhere else — which is
# the ordinary case for the bot, whose checkout is not this one.
DOSSIER_BIN = os.getenv("DOSSIER_BIN") or _find_engine()

def _find_font() -> str:
    """The typeface the HUD and the combo numbers are set in.

    The engine looks for `assets/fonts/` relative to its *working directory*
    and draws the play without numbers when it finds nothing — which is a
    render that looks finished and is wrong, reported by nobody. That is fine
    while the working directory is always the checkout, and it stops being fine
    the moment somebody runs an unpacked release from anywhere else.

    So the client names the file outright rather than leaving the engine to
    guess: next to the program for a release, then the checkout. Empty when
    there is no font to be found, which leaves the engine's own search exactly
    as it was.
    """
    here = os.path.join("assets", "fonts", "TorusNotched-Bold.ttf")
    for root in (_next_to_the_program(), _HERE):
        found = os.path.join(root, here)
        if os.path.isfile(found):
            return found
    return ""


# Named rather than searched for; see above. The engine takes it from the
# environment, and `runner` puts it there.
DOSSIER_FONT = os.getenv("DOSSIER_FONT") or _find_font()

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

# Every capital in this module and nothing else — which is what keeps `__all__`
# from having to be kept in step by hand. `sys` and `os` are lower case and so
# is `shutil`, and `_find_engine` starts with an underscore.
__all__ = [name for name in dir() if name.isupper() and not name.startswith("_")]
