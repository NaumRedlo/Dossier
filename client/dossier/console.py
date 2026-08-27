"""The client as something to use, rather than something to configure.

Setting a worker up meant creating a file in Notepad, in a folder that does not
exist yet, with a name beginning with a dot, and pasting a sixty-character
secret into it without picking up a quote or a trailing space. Every one of
those is a place to get it wrong quietly, and people did: two friends once ran
for days against a token that differed from the server's, and the only symptom
was work that never arrived.

So the program asks. It writes the same `worker.env` it always read — nothing
here is a new format, and a file edited by hand goes on working exactly as
before — but nobody has to know that.

The text here is Russian because the people running a worker are. The comments
are English like the rest of the repository.

Nothing in this module is reached unless the program is attached to a terminal
and was started with no instructions: a service, a pipe and `--once` all go
straight to work as they always did.
"""

import os
import shutil
import sys

# Pre-filled, because it is the same for everybody who will ever run this and
# typing it is one more thing to get wrong. Overridden by whatever is already
# in the config.
DEFAULT_SERVER = "https://onenineeightfour.ignorelist.com"

# What the machine's owner controls. Everything about how a render *looks* is
# decided by whoever asked for it, in the bot — this side owns how much of this
# computer the work may have, and nothing else.
LIMITS = ("RENDER_POLITE", "RENDER_THREADS", "RENDER_PAUSE", "RENDER_HOURS")


def interactive() -> bool:
    """Whether there is a person here to answer.

    Both ways: a program whose output is being captured should not draw a menu,
    and one whose input is a pipe cannot read an answer and would spin on
    end-of-file for ever.
    """
    try:
        return sys.stdin.isatty() and sys.stdout.isatty()
    except (AttributeError, ValueError):  # a closed or replaced stream
        return False


def wanted(options, given: list[str]) -> bool:
    """Whether to offer the menu at all.

    Any instruction on the command line is somebody who knows what they want,
    and the menu would be in the way. `--polite` and `--threads` are not
    instructions in that sense: they say how to work, not whether to start.
    """
    if not interactive():
        return False
    if options.check or options.service or options.once:
        return False
    # `--server` names a bot, which is the one thing the setup screen is for.
    return not any(argument.startswith("--server") for argument in given)


# ── drawing ─────────────────────────────────────────────────────────────────
#
# No curses and no third-party anything: this has to work inside a frozen
# executable on a Windows console, and every dependency added here is one more
# thing that can fail on a machine nobody can reach.


def _width() -> int:
    return max(48, min(78, shutil.get_terminal_size((80, 24)).columns))


def _clear() -> None:
    """Start the screen again.

    Two ways, because neither works everywhere. On Windows `cls` is the one
    thing every console understands, since ANSI needs virtual-terminal
    processing turned on and a frozen program started from Explorer does not
    get it. Everywhere else the escape is better than shelling out to `clear`:
    no process, and no `TERM environment variable not set` printed across the
    top of the first screen somebody ever sees.
    """
    try:
        if sys.platform == "win32":
            os.system("cls")  # noqa: S605,S607
        else:
            # Cursor home, then erase from there down.
            print("\033[H\033[J", end="")
    except Exception:  # noqa: BLE001 — a screen that scrolled still reads
        print("\n" * 3)


def _title(text: str) -> None:
    print()
    print(f"  {text}")
    print("  " + "─" * (_width() - 4))


def _ask(prompt: str, default: str = "") -> str:
    """One line of input, with Enter meaning the default.

    `EOFError` is Ctrl-D and `KeyboardInterrupt` is Ctrl-C, and both mean the
    same thing here — somebody wants out of this question. Neither should end
    with a traceback across a screen that was drawn to be calm.
    """
    shown = f" [{default}]" if default else ""
    try:
        said = input(f"  {prompt}{shown}: ").strip()
    except (EOFError, KeyboardInterrupt):
        print()
        return default
    return said or default


def _pause() -> None:
    try:
        input("\n  — Enter, чтобы вернуться —")
    except (EOFError, KeyboardInterrupt):
        print()


# ── the file ────────────────────────────────────────────────────────────────


def write_pairs(path: str, pairs: dict[str, str]) -> str:
    """Save the settings, keeping anything this program does not know about.

    Rewritten rather than patched, because a file this program wrote is a file
    it can read back exactly — and the alternative, editing lines in place,
    goes wrong the first time somebody's editor leaves a stray blank line or a
    duplicate key.

    Whatever keys are not ours are carried through untouched. Somebody may have
    put `DOSSIER_FFMPEG` in here, and losing it because a menu did not know the
    name would be the menu doing harm.
    """
    from dossier.worker import where

    full = where(path)
    os.makedirs(os.path.dirname(full) or ".", exist_ok=True)

    known = ("RENDER_SERVER", "RENDER_WORKER_TOKEN", *LIMITS)
    rest = {key: value for key, value in pairs.items() if key not in known}

    lines = [
        "# Dossier — настройки воркера.",
        "#",
        "# Файл перечитывается на каждом опросе: правки применяются без",
        "# перезапуска. Программа пишет сюда сама, но руками тоже можно.",
        "",
        "# Кому работать и чем это доказать.",
    ]
    for key in ("RENDER_SERVER", "RENDER_WORKER_TOKEN"):
        lines.append(f"{key}={pairs.get(key, '')}")
    lines += ["", "# Сколько отдавать этой машины."]
    for key in LIMITS:
        value = pairs.get(key, "")
        # An empty limit is no limit, and a line saying so is clearer than the
        # absence of a line — somebody reading the file can see the knob exists.
        lines.append(f"{key}={value}" if value else f"# {key}=")
    if rest:
        lines += ["", "# Остальное."]
        lines += [f"{key}={value}" for key, value in sorted(rest.items())]
    lines.append("")

    with open(full, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))

    # The token is in here. Nothing else on the machine needs to read it, and
    # a home directory is not always the only account on a computer.
    try:
        os.chmod(full, 0o600)
    except OSError:  # Windows, and it does not mean the same thing there
        pass
    return full


# ── screens ─────────────────────────────────────────────────────────────────


async def _try_the_bot(server: str, token: str, name: str) -> tuple[bool, str]:
    """Ask the bot whether it knows this token, before anything is saved.

    The whole reason this screen exists. A token typed correctly and a token
    typed *nearly* correctly look identical in a text editor, and the second
    one used to be discovered days later by somebody wondering why their
    machine never got any work.
    """
    import types

    from dossier import worker

    engine = await worker.engine_build.local(refresh=True)
    # Everything `_ask_the_bot` reads, and a test holds this to that — a
    # namespace missing one attribute fails as "could not ask the bot", which
    # reads as the bot being unreachable rather than as this line being wrong.
    settings = types.SimpleNamespace(
        server=server, name=name, config="/nonexistent",
    )
    try:
        checks = await worker._ask_the_bot(settings, token, engine)
    except Exception as exc:  # noqa: BLE001 — any failure here is "could not ask"
        return False, f"не удалось спросить бота: {exc}"

    for check in checks:
        if check.name == "the bot" and check.ok is False:
            return False, check.said
    said = "; ".join(check.said for check in checks if check.ok)
    return True, said or "бот ответил"


async def connection(path: str, pairs: dict[str, str], name: str = "worker") -> dict[str, str]:
    """Which bot, and the secret that proves this worker may work for it."""
    from dossier.worker import fingerprint

    _title("Подключение")
    print("  Токен даёт право брать задачи. Никому его не показывай.")
    print()

    server = _ask("Адрес бота", pairs.get("RENDER_SERVER") or DEFAULT_SERVER)
    was = pairs.get("RENDER_WORKER_TOKEN", "")
    if was:
        print(f"\n  Сейчас записан токен: {fingerprint(was)}")
        print("  Enter — оставить как есть.")
    token = _ask("Токен", was)

    if not token:
        print("\n  Без токена работать не выйдет — бот не поймёт, кто это.")
        _pause()
        return pairs

    print("\n  Спрашиваю бота…")
    good, said = await _try_the_bot(server, token, name)
    print(f"  {'✓' if good else '✗'} {said}")
    if not good:
        # The bot names the fingerprint when it is the token it disliked, so
        # repeating it here would be the same eight characters twice on one
        # screen. When the refusal was about something else — unreachable,
        # a build mismatch — the fingerprint is still the thing somebody needs
        # in order to compare theirs with the one that was handed out.
        mark = fingerprint(token)
        if mark not in said:
            print(f"\n  Записанный токен: {mark}")
        print("  Если у того, кто его выдал, отпечаток другой — строка не та.")
        if _ask("Сохранить всё равно? (да/нет)", "нет").lower() not in ("да", "д", "y", "yes"):
            _pause()
            return pairs

    pairs = {**pairs, "RENDER_SERVER": server, "RENDER_WORKER_TOKEN": token}
    print(f"\n  Сохранено: {write_pairs(path, pairs)}")
    _pause()
    return pairs


def limits(path: str, pairs: dict[str, str]) -> dict[str, str]:
    """How much of this computer the farm may have.

    These four and no more. Everything about how a video *looks* — its size,
    its frame rate, its skin — belongs to whoever asked for the render, and a
    screen here offering to change any of it would be a screen that lies.
    """
    pairs = dict(pairs)
    while True:
        polite = pairs.get("RENDER_POLITE", "").lower() in ("1", "true", "yes", "on")
        paused = pairs.get("RENDER_PAUSE", "").lower() in ("1", "true", "yes", "on")
        threads = pairs.get("RENDER_THREADS", "")
        hours = pairs.get("RENDER_HOURS", "")

        _clear()
        _title("Сколько отдавать этой машины")
        print(f"   1  вполсилы, когда я за компьютером   [{'да' if polite else 'нет'}]")
        print(f"   2  не больше N ядер                   [{threads or 'сколько решит сам'}]")
        print(f"   3  пауза — стоять и не брать задачи   [{'да' if paused else 'нет'}]")
        print(f"   4  работать только в часы             [{hours or 'круглосуточно'}]")
        print("   0  назад")
        print()
        print("  Машина и так сама сбавляет на батарее, в жару и когда ты")
        print("  за клавиатурой. Это — потолок поверх её решений.")

        said = _ask("\n  Что меняем", "0")
        if said == "0":
            break
        if said == "1":
            pairs["RENDER_POLITE"] = "" if polite else "1"
        elif said == "2":
            answer = _ask("Сколько ядер максимум (пусто — без потолка)", threads)
            pairs["RENDER_THREADS"] = answer if answer.isdigit() and int(answer) > 0 else ""
        elif said == "3":
            pairs["RENDER_PAUSE"] = "" if paused else "1"
        elif said == "4":
            print("\n  Например 22-6 — с десяти вечера до шести утра.")
            pairs["RENDER_HOURS"] = _ask("Часы (пусто — круглосуточно)", hours)
        else:
            continue
        write_pairs(path, pairs)
    return pairs


async def _standing(pairs: dict[str, str]) -> list[str]:
    """The four lines at the top of the menu: who, where, what, and how much."""
    from dossier import machine, runner
    from dossier.worker import fingerprint

    token = pairs.get("RENDER_WORKER_TOKEN", "")
    lines = [
        f"  сервер:  {pairs.get('RENDER_SERVER') or 'не задан'}",
        f"  токен:   {fingerprint(token) if token else 'не задан'}",
    ]
    if runner.is_available():
        from dossier import build as engine_build

        lines.append(f"  движок:  {await engine_build.local()}")
    else:
        lines.append(f"  движок:  не найден — {runner.binary_path()}")

    capacity = machine.capacity(os.cpu_count() or 1)
    lines.append(f"  машина:  {capacity.reason}, {capacity.threads} потоков")
    return lines


async def run(options) -> str:
    """The menu. Returns what to do next: `work` or `quit`.

    A loop rather than a wizard: somebody comes back to this to pause their
    machine for an evening, not only once to set it up.
    """
    from dossier.worker import read_pairs

    path = options.config
    pairs = read_pairs(path)

    # Nothing saved and nobody to ask means a first run. Straight to the one
    # question that has to be answered, rather than to a menu of things that
    # cannot work yet.
    if not pairs.get("RENDER_WORKER_TOKEN"):
        _clear()
        _title("Dossier — рендер-воркер")
        print("  Первый запуск. Нужны две вещи, и обе у того, кто позвал тебя")
        print("  в ферму: адрес бота и токен.")
        pairs = await connection(path, pairs, options.name)

    while True:
        _clear()
        _title("Dossier — рендер-воркер")
        for line in await _standing(pairs):
            print(line)
        print()
        print("   1  начать работу")
        print("   2  проверить, всё ли готово")
        print("   3  сколько отдавать этой машины")
        print("   4  подключение — сервер и токен")
        print("   0  выход")

        said = _ask("\n  Что делаем", "1")
        if said == "1":
            if not pairs.get("RENDER_WORKER_TOKEN"):
                print("\n  Сначала токен — пункт 4.")
                _pause()
                continue
            # `main` reads these two from the environment, and `load_config`
            # deliberately will not overwrite a value already there — the real
            # environment beats the file, so that exporting one for a single
            # run means something. But "проверить" a moment ago called
            # `load_config` itself, so the *old* token may be sitting in
            # `os.environ` right now, and the one just typed would be ignored.
            #
            # A setting the program itself changed is changed.
            for key in ("RENDER_SERVER", "RENDER_WORKER_TOKEN"):
                if pairs.get(key):
                    os.environ[key] = pairs[key]
            return "work"
        if said == "0":
            return "quit"
        if said == "2":
            from dossier.worker import check

            _clear()
            await check(options)
            _pause()
        elif said == "3":
            pairs = limits(path, pairs)
        elif said == "4":
            _clear()
            pairs = await connection(path, pairs, options.name)
