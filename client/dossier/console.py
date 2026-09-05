import os
import shutil
import sys

DEFAULT_SERVER = "https://onenineeightfour.ignorelist.com"

LIMITS = ("RENDER_POLITE", "RENDER_THREADS", "RENDER_PAUSE", "RENDER_HOURS")

def interactive() -> bool:
    try:
        return sys.stdin.isatty() and sys.stdout.isatty()
    except (AttributeError, ValueError):
        return False

def own_console() -> bool:
    if sys.platform != "win32":
        return False
    try:
        import ctypes

        attached = (ctypes.c_uint * 2)()
        return ctypes.windll.kernel32.GetConsoleProcessList(attached, 2) == 1
    except Exception:
        return False

def hold_the_window() -> None:
    if not own_console() or not interactive():
        return
    try:
        input("\n  — Enter, чтобы закрыть окно —")
    except (EOFError, KeyboardInterrupt):
        pass

def wanted(options, given: list[str]) -> bool:
    if not interactive():
        return False
    if options.check or options.service or options.once:
        return False

    return not any(argument.startswith("--server") for argument in given)

class Line:

    def __init__(self, stream=None) -> None:
        self._to = stream if stream is not None else sys.stderr
        self._width = 0
        self._showing = False

    def _live(self) -> bool:
        try:
            return bool(self._to) and self._to.isatty()
        except (AttributeError, ValueError):
            return False

    def say(self, text: str) -> None:
        if not self._live():
            return

        self._to.write("\r" + text + " " * max(0, self._width - len(text)))
        self._to.flush()
        self._width = len(text)
        self._showing = True

    def clear(self) -> None:
        if not self._showing or not self._live():
            self._showing = False
            return
        self._to.write("\r" + " " * self._width + "\r")
        self._to.flush()
        self._width = 0
        self._showing = False

    @property
    def showing(self) -> bool:
        return self._showing

def _width() -> int:
    return max(48, min(78, shutil.get_terminal_size((80, 24)).columns))

def _clear() -> None:
    try:
        if sys.platform == "win32":
            os.system("cls")
        else:

            print("\033[H\033[J", end="")
    except Exception:
        print("\n" * 3)

def _title(text: str) -> None:
    print()
    print(f"  {text}")
    print("  " + "─" * (_width() - 4))

def _ask(prompt: str, default: str = "") -> str:
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

def write_pairs(path: str, pairs: dict[str, str]) -> str:
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

        lines.append(f"{key}={value}" if value else f"# {key}=")
    if rest:
        lines += ["", "# Остальное."]
        lines += [f"{key}={value}" for key, value in sorted(rest.items())]
    lines.append("")

    with open(full, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))

    try:
        os.chmod(full, 0o600)
    except OSError:
        pass
    return full

CODE_AT_MOST = 16

def looks_like_a_code(said: str) -> bool:
    return 0 < len("".join(said.split())) <= CODE_AT_MOST

async def redeem(server: str, code: str, name: str) -> tuple[str, str]:
    import aiohttp

    from dossier.update import trusted

    try:
        async with aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=20),
            connector=aiohttp.TCPConnector(ssl=trusted()),
        ) as session:
            async with session.post(
                f"{server.rstrip('/')}/render/join",
                json={"code": code, "name": _machine_name()},
            ) as reply:
                if reply.status == 200:
                    return (await reply.json()).get("token", ""), ""
                if reply.status == 403:
                    return "", ("код не подошёл — он уже использован, просрочен "
                                "или набран с ошибкой. Попроси новый.")
                if reply.status == 404:
                    return "", ("этот бот ещё не умеет коды — попроси токен "
                                "по-старому")
                return "", f"бот ответил {reply.status}"
    except Exception as exc:
        return "", f"не удалось связаться с ботом: {exc}"

def _machine_name() -> str:
    import platform

    return platform.node() or "worker"

async def _try_the_bot(server: str, token: str, name: str) -> tuple[bool, str]:
    import types

    from dossier import worker

    engine = await worker.engine_build.local(refresh=True)

    settings = types.SimpleNamespace(
        server=server, name=name, config="/nonexistent",
    )
    try:
        checks = await worker._ask_the_bot(settings, token, engine)
    except Exception as exc:
        return False, f"не удалось спросить бота: {exc}"

    for check in checks:
        if check.name == "the bot" and check.ok is False:
            return False, check.said
    said = "; ".join(check.said for check in checks if check.ok)
    return True, said or "бот ответил"

async def connection(path: str, pairs: dict[str, str], name: str = "worker") -> dict[str, str]:
    from dossier.worker import fingerprint

    _title("Подключение")
    print("  Нужен код — попроси его у того, кто позвал тебя в ферму.")
    print("  В боте он берётся командой cltoken.")
    print()

    server = _ask("Адрес бота", pairs.get("RENDER_SERVER") or DEFAULT_SERVER)
    was = pairs.get("RENDER_WORKER_TOKEN", "")
    if was and looks_like_a_code(was):

        print(f"\n  Записано «{was}» — это код, а не ключ.")
        print("  Enter — обменяю его на ключ прямо сейчас.")
    elif was:
        print(f"\n  Сейчас записан ключ: {fingerprint(was)}")
        print("  Enter — оставить как есть.")
    said = _ask("Код", was)

    if not said:
        print("\n  Без кода работать не выйдет — бот не поймёт, кто это.")
        _pause()
        return pairs

    if said == was and not looks_like_a_code(said):

        token = was
    elif looks_like_a_code(said):
        print("\n  Меняю код на ключ…")
        token, why = await redeem(server, said, name)
        if not token:
            print(f"  ✗ {why}")
            _pause()
            return pairs
        print("  ✓ готово — ключ выдан этой машине и записан")
    else:

        token = said

    print("\n  Спрашиваю бота…")
    good, said = await _try_the_bot(server, token, name)
    print(f"  {'✓' if good else '✗'} {said}")
    if not good:

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

def journal(lines: int = 40) -> None:
    from dossier import log

    _title("Журнал")
    said = log.tail(lines)
    if not said:
        print("  Пока пусто — здесь появится то, что происходило во время работы.")
    else:
        for one in said:
            print(f"  {one}")

    print()
    print("  Весь журнал лежит здесь — этот файл и надо присылать:")
    print(f"    {log.FILE}")
    if sys.platform == "darwin":
        print("\n  Открыть папку с ним:  open ~/.dossier")
    elif sys.platform == "win32":
        print("\n  Открыть папку с ним:  explorer %USERPROFILE%\\.dossier")
    _pause()

async def _standing(pairs: dict[str, str]) -> list[str]:
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
    from dossier.worker import read_pairs

    path = options.config
    pairs = read_pairs(path)

    if not pairs.get("RENDER_WORKER_TOKEN"):
        _clear()
        _title("Dossier — рендер-воркер")
        print("  Первый запуск. Нужен код — его выдаёт тот, кто позвал тебя")
        print("  в ферму. Адрес бота уже подставлен.")
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
        print("   4  подключение — сервер и код")
        print("   5  журнал — что было и что сломалось")
        print("   0  выход")

        said = _ask("\n  Что делаем", "1")
        if said == "1":
            if not pairs.get("RENDER_WORKER_TOKEN"):
                print("\n  Сначала токен — пункт 4.")
                _pause()
                continue

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
        elif said == "5":
            _clear()
            journal()
