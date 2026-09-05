#!/usr/bin/env python3

import argparse
import asyncio
import json
import os
import platform
import re
import shutil
import sys
import zipfile
import tempfile
from datetime import datetime
from time import monotonic

import aiohttp

from dossier import build as engine_build
from dossier import machine
from dossier import maps, runner, skins

from dossier.runner import _plural

from dossier import update
from dossier.log import get_logger

logger = get_logger("worker")

POLL_SECONDS = 1.0

HEARTBEAT_SECONDS = 20.0

MISMATCH_SECONDS = 30.0

RESTING_SECONDS = 15.0

SETTLED_SECONDS = 5.0

CONFIG = "~/.dossier/worker.env"

class Abandoned(Exception):
    pass

class BuildMismatch(RuntimeError):

    def __init__(self, said: str, release: str = "") -> None:
        super().__init__(said)
        self.release = release

class Server:

    def __init__(self, base: str, token: str, name: str) -> None:
        self.base = base.rstrip("/")
        self.headers = {"Authorization": f"Bearer {token}", "X-Render-Worker": name}
        self.session: aiohttp.ClientSession | None = None

    async def __aenter__(self):
        self.session = aiohttp.ClientSession(
            headers=self.headers,
            connector=aiohttp.TCPConnector(ssl=update.trusted()),
        )
        return self

    async def __aexit__(self, *_):
        await self.session.close()

    async def claim(self, engine: str | None, capacity=None) -> dict | None:
        told = {"engine": engine}
        if capacity is not None:

            told["capacity"] = {
                "take": bool(capacity.take),
                "reason": capacity.reason,

                "code": capacity.code,
                "detail": capacity.detail,
                "threads": int(capacity.threads or 0),
                "polite": bool(capacity.polite),
            }
        async with self.session.post(
            f"{self.base}/render/claim", json=told
        ) as reply:
            if reply.status == 204:
                return None
            if reply.status == 401:
                raise SystemExit("the server rejected the token")
            if reply.status == 409:
                body = await reply.json()
                raise BuildMismatch(
                    body.get("reason", "the builds do not match"),
                    str(body.get("release", "")),
                )
            reply.raise_for_status()
            return await reply.json()

    async def fetch_replay(self, job_id: str, into: str) -> None:
        async with self.session.get(f"{self.base}/render/job/{job_id}/replay") as reply:
            reply.raise_for_status()
            with open(into, "wb") as handle:
                async for chunk in reply.content.iter_chunked(1 << 16):
                    handle.write(chunk)

    async def fetch_asset(self, job_id: str, name: str, into: str) -> None:
        async with self.session.get(
            f"{self.base}/render/job/{job_id}/file/{name}"
        ) as reply:
            reply.raise_for_status()
            with open(into, "wb") as handle:
                async for chunk in reply.content.iter_chunked(1 << 16):
                    handle.write(chunk)

    async def heartbeat(self, job_id: str, progress: dict | None = None) -> bool:
        try:
            async with self.session.post(
                f"{self.base}/render/job/{job_id}/heartbeat",
                json={"progress": progress},
            ) as reply:
                return reply.status == 200
        except aiohttp.ClientError as exc:

            logger.warning("heartbeat failed: %s", exc)
            return True

    async def deliver(self, job_id: str, path: str, meta: dict) -> None:
        with open(path, "rb") as handle:
            async with self.session.post(
                f"{self.base}/render/job/{job_id}/result",
                data=handle,
                headers={"X-Render-Meta": json.dumps(meta),
                         "Content-Type": "application/octet-stream"},
            ) as reply:
                reply.raise_for_status()

    async def give_back(self, job_id: str, reason: str) -> None:
        try:
            async with self.session.post(
                f"{self.base}/render/job/{job_id}/give-back", json={"reason": reason}
            ):
                pass
        except aiohttp.ClientError as exc:

            logger.warning("could not hand job %s back: %s", job_id, exc)

def _skin_cache() -> str:
    return os.path.expanduser("~/.dossier/worker-skins")

SKIN_CACHE_BYTES = 2 * 1024 * 1024 * 1024

def _folder_size(path: str) -> int:
    total = 0
    for here, _, leaves in os.walk(path):
        for leaf in leaves:
            try:
                total += os.path.getsize(os.path.join(here, leaf))
            except OSError:
                pass
    return total

def prune_skins(cap: int = SKIN_CACHE_BYTES) -> int:
    root = _skin_cache()
    try:
        folders = [
            (entry.stat().st_mtime, entry.path)
            for entry in os.scandir(root)
            if entry.is_dir() and not entry.name.endswith(".incoming")
        ]
    except OSError:
        return 0

    held = sum(_folder_size(path) for _, path in folders)
    if held <= cap:
        return 0

    dropped = 0
    for _, path in sorted(folders):
        if held <= cap:
            break
        held -= _folder_size(path)
        shutil.rmtree(path, ignore_errors=True)
        dropped += 1
    logger.info(
        "skin cache: dropped %d least-used skin(s), now about %d MB",
        dropped, held // (1024 * 1024),
    )
    return dropped

def _localised_skin(settings: dict, here: dict) -> str | None:
    named = settings.get("skin")
    if not named:
        return None
    if not named.startswith("{{"):

        return None

    digest = settings.get("skin_hash") or "unknown"
    if not digest.isalnum():
        logger.warning("odd skin hash %r", digest)
        return None
    folder = os.path.join(_skin_cache(), digest)
    if os.path.isdir(folder):
        logger.info("skin %s already here", digest)

        try:
            os.utime(folder, None)
        except OSError:
            pass
        return folder

    archive = here.get(named.strip("{}"))
    if not archive or not os.path.isfile(archive):
        logger.warning("job named a skin that did not arrive")
        return None

    staging = folder + ".incoming"
    shutil.rmtree(staging, ignore_errors=True)
    os.makedirs(staging, exist_ok=True)
    try:
        with zipfile.ZipFile(archive) as pack:
            for item in pack.infolist():
                if item.is_dir():
                    continue
                leaf = os.path.basename(item.filename)
                if not leaf:
                    continue
                with pack.open(item) as source, open(
                    os.path.join(staging, leaf), "wb"
                ) as sink:
                    shutil.copyfileobj(source, sink)
    except (zipfile.BadZipFile, OSError) as exc:
        logger.warning("skin %s would not unpack: %s", digest, exc)
        shutil.rmtree(staging, ignore_errors=True)
        return None

    converted = skins.convert_folder(staging)

    os.makedirs(_skin_cache(), exist_ok=True)
    os.replace(staging, folder)

    prune_skins()
    logger.info(
        "skin %s unpacked%s",
        digest,
        f", {converted} sample(s) converted" if converted else "",
    )
    return folder

_HINTS = (
    ("ffmpeg", "ffmpeg is not on PATH — a skin's samples cannot be converted "
               "and the audio cannot be muxed"),
    ("dossier", "the engine may not be built: cargo build --release"),
    ("401", "the bot refused this worker's token — check RENDER_WORKER_TOKEN"),
    ("No space left", "the disk is full where this worker renders"),
)

def hint(exc: Exception) -> None:
    said = str(exc)
    for needle, meaning in _HINTS:
        if needle in said:
            logger.warning("  ^ %s", meaning)
            return

async def _render(server: Server, job: dict, capacity) -> bool:
    job_id = job["id"]
    workdir = tempfile.mkdtemp(prefix="render-worker-")
    replay = os.path.join(workdir, "replay.osr")
    out = os.path.join(workdir, "render.mp4")

    lost = asyncio.Event()

    async def on_progress(told) -> None:
        if lost.is_set():
            return
        if not await server.heartbeat(job_id, {
            "done": told.done, "total": told.total, "fps": told.fps,
            "seconds_left": told.seconds_left,
            "clip": list(told.clip) if told.clip else None,
        }):
            lost.set()

    async def keep_alive() -> None:
        while not lost.is_set():
            await asyncio.sleep(HEARTBEAT_SECONDS)
            if not lost.is_set() and not await server.heartbeat(job_id):
                lost.set()

    try:

        known = job["settings"].get("beatmap") or {}
        if not (known.get("beatmapset_id") or known.get("id")):
            raise maps.MapUnavailable(
                "this job names no map, which means the bot is older than this "
                "worker — `git pull` and restart it there"
            )

        await server.fetch_replay(job_id, replay)

        here = {}
        for name in job.get("assets") or []:
            if not name.isalnum():
                logger.warning("job %s offered an odd asset name %r", job_id, name)
                continue

            suffix = "zip" if f"{{{{{name}}}}}" == job["settings"].get("skin") else "png"
            landed = os.path.join(workdir, f"{name}.{suffix}")
            await server.fetch_asset(job_id, name, landed)
            here[name] = landed

        def localise(text):
            for name, path in here.items():
                text = text.replace("{{%s}}" % name, path)

            return re.sub(r"\{\{a\d+\}\}", "", text)

        settings = job["settings"]
        skin = _localised_skin(settings, here)
        board = settings.get("leaderboard")
        board = localise(board) if board else None
        mine = tuple(localise(p) or None for p in (settings.get("my_pictures") or ["", ""]))

        header = await runner.inspect(replay)
        checksum = header.get("beatmap_hash") or ""

        await maps.ensure_known(known, checksum)

        with machine.awake() as stay_awake:
            watcher = asyncio.create_task(keep_alive())

            engine = runner.exhibit if job["settings"].get("kind") == "exhibit" else runner.video
            render = asyncio.create_task(engine(
                replay, maps.songs_dir(), out,
                size=settings.get("size") or "1280x720",
                fps=int(settings.get("fps") or 60),
                mute=bool(settings.get("mute")),
                background=bool(settings.get("background")),
                bare=bool(settings.get("bare")),

                effects=settings.get("effects"),
                music=settings.get("music"),
                hitsounds=settings.get("hitsounds"),

                map_hitsounds=bool(settings.get("map_hitsounds", True)),
                dim=settings.get("dim"),
                meter=settings.get("meter"),
                cursor=settings.get("cursor"),
                blur=settings.get("blur"),
                volume=settings.get("volume"),
                skin=skin,
                leaderboard=board,
                my_pictures=mine,
                on_progress=on_progress,
                threads=capacity.threads,
                encoder_threads=capacity.encoder_threads,
                polite=capacity.polite,
                prefix=stay_awake,
            ))

            gone = asyncio.create_task(lost.wait())
            try:
                await asyncio.wait({render, gone}, return_when=asyncio.FIRST_COMPLETED)
                if not render.done():
                    render.cancel()
                    raise Abandoned("задачу забрали, пока шёл рендер")
                result = render.result()
            finally:
                lost.set()
                for task in (watcher, gone):
                    task.cancel()
                if not render.done():
                    render.cancel()

                await asyncio.gather(render, watcher, gone, return_exceptions=True)

        made = getattr(result, "render", result)
        await server.deliver(job_id, out, {
            "report": made.report, "width": made.width,
            "height": made.height, "duration": made.duration,
        })
        logger.info("job %s delivered", job_id)
        return True
    except Abandoned as exc:

        logger.info("job %s: %s", job_id, exc)
        await asyncio.sleep(POLL_SECONDS)
        return False
    except (runner.DossierError, maps.MapUnavailable, aiohttp.ClientError, OSError) as exc:
        logger.warning("job %s handed back: %s", job_id, exc)
        hint(exc)
        await server.give_back(job_id, str(exc))

        await asyncio.sleep(POLL_SECONDS)
        return False
    except Exception as exc:

        logger.exception("job %s failed on this worker", job_id)
        await server.give_back(job_id, f"воркер не справился: {exc}")
        await asyncio.sleep(POLL_SECONDS)
        return False
    finally:
        shutil.rmtree(workdir, ignore_errors=True)

def fingerprint(secret: str) -> str:
    if not secret:
        return "нет"
    import hashlib

    short = hashlib.sha256(secret.encode()).hexdigest()[:8]
    return f"{len(secret)} {_plural(len(secret), 'знак', 'знака', 'знаков')}, {short}"

def where(path: str) -> str:
    return os.path.normpath(os.path.expanduser(path))

def read_pairs(path: str) -> dict[str, str]:
    try:

        with open(where(path), "r", encoding="utf-8-sig") as handle:
            lines = handle.readlines()
    except OSError:
        return {}

    found: dict[str, str] = {}
    for line in lines:

        line = line.replace("\u00a0", " ").strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")

        found[key.strip()] = value.strip().strip("'\"")
    return found

def load_config(path: str) -> str | None:
    pairs = read_pairs(path)
    if not pairs:
        return None
    for key, value in pairs.items():
        if key and key not in os.environ:
            os.environ[key] = value
    return where(path)

def _limits_read(limits: machine.Limits) -> str:
    said = []
    if limits.paused:
        said.append("paused")
    if limits.polite:
        said.append("polite")
    if limits.threads:
        said.append(f"at most {limits.threads} threads")
    if limits.hours:
        said.append("between {:02d}:00 and {:02d}:00".format(*limits.hours))
    return ", ".join(said) or "no limits"

def asked_for(path: str, options) -> machine.Limits:
    pairs = read_pairs(path)

    def flag(key: str, unless: bool) -> bool:
        said = pairs.get(key)
        if said is None:
            return unless
        return said.strip().lower() in ("1", "true", "yes", "on")

    threads = options.threads
    if pairs.get("RENDER_THREADS", "").strip().isdigit():
        threads = int(pairs["RENDER_THREADS"])

    return machine.Limits(
        polite=flag("RENDER_POLITE", options.polite),
        threads=max(0, threads),
        hours=machine.parse_hours(pairs.get("RENDER_HOURS", "")),
        paused=flag("RENDER_PAUSE", False),
    )

class Check:

    __slots__ = ("name", "ok", "said", "fix")

    def __init__(self, name: str, ok: bool | None, said: str, fix: str = "") -> None:
        self.name, self.ok, self.said, self.fix = name, ok, said, fix

    def __str__(self) -> str:
        mark = "?" if self.ok is None else ("+" if self.ok else "!")
        line = f" [{mark}] {self.name}: {self.said}"
        return line + (f"\n       -> {self.fix}" if self.fix and not self.ok else "")

async def check(options) -> int:

    in_file = read_pairs(options.config)
    found = load_config(options.config)
    options.server = options.server or os.getenv("RENDER_SERVER", "")

    token = os.getenv("RENDER_WORKER_TOKEN", "")

    checks = [Check("настройки", True if found else None,
                    found or f"нет в {where(options.config)} — "
                             f"их можно держать там, а не в переменных оболочки")]
    if found:

        wanted = ("RENDER_SERVER", "RENDER_WORKER_TOKEN")
        missing = [key for key in wanted if not in_file.get(key)]
        checks.append(Check(
            "в этом файле", not missing,
            ", ".join(key for key in wanted if in_file.get(key)) or "ничего читаемого",
            "не хватает: " + ", ".join(missing) if missing else "",
        ))

    from_file = in_file.get("RENDER_WORKER_TOKEN", "")
    if from_file and token and token != from_file:
        checks.append(Check(
            "токен", False,
            f"{fingerprint(token)} — из окружения, а не из файла",
            f"в файле лежит {fingerprint(from_file)}, но переменная с тем же "
            f"именем его перебивает. В Windows: закройте терминал и откройте "
            f"заново, а если вернётся — `setx RENDER_WORKER_TOKEN \"\"`. "
            f"В остальных: `unset RENDER_WORKER_TOKEN`.",
        ))
    else:
        checks.append(Check("токен", bool(token),
                            fingerprint(token) if token else "нет",
                            "RENDER_WORKER_TOKEN, тот же самый, что у бота"))

    built = runner.is_available()
    checks.append(Check("движок", built,
                        runner.binary_path() if built else f"нет по пути {runner.binary_path()}",
                        "cargo build --release"))
    engine = await engine_build.local(refresh=True) if built else None
    checks.append(Check("сборка", engine is not None,
                        engine or "движок не назвал себя",
                        "пересоберите — движок, который не отвечает на --version, "
                        "слишком стар, чтобы доверять ему рендер"))

    checks.append(Check("ffmpeg", shutil.which("ffmpeg") is not None,
                        shutil.which("ffmpeg") or "нет в PATH",
                        "нужен, чтобы перегнать звуки скина и склеить дорожку"))

    from dossier.settings import DOSSIER_FONT

    checks.append(Check("шрифт", bool(DOSSIER_FONT) and os.path.isfile(DOSSIER_FONT),
                        where(DOSSIER_FONT) if DOSSIER_FONT else "не найден",
                        "без него ролики выходят без счёта, точности и комбо — "
                        "он лежит рядом с движком, так что обычно это значит, "
                        "что файл унесли из папки, в которой он приехал"))

    songs = where(maps.songs_dir())
    checks.append(Check("склад карт", os.path.isdir(songs) or _can_make(songs), songs,
                        "воркер качает карты сюда и не смог создать эту папку"))

    limits = asked_for(options.config, options)
    shut = limits.closed(datetime.now().hour)
    capacity = machine.Capacity(False, shut) if shut else machine.capacity(
        os.cpu_count() or 4, polite=limits.polite, ceiling=limits.threads
    )
    checks.append(Check(
        "эта машина", capacity.take or None,
        f"{capacity.reason}"
        + (
            f", {capacity.threads} "
            + _plural(capacity.threads, "поток", "потока", "потоков")
            if capacity.take
            else ""
        ),
        ""))

    checks.extend(await _ask_the_bot(options, token, engine))

    print(f"воркер рендера dossier — {options.name}")
    for line in checks:
        print(line)
    stopped = [c for c in checks if c.ok is False]
    unsure = [c for c in checks if c.ok is None and c.fix]
    if not stopped:

        if unsure:
            count = len(unsure)
            print(f"\nготово, но {count} "
                  f"{_plural(count, 'пункт', 'пункта', 'пунктов')} выше "
                  f"стоит прочитать сначала")
        else:
            print("\nготово — запускайте без --check")
        return 0
    count = len(stopped)
    print(f"\n{count} {_plural(count, 'пункт', 'пункта', 'пунктов')} "
          f"надо поправить, прежде чем воркер сможет рисовать")
    return 1

def _can_make(path: str) -> bool:
    try:
        os.makedirs(path, exist_ok=True)
        return True
    except OSError:
        return False
async def _ask_the_bot(options, token: str, engine: str | None) -> list:
    if not options.server:
        return [Check("бот", False, "адрес не задан",
                      "--server или RENDER_SERVER в настройках")]
    if not token:
        return [Check("бот", None, "не спрашивали — нечем, токена нет")]

    base = options.server.rstrip("/")
    try:
        async with aiohttp.ClientSession(
            headers={"Authorization": f"Bearer {token}", "X-Render-Worker": options.name},
            timeout=aiohttp.ClientTimeout(total=15),
            connector=aiohttp.TCPConnector(ssl=update.trusted()),
        ) as session:
            async with session.get(
                f"{base}/render/hello", params={"engine": engine or ""}
            ) as reply:
                if reply.status == 401:
                    return [Check(
                        "бот", False,
                        f"токен отвергнут — этот {fingerprint(token)}",
                        "сравните с тем, что бот печатает при запуске: тот же "
                        "отпечаток значит, что дело не в токене, а другой — что "
                        "у кого-то из двоих не та строка. Длина на единицу "
                        "больше ожидаемой — это кавычка или перевод строки, "
                        "приехавшие вместе с ним.",
                    )]
                if reply.status == 404:
                    return [Check("бот", False, "отвечает, но у него нет "
                                  "/render/hello", "бот старше этого воркера — "
                                  "обновите его")]
                reply.raise_for_status()
                said = await reply.json()
    except (aiohttp.ClientError, asyncio.TimeoutError) as exc:
        return [Check("бот", False, f"не достучались до {base}: {exc}",
                      "проверьте адрес и что бот вообще запущен")]

    waiting = said.get("waiting", 0)
    checks = [Check("бот", True,
                    f"{base}, {waiting} "
                    f"{_plural(waiting, 'задача', 'задачи', 'задач')} в очереди")]

    mine = engine_build.build_of(engine)
    theirs = said.get("build") or engine_build.UNKNOWN
    if engine_build.UNKNOWN in (mine, theirs):
        which = "этого воркера" if mine == engine_build.UNKNOWN else "бота"
        checks.append(Check(
            "сборки", None,
            f"движок {which} не говорит, из чего собран, так что сравнивать "
            f"нечего",
            "почти всегда это дерево исходников без git — скачанный zip вместо "
            "`git clone`. Клонируйте репозиторий и соберите заново, иначе "
            "воркер будет рисовать тем кодом, какой окажется под рукой.",
        ))
    else:
        checks.append(Check("сборки", bool(said.get("agree")),
                            said.get("reason") or "?",
                            "причина говорит, какую сторону пересобрать"))
    return checks

def service(options) -> int:
    if getattr(sys, "frozen", False):

        args = [os.path.abspath(sys.executable)]
        root = os.path.dirname(args[0])
    else:

        here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        args = [sys.executable, os.path.join(here, "worker.py")]
        root = os.path.dirname(here)
    if options.server:
        args += ["--server", options.server]
    if options.name:
        args += ["--name", options.name]
    if options.polite:
        args.append("--polite")
    if options.threads:
        args += ["--threads", str(options.threads)]
    if options.config != CONFIG:
        args += ["--config", options.config]
    line = " ".join(args)

    if sys.platform == "darwin":
        where = os.path.expanduser("~/Library/LaunchAgents/org.dossier.worker.plist")
        body = "\n".join(
            ['<?xml version="1.0" encoding="UTF-8"?>',
             '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" '
             '"http://www.apple.com/DTDs/PropertyList-1.0.dtd">',
             '<plist version="1.0"><dict>',
             '  <key>Label</key><string>org.dossier.worker</string>',
             '  <key>ProgramArguments</key><array>']
            + [f"    <string>{arg}</string>" for arg in args]
            + ['  </array>',
               '  <key>WorkingDirectory</key>' f'<string>{root}</string>',

               '  <key>KeepAlive</key><true/>',
               '  <key>RunAtLoad</key><true/>',
               f'  <key>StandardOutPath</key><string>{root}/worker.log</string>',
               f'  <key>StandardErrorPath</key><string>{root}/worker.log</string>',
               '</dict></plist>'])
        after = (f"launchctl unload {where} 2>/dev/null\n"
                 f"launchctl load -w {where}")
    else:
        where = os.path.expanduser("~/.config/systemd/user/dossier-worker.service")
        body = "\n".join([
            "[Unit]",
            "Description=dossier render worker",
            "After=network-online.target",
            "",
            "[Service]",
            f"ExecStart={line}",
            f"WorkingDirectory={root}",

            "Restart=always",
            "RestartSec=10",
            "",
            "[Install]",
            "WantedBy=default.target",
        ])
        after = ("systemctl --user daemon-reload\n"
                 "systemctl --user enable --now dossier-worker\n"
                 "# and, so it survives logging out:\n"
                 f"loginctl enable-linger {os.getenv('USER', 'you')}")

    print(f"# write this to {where}\n")
    print(body)
    print(f"\n# then:\n{after}")
    return 0

async def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--server", help="where the bot answers "
                                         "(default: RENDER_SERVER)")

    parser.add_argument("--name", default=platform.node() or "worker",
                        help="how to call this worker")
    parser.add_argument("--once", action="store_true", help="take one job and stop")
    parser.add_argument("--config", default=CONFIG,
                        help=f"where the settings are (default: {CONFIG})")
    parser.add_argument("--check", action="store_true",
                        help="say whether this machine is set up, and stop")

    parser.add_argument("--polite", action="store_true",
                        help="somebody is using this machine — take less of it")
    parser.add_argument("--threads", type=int, default=0, metavar="N",
                        help="never use more than N threads, whatever the policy says")
    parser.add_argument("--service", action="store_true",
                        help="print the unit that would keep this worker "
                             "running, and stop")
    options = parser.parse_args()

    if options.service:
        load_config(options.config)
        options.server = options.server or os.getenv("RENDER_SERVER", "")
        raise SystemExit(service(options))

    if options.check:

        raise SystemExit(await check(options))

    from dossier import console

    if console.wanted(options, sys.argv[1:]):
        if await console.run(options) == "quit":
            return

    load_config(options.config)
    options.server = options.server or os.getenv("RENDER_SERVER", "")

    token = os.getenv("RENDER_WORKER_TOKEN", "")

    missing = [what for what, got in (
        ("--server (or RENDER_SERVER)", options.server),
        ("RENDER_WORKER_TOKEN", token),
    ) if not got]
    if missing:
        raise SystemExit(
            f"not set: {', '.join(missing)}\n"
            f"put them in {options.config}, then `--check` to see the rest"
        )
    if not runner.is_available():
        raise SystemExit(f"the engine is not built: {runner.binary_path()}\n"
                         f"cargo build --release")

    await _watch(options, token)

async def _offer_the_right_build(release: str) -> bool:
    from dossier import console

    if not release:
        return False

    if update.from_a_checkout():
        logger.info("this is a checkout — `git pull && cargo build --release`")
        return False
    if update.already_handed_over():

        logger.warning("already updated once and the builds still differ — "
                       "the bot may have moved again since")
        return False
    if not console.interactive():
        logger.warning("the bot is on %s — download it and restart:\n  %s",
                       release, update.where_to_get_it(release))
        return False

    print(f"\n  Бот работает на версии {release}, а эта — другая.")
    print("  Поэтому задачи и не берутся: разные сборки рисуют по-разному.")
    if console._ask("  Скачать нужную и перезапуститься? (да/нет)", "да").lower() \
            not in ("да", "д", "y", "yes"):
        print("  Хорошо, стою и жду.")
        return False

    try:
        landing = update.fetch(release)
    except update.Cannot as exc:
        print(f"  ✗ {exc}")
        return False

    print("  Перезапускаюсь на новой версии.\n")
    update.hand_over(landing)
    return True

async def _watch(options, token: str) -> None:
    from dossier.console import Line

    cores = os.cpu_count() or 4
    refused = None
    standing_by = None
    told_so = None
    done = handed_back = 0

    line = Line()
    away_since = None
    dots = 0
    settle_at = None

    engine = await engine_build.local()
    logger.info("engine: %s", engine or "could not be asked its version")

    async with Server(options.server, token, options.name) as server:
        logger.info("worker %s watching %s", options.name, options.server)
        while True:

            limits = asked_for(options.config, options)
            if limits != told_so:
                if told_so is not None:
                    logger.info("limits changed: %s", _limits_read(limits))
                told_so = limits

            hour = datetime.now().hour
            shut = limits.closed(hour)
            capacity = (
                machine.Capacity(False, shut, code=limits.code(hour),
                                 detail=f"{limits.hours[0]:02d}:00–{limits.hours[1]:02d}:00"
                                 if limits.hours else "")
                if shut
                else machine.capacity(cores, polite=limits.polite, ceiling=limits.threads)
            )
            if not capacity.take and capacity.reason != refused:

                logger.info("not taking work: %s", capacity.reason)
                refused = capacity.reason
            if capacity.take:
                refused = None

            try:

                job = await server.claim(engine, capacity)
            except BuildMismatch as exc:

                if options.once:
                    raise SystemExit(f"this worker cannot take work: {exc}") from exc
                if str(exc) != standing_by:
                    logger.warning("standing by — %s", exc)
                    standing_by = str(exc)

                    if await _offer_the_right_build(exc.release):
                        return
                await asyncio.sleep(MISMATCH_SECONDS)

                engine = await engine_build.local(refresh=True)
                continue
            except aiohttp.ClientError as exc:
                if away_since is None:

                    away_since = monotonic()
                    logger.warning("lost the bot: %s", exc)
                dots = dots % 3 + 1
                line.say(f"  Потеряно соединение с ботом{'.' * dots}"
                         f"  Повторяю подключение{'.' * dots}")
                await asyncio.sleep(POLL_SECONDS)
                continue

            if away_since is not None:

                line.clear()

                logger.info("the bot is back after %.0fs", monotonic() - away_since)
                line.say("  Соединение восстановлено!")
                settle_at = monotonic() + SETTLED_SECONDS
                away_since = None
                dots = 0
            elif settle_at is not None and monotonic() >= settle_at:
                line.clear()
                settle_at = None

            if standing_by is not None:
                logger.info("the builds agree again (%s) — taking work", engine)
                standing_by = None

            if job is None:
                await asyncio.sleep(
                    POLL_SECONDS if capacity.take else RESTING_SECONDS
                )
                continue

            logger.info("job %s (%s): %s, %s threads", job["id"], job.get("title") or "?",
                        capacity.reason, capacity.threads)
            if await _render(server, job, capacity):
                done += 1
            else:
                handed_back += 1

            logger.info("this worker: %s delivered, %s handed back", done, handed_back)
            if options.once:
                return

def _readable_output() -> None:
    if sys.platform == "win32":
        try:
            import ctypes

            ctypes.windll.kernel32.SetConsoleOutputCP(65001)
        except Exception:
            pass

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

def run() -> None:
    _readable_output()

    from dossier import log as _log

    _log.to_console()
    _log.to_file()

    code = 0
    try:
        asyncio.run(main())
    except KeyboardInterrupt:

        pass
    except SystemExit as exc:

        if isinstance(exc.code, str):
            print(exc.code, file=sys.stderr)
            code = 1
        elif exc.code:
            code = exc.code

    from dossier import console

    console.hold_the_window()
    if code:
        raise SystemExit(code)

if __name__ == "__main__":
    run()
