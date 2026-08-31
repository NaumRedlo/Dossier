"""Fetching the release the bot is on, when this machine is on another one.

A worker whose engine differs from the bot's is turned away, and rightly: a
stale binary renders something that looks right and is not. What it was told to
do about that was `git pull` and `cargo build` — in a checkout most people
running this do not have, because they downloaded a zip.

The bot now says which release it is on, and this fetches it.

## Where it goes, and why not on top of itself

Into `~/.dossier/engines/<tag>-<platform>`, and the program relaunches from
there. Not over the folder it is running out of, which sounds tidier and is
where this kind of thing goes wrong: a running program cannot overwrite its own
executable on Windows at all, and a half-replaced folder on any system is a
worker that starts and then cannot render.

The old folder stays. Somebody's shortcut goes on pointing at it, and that is
fine — the old copy starts, finds the same mismatch, and hands over again. It
heals rather than needing to be tidied.
"""

import hashlib
import os
import ssl
import platform
import shutil
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
import zipfile

from dossier.log import get_logger

logger = get_logger("update")

REPO = "NaumRedlo/Dossier"
HOME = os.path.expanduser("~/.dossier")
ENGINES = os.path.join(HOME, "engines")

# Large enough for a release and small enough that a redirect to something else
# is caught rather than written to disk.
MOST_BYTES = 200 * 1024 * 1024

# Set on the program we hand over to, so a fault that leaves the two disagreeing
# cannot become two processes replacing each other for ever.
HANDED_OVER = "DOSSIER_HANDED_OVER"


def trusted() -> "ssl.SSLContext | None":
    """The certificate authorities this machine should believe, for HTTPS.

    Reported from a friend's Mac, against a server every browser is happy with:

        [SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed:
        unable to get local issuer certificate

    Python does not read the system's certificate store on macOS — it asks
    OpenSSL, which looks in a directory that a Homebrew or python.org install
    leaves empty. `requests` never showed this because it carries `certifi` and
    uses it by default; `aiohttp` builds a default context and inherits the
    hole. So the same authorities are handed to both — `aiohttp`, which the
    worker talks to the bot with, and `urllib` here, which fetches the
    release. This lived beside the first of those and the second inherited
    nothing, so a Mac that could reach the bot could not update from it:
    the same error, at the one moment it stops everything.

    Frozen into one executable it matters more rather than less: there is no
    system Python whose install script somebody might once have run.

    `None` — meaning aiohttp's own default — if `certifi` is somehow absent,
    because a machine whose OpenSSL *is* set up works fine that way and
    refusing to start would be the worse answer.
    """
    try:
        import certifi
    except ImportError:
        return None
    return ssl.create_default_context(cafile=certifi.where())

class Cannot(RuntimeError):
    """Said to a person, not to a traceback."""


def slug() -> str:
    """Which release this machine can run, named the way the release names it."""
    machine = platform.machine().lower()
    if sys.platform.startswith("linux") and machine in ("x86_64", "amd64"):
        return "linux-x64"
    if sys.platform == "darwin" and machine in ("arm64", "aarch64"):
        return "macos-arm64"
    if sys.platform == "win32" and machine in ("x86_64", "amd64"):
        return "windows-x64"
    raise Cannot(
        f"для {sys.platform}/{machine} готовых сборок нет — только linux-x64, "
        f"macos-arm64 и windows-x64. Собери из исходников: "
        f"github.com/{REPO}"
    )


def from_a_checkout() -> bool:
    """Whether this is running from source rather than from a release.

    Updating a checkout by downloading a zip over it would replace somebody's
    working copy with a build, which is not what they want and not something to
    do without being asked very clearly. They get told to pull instead.
    """
    return not getattr(sys, "frozen", False)


def already_handed_over() -> bool:
    return os.environ.get(HANDED_OVER) == "1"


def _fetch(url: str) -> bytes:
    try:
        with urllib.request.urlopen(url, timeout=120, context=trusted()) as reply:  # noqa: S310
            body = reply.read(MOST_BYTES + 1)
    except urllib.error.HTTPError as exc:
        if exc.code == 404:
            raise Cannot(f"в релизе нет файла для этой системы:\n  {url}") from exc
        raise Cannot(f"{url}: {exc}") from exc
    except urllib.error.URLError as exc:
        raise Cannot(f"не удалось скачать: {exc.reason}") from exc
    if len(body) > MOST_BYTES:
        raise Cannot("скачанное больше, чем может быть релизом")
    return body


def fetch(tag: str, *, say=print) -> str:
    """Download the release, check it, unpack it. Returns where it landed.

    The hash is published beside the archive by the job that built it, and it
    is checked before a single byte is unpacked. This runs on somebody's own
    computer, unattended, and "it downloaded" is not "it downloaded the right
    thing".
    """
    named = f"dossier-{tag}-{slug()}"
    base = f"https://github.com/{REPO}/releases/download/{tag}"

    say(f"  беру {named}.zip…")
    body = _fetch(f"{base}/{named}.zip")

    said = _fetch(f"{base}/{named}.zip.sha256").decode("utf-8", "replace").split()
    expected = said[0] if said else ""
    got = hashlib.sha256(body).hexdigest()
    if not expected:
        raise Cannot("рядом с архивом нет контрольной суммы — нечем проверить")
    if got != expected:
        raise Cannot(
            f"скачанное не совпало с опубликованной суммой:\n"
            f"    ожидалось {expected}\n    получилось {got}\n"
            f"  Ничего не распаковано."
        )
    say(f"  сумма сошлась: {got[:16]}…")

    os.makedirs(ENGINES, exist_ok=True)
    landing = os.path.join(ENGINES, f"{tag}-{slug()}")
    staging = landing + ".unpacking"
    shutil.rmtree(staging, ignore_errors=True)

    with tempfile.NamedTemporaryFile(suffix=".zip", delete=False) as handle:
        handle.write(body)
        temporary = handle.name
    try:
        with zipfile.ZipFile(temporary) as archive:
            # Into staging and then swapped whole, so a folder that exists is a
            # folder that is finished. An interrupted download must not leave
            # something this program could then hand over to.
            archive.extractall(staging)
    finally:
        os.unlink(temporary)

    inner = [name for name in os.listdir(staging)
             if os.path.isdir(os.path.join(staging, name))]
    made = os.path.join(staging, inner[0]) if len(inner) == 1 else staging

    shutil.rmtree(landing, ignore_errors=True)
    shutil.move(made, landing)
    shutil.rmtree(staging, ignore_errors=True)

    for name in ("dossier", "dossier-worker", "dossier.exe", "dossier-worker.exe"):
        binary = os.path.join(landing, name)
        if os.path.isfile(binary):
            os.chmod(binary, 0o755)
    say(f"  распаковано: {landing}")
    return landing


def hand_over(landing: str) -> None:
    """Start the new copy and stop being the old one.

    Two ways, because a console is not the same thing on both.

    Everywhere but Windows this replaces the process: same terminal, same
    place in whatever started it, and nothing to notice but a program that
    knows more than it did a moment ago.

    On Windows there is no replacing a process, and a console made for a
    double-clicked program is destroyed when that program ends — so a child
    sharing it would lose its window the instant this one exits. It gets a
    console of its own instead. A second window appearing is odd to see once;
    a window that vanishes is a program that broke.
    """
    name = "dossier-worker.exe" if os.name == "nt" else "dossier-worker"
    binary = os.path.join(landing, name)
    if not os.path.isfile(binary):
        raise Cannot(f"в распакованном нет {name} — брать нечего")

    environment = {**os.environ, HANDED_OVER: "1"}
    arguments = [binary, *sys.argv[1:]]
    logger.info("handing over to %s", binary)

    if sys.platform == "win32":
        CREATE_NEW_CONSOLE = 0x00000010
        subprocess.Popen(  # noqa: S603 — a file this program just verified
            arguments, env=environment, creationflags=CREATE_NEW_CONSOLE,
        )
        raise SystemExit(0)

    os.execve(binary, arguments, environment)


def where_to_get_it(tag: str) -> str:
    """The address, for a worker with nobody at the keyboard to ask.

    A service cannot answer a question, and updating one behind its owner's
    back is not something to do. It gets told exactly what to download instead
    — which is still the whole of what `git pull` never was.
    """
    try:
        named = f"dossier-{tag}-{slug()}.zip"
    except Cannot:
        return f"https://github.com/{REPO}/releases/tag/{tag}"
    return f"https://github.com/{REPO}/releases/download/{tag}/{named}"
