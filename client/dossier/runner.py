import asyncio
import json
import os
import re
import shutil
from collections.abc import Awaitable, Callable
from typing import NamedTuple, Optional

from dossier.settings import (
    DOSSIER_BIN,
    DOSSIER_CRF,
    DOSSIER_ENCODER_THREADS,
    DOSSIER_FONT,
    DOSSIER_GAME_SOUNDS,
    DOSSIER_PRESET,
    DOSSIER_SKIN,
)
from dossier.log import get_logger

logger = get_logger("runner")

def _engine_environment() -> dict:
    environment = dict(os.environ)
    if DOSSIER_FONT:
        environment["DOSSIER_FONT"] = DOSSIER_FONT
    return environment

def _plural(n: int, one: str, few: str, many: str) -> str:
    if n % 10 == 1 and n % 100 != 11:
        return one
    if 2 <= n % 10 <= 4 and not 12 <= n % 100 <= 14:
        return few
    return many

_TIMEOUT_SECONDS = 120

_VIDEO_TIMEOUT_SECONDS = 1800

class DossierError(RuntimeError):
    pass

def binary_path() -> str:
    return os.path.expanduser(DOSSIER_BIN)

def is_available() -> bool:
    path = binary_path()
    return os.path.isfile(path) and os.access(path, os.X_OK)

async def _launch(args: tuple[str, ...], timeout: int) -> tuple[int, str, str]:
    path = binary_path()
    if not is_available():
        raise DossierError(
            f"движок не собран: {path} нет или он не исполняемый.\n"
            "Собрать: cargo build --release"
        )

    try:
        process = await asyncio.create_subprocess_exec(
            path,
            *args,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=_engine_environment(),
        )
    except OSError as exc:
        raise DossierError(f"не удалось запустить движок: {exc}") from exc

    try:
        stdout, stderr = await asyncio.wait_for(process.communicate(), timeout)
    except asyncio.TimeoutError:
        process.kill()
        await process.wait()
        raise DossierError(f"движок не ответил за {timeout} с")

    return (
        process.returncode or 0,
        stdout.decode("utf-8", "replace"),
        stderr.decode("utf-8", "replace"),
    )

async def _run(*args: str, timeout: int = _TIMEOUT_SECONDS) -> list[dict]:
    _, stdout, stderr = await _launch(args, timeout)

    results = []
    for line in stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            results.append(json.loads(line))
        except json.JSONDecodeError:
            logger.warning("dossier emitted a non-JSON line: %r", line[:200])

    if not results:
        raise DossierError((stderr.strip() or "движок ничего не вернул")[:500])
    return results

class Progress(NamedTuple):

    done: int
    total: int
    fps: float
    seconds_left: float

    clip: tuple[int, int] | None = None

    @property
    def fraction(self) -> float:
        return self.done / self.total if self.total else 0.0

def _progress_of(event: dict, clip: tuple[int, int] | None) -> Progress | None:
    try:
        return Progress(
            int(event["frames"]),
            int(event["of"]),
            float(event["per_second"]),
            float(event["left_seconds"]),
            clip,
        )
    except (KeyError, TypeError, ValueError):
        logger.warning("dossier sent a progress event this side cannot read: %r", event)
        return None

def _clip_of(event: dict) -> tuple[int, int] | None:
    try:
        return int(event["index"]), int(event["of"])
    except (KeyError, TypeError, ValueError):
        return None

def _polite_prefix() -> tuple[str, ...]:
    for candidate in ("/usr/bin/nice", "/bin/nice"):
        if os.access(candidate, os.X_OK):
            return (candidate, "-n", "10")
    found = shutil.which("nice")
    return (found, "-n", "10") if found else ()

async def _launch_watched(
    args: tuple[str, ...],
    timeout: int,
    on_progress: Callable[[Progress], Awaitable[None]] | None,
    polite: bool = False,
    prefix: tuple[str, ...] = (),
) -> tuple[int, str, list[dict]]:
    path = binary_path()
    if not is_available():
        raise DossierError(
            f"движок не собран: {path} нет или он не исполняемый.\n"
            "Собрать: cargo build --release"
        )

    engine = (*prefix, *_polite_prefix(), path) if polite else (*prefix, path)
    argv = (*engine, *args)
    try:
        process = await asyncio.create_subprocess_exec(
            *argv,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=_engine_environment(),
        )
    except OSError as exc:
        raise DossierError(f"не удалось запустить движок: {exc}") from exc

    collected: list[str] = []
    events: list[dict] = []

    async def watch() -> None:
        clip: tuple[int, int] | None = None
        while True:
            line = await process.stdout.readline()
            if not line:
                break
            text = line.decode("utf-8", "replace").strip()
            if not text:
                continue
            try:
                event = json.loads(text)
            except json.JSONDecodeError:
                logger.warning("dossier sent a non-event line: %r", text[:200])
                continue
            events.append(event)
            kind = event.get("event")
            if kind == "clip":

                clip = _clip_of(event) or clip
            elif kind == "progress" and on_progress:
                progress = _progress_of(event, clip)
                if progress:
                    await on_progress(progress)

    async def keep() -> None:
        while True:
            block = await process.stderr.read(4096)
            if not block:
                break
            collected.append(block.decode("utf-8", "replace"))

    try:
        await asyncio.wait_for(asyncio.gather(watch(), keep(), process.wait()), timeout)
    except asyncio.TimeoutError:
        process.kill()
        await process.wait()
        raise DossierError(f"движок не ответил за {timeout} с")
    except asyncio.CancelledError:

        process.kill()
        await process.wait()
        raise

    return process.returncode or 0, "".join(collected), events

_USAGE = re.compile(r"^(-|dossier \w+ \[OPTIONS\]|Options:|Examples?:)")

def _why_it_failed(report: list[str]) -> str:
    refused_to_start = any(_USAGE.match(line) for line in report)
    meat = [line for line in report if not _USAGE.match(line)] or report
    said = meat[:2] if refused_to_start else meat[-6:]
    return "\n".join(said).strip()[:500]

def _report_lines(stderr: str) -> list[str]:
    lines = []
    for chunk in stderr.replace("\r", "\n").splitlines():
        chunk = chunk.strip()
        if chunk and "frames," not in chunk:
            lines.append(chunk)
    return lines

async def inspect(replay_path: str) -> dict:
    return (await _run("inspect", "--json", replay_path))[0]

async def judge(replay_path: str, songs_dir: str) -> dict:
    return (await _run("judge", "--json", "--songs", os.path.expanduser(songs_dir), replay_path))[0]

class RenderResult(NamedTuple):

    report: list[str]
    width: int | None
    height: int | None
    duration: int | None

def _video_meta(events: list[dict]) -> tuple[int | None, int | None, int | None]:

    for event in reversed(events):
        if event.get("event") != "video":
            continue
        try:
            return (
                int(event["width"]),
                int(event["height"]),
                round(float(event["seconds"])),
            )
        except (KeyError, TypeError, ValueError):
            logger.warning("dossier sent a video event this side cannot read: %r", event)
            return None, None, None
    return None, None, None

def _render_args(
    command: str,
    replay_path: str,
    songs_dir: str,
    out_path: str,
    *,
    size: str,
    fps: int,
    mute: bool,
    skin: str | None,
    leaderboard: str | None,
    my_pictures: tuple[str | None, str | None],
    extra: tuple[str, ...] = (),
    threads: int | None = None,
    encoder_threads: int | None = None,
    background: bool = False,
    bare: bool = False,
    effects: str | None = None,
    music: int | None = None,
    hitsounds: int | None = None,
    map_hitsounds: bool = True,
    dim: int | None = None,
    meter: int | None = None,
    cursor: int | None = None,
    blur: int | None = None,
    volume: int | None = None,
) -> list[str]:
    args = [
        command,

        "--events",
        "--skin",
        skin or DOSSIER_SKIN,
        "--preset",
        DOSSIER_PRESET,
        "--crf",
        DOSSIER_CRF,
        "--songs",
        os.path.expanduser(songs_dir),
        "--size",
        size,
        "--fps",
        str(fps),

        *(["--game-sounds", os.path.expanduser(DOSSIER_GAME_SOUNDS)]
          if DOSSIER_GAME_SOUNDS else []),
        *extra,
        "--out",
        out_path,
        replay_path,
    ]
    if mute:
        args.append("--mute")

    if background:
        args.append("--background")
    if bare:
        args.append("--bare")

    if effects is not None:
        args += ["--effects", effects]

    if music is not None and music != 100:
        args += ["--music", str(music)]
    if hitsounds is not None and hitsounds != 100:
        args += ["--hitsounds", str(hitsounds)]

    if not map_hitsounds:
        args.append("--no-map-hitsounds")

    if dim is not None:
        args += ["--dim", str(dim)]

    if meter is not None:
        args += ["--meter-scale", f"{meter / 100:.2f}"]
    if cursor is not None:
        args += ["--cursor-scale", f"{cursor / 100:.2f}"]
    if blur is not None:
        args += ["--blur", str(blur)]
    if volume is not None:
        args += ["--volume", str(volume)]

    if leaderboard:
        path = os.path.join(os.path.dirname(out_path) or ".", "rivals.tsv")
        try:
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(leaderboard)
            args[1:1] = ["--leaderboard", path]
        except OSError as exc:
            logger.warning("could not write the scoreboard: %s", exc)

    if leaderboard and all(my_pictures):
        args[1:1] = ["--my-pictures", my_pictures[0], my_pictures[1]]

    if threads:
        args[1:1] = ["--threads", str(threads)]
    cap = str(encoder_threads) if encoder_threads else DOSSIER_ENCODER_THREADS.strip()
    if cap:
        args[1:1] = ["--encoder-threads", cap]
    return args

async def video(
    replay_path: str,
    songs_dir: str,
    out_path: str,
    *,
    size: str = "1280x720",
    fps: int = 60,
    mute: bool = False,
    skin: str | None = None,
    leaderboard: str | None = None,
    my_pictures: tuple[str | None, str | None] = (None, None),
    on_progress: Callable[[Progress], Awaitable[None]] | None = None,
    threads: int | None = None,
    encoder_threads: int | None = None,
    polite: bool = False,
    prefix: tuple[str, ...] = (),
    background: bool = False,
    bare: bool = False,
    effects: str | None = None,
    music: int | None = None,
    hitsounds: int | None = None,
    map_hitsounds: bool = True,
    dim: int | None = None,
    meter: int | None = None,
    cursor: int | None = None,
    blur: int | None = None,
    volume: int | None = None,
) -> RenderResult:
    args = _render_args(
        "video",
        replay_path,
        songs_dir,
        out_path,
        size=size,
        fps=fps,
        mute=mute,
        skin=skin,
        leaderboard=leaderboard,
        my_pictures=my_pictures,
        threads=threads,
        encoder_threads=encoder_threads,
        background=background,
        bare=bare,
        effects=effects,
        music=music,
        hitsounds=hitsounds,
        map_hitsounds=map_hitsounds,
        dim=dim,
        meter=meter,
        cursor=cursor,
        blur=blur,
        volume=volume,
    )

    code, stderr, events = await _launch_watched(
        tuple(args), _VIDEO_TIMEOUT_SECONDS, on_progress, polite=polite, prefix=prefix
    )
    report = _report_lines(stderr)

    if code != 0:

        said = _why_it_failed(report)
        raise DossierError(said or f"движок завершился с кодом {code} и ничего не сказал")
    if not os.path.exists(out_path) or os.path.getsize(out_path) == 0:
        raise DossierError("движок отработал, но файла нет")

    for line in report:
        logger.info("dossier: %s", line)
    width, height, duration = _video_meta(events)
    return RenderResult(report, width, height, duration)

class Moment(NamedTuple):

    from_ms: float
    to_ms: float
    scorer: str

    reason: str

    detail: dict

    also: "Moment | None" = None

    def stamp(self) -> str:
        total = max(self.from_ms, 0.0) / 1000.0
        return f"{int(total // 60)}:{int(total % 60):02d}"

    def say(self) -> str:
        say = _PHRASE.get(self.scorer)
        if not say:
            return self.reason
        try:
            return say(self.detail)
        except (KeyError, TypeError, ValueError):
            return self.reason

_PHRASE = {
    "kiai": lambda d: (
        f"кияй — {d['length_ms'] / 1000:.0f} с, отмеченные маппером, на {d['bpm']:.0f} BPM"
    ),
    "peak": lambda d: f"самая длинная серия игры, {d['combo']}x, кончается здесь",
    "choke": lambda d: (
        f"серия {d['combo']}x рвётся на {d['through'] * 100:.0f}% пути"
    ),
    "storm": lambda d: (
        f"самый плотный участок карты, {d['objects']} "
        f"{_plural(d['objects'], 'объект', 'объекта', 'объектов')}"
        if d.get("of_densest", 1.0) >= 0.999
        else f"плотный участок, {d['objects']} "
        f"{_plural(d['objects'], 'объект', 'объекта', 'объектов')} — "
        f"{d['of_densest'] * 100:.0f}% от самого плотного"
    ),
    "precision": lambda d: (
        f"{d['clicks']} {_plural(d['clicks'], 'клик', 'клика', 'кликов')} "
        f"со средней {d['mean_error_ms']:.1f} мс против {d['baseline_ms']:.1f} мс за игру"
    ),
    "scramble": lambda d: _scramble(d["misses"], d["refused"]),
    "opening": lambda d: (
        f"как игра начинается, {d['objects']} "
        f"{_plural(d['objects'], 'объект', 'объекта', 'объектов')} в кадре"
    ),
    "brink": lambda d: (
        f"полоса падает до {d['low']:.0f}% и возвращается к {d['recovered_to']:.0f}%"
    ),
    "finale": lambda d: _finale(d),
    "tapping": lambda d: (
        f"самый частый тап в игре, {d['taps']} "
        f"{_plural(d['taps'], 'нажатие', 'нажатия', 'нажатий')} "
        f"по {d['per_second']:.1f} в секунду"
        if d.get("of_hardest", 1.0) >= 0.999
        else (
            f"частый тап, {d['taps']} "
            f"{_plural(d['taps'], 'нажатие', 'нажатия', 'нажатий')} "
            f"по {d['per_second']:.1f} в секунду"
        )
    ),
    "travel": lambda d: (
        f"самое тяжёлое движение в игре, {d['speed']:.0f} osu!px в секунду"
        if d.get("of_fastest", 1.0) >= 0.999
        else f"тяжёлое движение, {d['speed']:.0f} osu!px в секунду"
    ),
}

def _finale(d: dict) -> str:
    if d.get("failed"):
        return f"игра обрывается — полоса пустеет на {d['combo']}x, {d['accuracy']:.2f}%"
    if d.get("full_combo"):
        return f"доигрывает — {d['combo']}x без единого срыва, {d['accuracy']:.2f}%"
    return f"чем всё кончается — {d['combo']}x, {d['accuracy']:.2f}%"

def _scramble(misses: int, refused: int) -> str:
    parts = []
    if misses:
        parts.append(
            f"{misses} {_plural(misses, 'промах', 'промаха', 'промахов')}"
        )
    if refused:
        parts.append(
            f"{refused} {_plural(refused, 'отказанный клик', 'отказанных клика', 'отказанных кликов')}"
        )
    return " и ".join(parts) + " подряд"

class Selection(NamedTuple):

    clips: list[Moment]
    rate: float

    def watch_seconds(self) -> float:
        span = sum(clip.to_ms - clip.from_ms for clip in self.clips)
        return span / 1000.0 / (self.rate or 1.0)

class ReelResult(NamedTuple):
    render: RenderResult
    selection: Selection

def _moments_of(answer: dict) -> Selection:
    return Selection(
        [
            Moment(
                float(clip.get("from_ms", 0.0)),
                float(clip.get("to_ms", 0.0)),
                str(clip.get("scorer", "?")),
                str(clip.get("reason", "")),
                clip.get("detail") or {},
                _also_of(clip),
            )
            for clip in answer.get("clips", [])
        ],
        float(answer.get("rate") or 1.0),
    )

def _also_of(clip: dict) -> "Moment | None":
    with_ = clip.get("with")
    if not with_:
        return None
    return Moment(
        float(clip.get("from_ms", 0.0)),
        float(clip.get("to_ms", 0.0)),
        str(with_.get("scorer", "?")),
        str(with_.get("reason", "")),
        with_.get("detail") or {},
    )

def _reel_args(budget_s: int | None, clip_s: int | None) -> list[str]:
    args = []
    if budget_s is not None:
        args += ["--for", str(budget_s)]
    if clip_s is not None:
        args += ["--clip", str(clip_s)]
    return args

async def moments(
    replay_path: str,
    songs_dir: str,
    *,
    budget_s: int | None = None,
    clip_s: int | None = None,
) -> Selection:
    answer = await _run(
        "exhibit",
        "--json",
        "--songs",
        os.path.expanduser(songs_dir),
        *_reel_args(budget_s, clip_s),
        replay_path,
    )
    return _moments_of(answer[0])

async def exhibit(
    replay_path: str,
    songs_dir: str,
    out_path: str,
    *,
    size: str = "1280x720",
    fps: int = 60,
    mute: bool = False,
    skin: str | None = None,
    leaderboard: str | None = None,
    my_pictures: tuple[str | None, str | None] = (None, None),
    budget_s: int | None = None,
    clip_s: int | None = None,
    chosen: Selection | None = None,
    on_progress: Callable[[Progress], Awaitable[None]] | None = None,
    threads: int | None = None,
    encoder_threads: int | None = None,
    polite: bool = False,
    prefix: tuple[str, ...] = (),
    background: bool = False,
    bare: bool = False,
    effects: str | None = None,
    music: int | None = None,
    hitsounds: int | None = None,
    map_hitsounds: bool = True,
    dim: int | None = None,
    meter: int | None = None,
    cursor: int | None = None,
    blur: int | None = None,
    volume: int | None = None,
) -> ReelResult:
    if chosen is None:
        chosen = await moments(replay_path, songs_dir, budget_s=budget_s, clip_s=clip_s)
    if not chosen.clips:
        raise DossierError(
            "в этом реплее нечего показать — он короче одного клипа"
        )

    args = _render_args(
        "exhibit",
        replay_path,
        songs_dir,
        out_path,
        size=size,
        fps=fps,
        mute=mute,
        skin=skin,
        leaderboard=leaderboard,
        my_pictures=my_pictures,
        extra=tuple(_reel_args(budget_s, clip_s)),
        threads=threads,
        encoder_threads=encoder_threads,
        background=background,
        bare=bare,
        effects=effects,
        music=music,
        hitsounds=hitsounds,
        map_hitsounds=map_hitsounds,
        dim=dim,
        meter=meter,
        cursor=cursor,
        blur=blur,
        volume=volume,
    )

    code, stderr, events = await _launch_watched(
        tuple(args), _VIDEO_TIMEOUT_SECONDS, on_progress, polite=polite, prefix=prefix
    )
    report = _report_lines(stderr)

    if code != 0:
        said = _why_it_failed(report)
        raise DossierError(said or f"движок завершился с кодом {code} и ничего не сказал")
    if not os.path.exists(out_path) or os.path.getsize(out_path) == 0:
        raise DossierError("движок отработал, но файла нет")

    for line in report:
        logger.info("dossier: %s", line)
    width, height, duration = _video_meta(events)
    return ReelResult(RenderResult(report, width, height, duration), chosen)

async def version() -> Optional[str]:
    if not is_available():
        return None
    try:
        stat = os.stat(binary_path())
    except OSError:
        return None
    return f"{stat.st_size // 1024} KiB, mtime {int(stat.st_mtime)}"
