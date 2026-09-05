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

MOST_BYTES = 200 * 1024 * 1024

HANDED_OVER = "DOSSIER_HANDED_OVER"

def trusted() -> "ssl.SSLContext | None":
    try:
        import certifi
    except ImportError:
        return None
    return ssl.create_default_context(cafile=certifi.where())

class Cannot(RuntimeError):
    pass

def slug() -> str:
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
    return not getattr(sys, "frozen", False)

def already_handed_over() -> bool:
    return os.environ.get(HANDED_OVER) == "1"

def _fetch(url: str) -> bytes:
    try:
        with urllib.request.urlopen(url, timeout=120, context=trusted()) as reply:
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
    name = "dossier-worker.exe" if os.name == "nt" else "dossier-worker"
    binary = os.path.join(landing, name)
    if not os.path.isfile(binary):
        raise Cannot(f"в распакованном нет {name} — брать нечего")

    environment = {**os.environ, HANDED_OVER: "1"}
    arguments = [binary, *sys.argv[1:]]
    logger.info("handing over to %s", binary)

    if sys.platform == "win32":
        CREATE_NEW_CONSOLE = 0x00000010
        subprocess.Popen(
            arguments, env=environment, creationflags=CREATE_NEW_CONSOLE,
        )
        raise SystemExit(0)

    os.execve(binary, arguments, environment)

def where_to_get_it(tag: str) -> str:
    try:
        named = f"dossier-{tag}-{slug()}.zip"
    except Cannot:
        return f"https://github.com/{REPO}/releases/tag/{tag}"
    return f"https://github.com/{REPO}/releases/download/{tag}/{named}"
