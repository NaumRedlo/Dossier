import ast
import asyncio
import inspect
import os
import sys
import types

import pytest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from dossier import maps, runner

WORKER = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "dossier", "worker.py",
)

def _engine_call_keywords() -> set[str]:
    tree = ast.parse(open(WORKER).read())
    for node in ast.walk(tree):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id == "engine"
        ):
            return {kw.arg for kw in node.keywords if kw.arg}
    raise AssertionError("the worker no longer calls the engine as `engine(...)`")

@pytest.mark.parametrize("command", ["video", "exhibit"])
def test_both_engine_commands_accept_the_call_the_worker_makes(command):
    signature = inspect.signature(getattr(runner, command))
    missing = _engine_call_keywords() - set(signature.parameters)
    assert not missing, f"runner.{command} does not take {sorted(missing)}"

def test_the_resource_controls_reached_the_reel_too():
    for command in ("video", "exhibit"):
        takes = set(inspect.signature(getattr(runner, command)).parameters)
        assert {"threads", "encoder_threads", "polite"} <= takes, command

class FakeServer:

    def __init__(self) -> None:
        self.handed_back: list[tuple[str, str]] = []

    async def fetch_replay(self, job_id, into):
        open(into, "wb").write(b"")

    async def heartbeat(self, job_id, progress=None):
        return True

    async def give_back(self, job_id, reason):
        self.handed_back.append((job_id, reason))

class Capacity:
    take, reason, threads, encoder_threads, polite = True, "idle", 4, 2, False

def _run_one_job(monkeypatch, failure: BaseException):
    import importlib.util

    spec = importlib.util.spec_from_file_location("render_worker", WORKER)
    worker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(worker)

    async def inspect_replay(_path):
        return {"beatmap_hash": "abc"}

    async def ensure_map(_api, _hash):
        return None

    async def explode(*_args, **_kw):
        raise failure

    monkeypatch.setattr(worker.runner, "inspect", inspect_replay)
    monkeypatch.setattr(worker.maps, "ensure_known", ensure_map)
    monkeypatch.setattr(worker.maps, "songs_dir", lambda: "/tmp")
    monkeypatch.setattr(worker.runner, "video", explode)
    monkeypatch.setattr(worker, "POLL_SECONDS", 0)

    server = FakeServer()

    job = {"id": "j1", "title": "x", "assets": [],
           "settings": {"kind": "video", "beatmap": {"id": 7, "beatmapset_id": 42}}}
    asyncio.run(worker._render(server, job, Capacity()))
    return server

def test_a_bug_in_the_worker_hands_the_job_back_rather_than_killing_it(monkeypatch):
    server = _run_one_job(monkeypatch, TypeError("unexpected keyword argument"))
    assert [job for job, _ in server.handed_back] == ["j1"]
    assert "unexpected keyword" in server.handed_back[0][1]

def test_a_render_that_fails_the_expected_way_still_says_only_what_went_wrong(monkeypatch):
    server = _run_one_job(monkeypatch, runner.DossierError("карта не открывается"))
    assert server.handed_back == [("j1", "карта не открывается")]

def test_the_worker_is_still_standing_afterwards(monkeypatch):
    server = _run_one_job(monkeypatch, TypeError("boom"))
    again = _run_one_job(monkeypatch, maps.MapUnavailable("нет карты"))
    assert len(server.handed_back) == 1 and len(again.handed_back) == 1

def test_the_machine_is_held_awake_for_exactly_the_render(monkeypatch):
    from dossier import machine

    monkeypatch.setattr(machine.sys, "platform", "darwin")
    monkeypatch.setattr(machine.os, "access", lambda *_: True)
    assert machine.wakeful()[0].endswith("caffeinate")
    assert "-i" in machine.wakeful(), "idle sleep"
    assert "-s" in machine.wakeful(), "and system sleep, which is the reported case"

def test_a_linux_without_systemd_is_not_a_machine_that_cannot_render(monkeypatch):
    from dossier import machine

    monkeypatch.setattr(machine.sys, "platform", "linux")
    monkeypatch.setattr(machine.shutil, "which", lambda _: None)
    assert machine.wakeful() == ()

def test_a_missing_caffeinate_is_not_an_error(monkeypatch):
    from dossier import machine

    monkeypatch.setattr(machine.sys, "platform", "darwin")
    monkeypatch.setattr(machine.os, "access", lambda *_: False)
    assert machine.wakeful() == ()

def test_both_engine_commands_take_the_wrapper():
    import inspect

    for command in ("video", "exhibit"):
        takes = set(inspect.signature(getattr(runner, command)).parameters)
        assert "prefix" in takes, command

def test_losing_the_job_mid_render_stops_the_render(monkeypatch):
    import importlib.util

    spec = importlib.util.spec_from_file_location("render_worker", WORKER)
    worker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(worker)

    cancelled = asyncio.Event()

    async def slow_render(*_args, **_kw):
        try:
            await asyncio.sleep(30)
        except asyncio.CancelledError:
            cancelled.set()
            raise
        raise AssertionError("the render was allowed to finish")

    async def inspect_replay(_path):
        return {"beatmap_hash": "abc"}

    class Sleeper(FakeServer):

        async def heartbeat(self, job_id, progress=None):
            return False

    monkeypatch.setattr(worker.runner, "inspect", inspect_replay)
    monkeypatch.setattr(worker.maps, "ensure_known", lambda *_: asyncio.sleep(0))
    monkeypatch.setattr(worker.maps, "songs_dir", lambda: "/tmp")
    monkeypatch.setattr(worker.runner, "video", slow_render)
    monkeypatch.setattr(worker, "POLL_SECONDS", 0)
    monkeypatch.setattr(worker, "HEARTBEAT_SECONDS", 0.01)

    server = Sleeper()

    job = {"id": "j1", "title": "x", "assets": [],
           "settings": {"kind": "video", "beatmap": {"id": 7, "beatmapset_id": 42}}}
    asyncio.run(worker._render(server, job, Capacity()))

    assert cancelled.is_set(), "the engine was left running"
    assert server.handed_back == [], "there is nothing to hand back — it is gone"

def test_a_linux_battery_is_read_from_its_two_files():
    from dossier import machine

    assert machine.parse_linux_battery("42", "Discharging") == (True, 42)

    assert machine.parse_linux_battery("100", "Full") == (False, 100)
    assert machine.parse_linux_battery("87", "Charging") == (False, 87)

def test_a_machine_that_will_not_say_is_treated_as_plugged_in():
    from dossier import machine

    assert machine.parse_linux_battery("", "") == (False, 100)
    assert machine.parse_linux_battery("nonsense", "Unknown") == (False, 100)

def test_a_percent_outside_the_range_is_brought_back_into_it():
    from dossier import machine

    assert machine.parse_linux_battery("140", "Discharging")[1] == 100
    assert machine.parse_linux_battery("-5", "Discharging")[1] == 0

def test_the_policy_is_the_same_decision_on_every_platform(monkeypatch):
    from dossier import machine

    for platform_name in ("darwin", "win32", "linux"):
        monkeypatch.setattr(machine.sys, "platform", platform_name)
        monkeypatch.setattr(machine, "_run", lambda *_: "")
        monkeypatch.setattr(machine, "_windows_battery", lambda: (True, 5))
        monkeypatch.setattr(machine, "_windows_idle_seconds", lambda: 900.0)
        monkeypatch.setattr(machine, "_linux_battery", lambda: (True, 5))
        monkeypatch.setattr(machine, "parse_battery", lambda _: (True, 5))

        assert not machine.capacity(8).take, platform_name

def test_asking_for_less_of_the_machine_does_not_assume_a_path():
    from dossier import runner

    prefix = runner._polite_prefix()
    assert prefix == () or (prefix[0].endswith("nice") and prefix[1:] == ("-n", "10"))

def test_the_worker_names_itself_without_os_uname():

    called = {
        f"{node.func.value.id}.{node.func.attr}"
        for node in ast.walk(ast.parse(open(WORKER).read()))
        if isinstance(node, ast.Call)
        and isinstance(node.func, ast.Attribute)
        and isinstance(node.func.value, ast.Name)
    }
    assert "os.uname" not in called
    assert "platform.node" in called

def _worker_module():
    import importlib.util

    spec = importlib.util.spec_from_file_location("render_worker", WORKER)
    worker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(worker)
    return worker

def test_the_settings_can_live_in_a_file_instead_of_a_shell(tmp_path, monkeypatch):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    written.write_text(
        "# a worker\n"
        "RENDER_SERVER=https://example.org\n"
        "\n"
        'RENDER_WORKER_TOKEN="quoted-because-it-was-pasted"\n'
        "DOSSIER_CRF = 12345 \n"
    )
    for key in ("RENDER_SERVER", "RENDER_WORKER_TOKEN", "DOSSIER_CRF"):
        monkeypatch.delenv(key, raising=False)

    assert worker.load_config(str(written)) == str(written)
    assert os.environ["RENDER_SERVER"] == "https://example.org"
    assert os.environ["RENDER_WORKER_TOKEN"] == "quoted-because-it-was-pasted"
    assert os.environ["DOSSIER_CRF"] == "12345", "spaces round the = are not the value"

def test_a_variable_set_for_one_run_beats_the_file(tmp_path, monkeypatch):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    written.write_text("RENDER_SERVER=https://the-file.example\n")
    monkeypatch.setenv("RENDER_SERVER", "https://the-shell.example")

    worker.load_config(str(written))
    assert os.environ["RENDER_SERVER"] == "https://the-shell.example"

def test_no_config_file_is_not_a_failure(tmp_path):
    assert _worker_module().load_config(str(tmp_path / "nothing-here")) is None

def test_a_build_mismatch_stands_by_rather_than_killing_the_worker(monkeypatch):
    worker = _worker_module()
    asked = []

    class Server:
        async def __aenter__(self):
            return self

        async def __aexit__(self, *_):
            return False

        async def claim(self, engine, capacity=None):
            asked.append(engine)
            if engine == "stale":
                raise worker.BuildMismatch("the bot renders with aaa and this worker bbb")
            raise SystemExit("agreed, and that is all this test needed")

    answers = iter(["stale", "stale", "fresh"])
    monkeypatch.setattr(worker, "Server", lambda *_a, **_k: Server())
    monkeypatch.setattr(worker.engine_build, "local",
                        lambda **_kw: _resolved(next(answers)))
    monkeypatch.setattr(worker.machine, "capacity", lambda _cores, **_kw: Capacity())
    monkeypatch.setattr(worker, "MISMATCH_SECONDS", 0)
    monkeypatch.setattr(worker, "POLL_SECONDS", 0)

    options = types.SimpleNamespace(server="x", name="w", once=False,
                                    polite=False, threads=0,
                                    config="/nonexistent")
    with pytest.raises(SystemExit):
        asyncio.run(worker._watch(options, "token"))

    assert asked == ["stale", "stale", "fresh"], (
        "the worker either died on the mismatch or never asked its binary again"
    )

def test_one_shot_still_gives_up_on_a_mismatch(monkeypatch):
    worker = _worker_module()

    class Server:
        async def __aenter__(self):
            return self

        async def __aexit__(self, *_):
            return False

        async def claim(self, _engine, _capacity=None):
            raise worker.BuildMismatch("they differ")

    monkeypatch.setattr(worker, "Server", lambda *_a, **_k: Server())
    monkeypatch.setattr(worker.engine_build, "local", lambda **_kw: _resolved("x"))
    monkeypatch.setattr(worker.machine, "capacity", lambda _cores, **_kw: Capacity())
    monkeypatch.setattr(worker, "MISMATCH_SECONDS", 0)

    options = types.SimpleNamespace(server="x", name="w", once=True,
                                    polite=False, threads=0,
                                    config="/nonexistent")
    with pytest.raises(SystemExit, match="cannot take work"):
        asyncio.run(worker._watch(options, "token"))

def _resolved(value):
    async def answer():
        return value

    return answer()

def test_a_failure_that_keeps_happening_is_named(monkeypatch, caplog):
    worker = _worker_module()
    server = _run_one_job(monkeypatch, runner.DossierError("ffmpeg: not found"))
    assert server.handed_back, "the job still goes back"

    import logging

    with caplog.at_level(logging.WARNING):
        worker.hint(runner.DossierError("ffmpeg: not found"))
    assert "PATH" in caplog.text

def _options(tmp_path, **over):
    base = dict(polite=False, threads=0, config=str(tmp_path / "worker.env"))
    base.update(over)
    return types.SimpleNamespace(**base)

def test_a_machine_can_be_paused_from_a_text_editor(tmp_path):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    options = _options(tmp_path)

    written.write_text("RENDER_PAUSE=1\n")
    assert worker.asked_for(str(written), options).closed(13) == "приостановлено владельцем"

    written.write_text("RENDER_PAUSE=0\n")
    assert worker.asked_for(str(written), options).closed(13) is None

def test_the_file_can_take_back_what_the_command_line_said(tmp_path):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    options = _options(tmp_path, polite=True, threads=4)

    written.write_text("")
    assert worker.asked_for(str(written), options).polite is True

    written.write_text("RENDER_POLITE=0\nRENDER_THREADS=16\n")
    later = worker.asked_for(str(written), options)
    assert later.polite is False and later.threads == 16

def test_the_limits_are_not_read_through_the_environment(tmp_path, monkeypatch):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    monkeypatch.setenv("RENDER_PAUSE", "1")

    written.write_text("RENDER_PAUSE=0\n")
    assert worker.asked_for(str(written), _options(tmp_path)).paused is False, (
        "a stale environment variable outranked the file"
    )

def test_no_file_leaves_the_command_line_standing(tmp_path):
    worker = _worker_module()
    options = _options(tmp_path, polite=True, threads=6)
    got = worker.asked_for(str(tmp_path / "nothing-here"), options)
    assert got.polite is True and got.threads == 6 and got.closed(13) is None

def test_a_paused_worker_still_says_hello(monkeypatch):
    worker = _worker_module()
    heard = []

    class Server:
        async def __aenter__(self):
            return self

        async def __aexit__(self, *_):
            return False

        async def claim(self, _engine, capacity=None):
            heard.append(capacity)
            raise SystemExit("said hello, and that is all this test needed")

    monkeypatch.setattr(worker, "Server", lambda *_a, **_k: Server())
    monkeypatch.setattr(worker.engine_build, "local", lambda **_kw: _resolved("x"))
    monkeypatch.setattr(worker, "asked_for",
                        lambda *_a: worker.machine.Limits(paused=True))
    monkeypatch.setattr(worker, "POLL_SECONDS", 0)

    options = types.SimpleNamespace(server="x", name="w", once=False,
                                    polite=False, threads=0, config="/nonexistent")
    with pytest.raises(SystemExit):
        asyncio.run(worker._watch(options, "token"))

    assert heard and heard[0].take is False
    assert heard[0].reason == "приостановлено владельцем", (
        "the farm view needs the reason, not just the refusal"
    )

def test_the_loop_re_reads_the_limits_rather_than_remembering_them(tmp_path):
    import importlib.util
    import pytest as _pytest

    spec = importlib.util.spec_from_file_location("render_worker", WORKER)
    worker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(worker)

    written = tmp_path / "worker.env"
    written.write_text("RENDER_PAUSE=1\n")
    said = []

    class Server:
        async def __aenter__(self):
            return self

        async def __aexit__(self, *_):
            return False

        async def claim(self, _engine, capacity=None):
            said.append(capacity.take)
            if len(said) == 1:

                written.write_text("RENDER_PAUSE=0\n")
                return None
            raise SystemExit("two polls is the whole of the test")

    import types as _types

    worker.Server = lambda *_a, **_k: Server()
    worker.engine_build.local = lambda **_kw: _resolved("x")
    worker.machine.capacity = lambda _cores, **_kw: Capacity()
    worker.POLL_SECONDS = 0
    worker.RESTING_SECONDS = 0

    options = _types.SimpleNamespace(server="x", name="w", once=False,
                                     polite=False, threads=0, config=str(written))
    with _pytest.raises(SystemExit):
        asyncio.run(worker._watch(options, "token"))

    assert said == [False, True], (
        f"the worker read its limits once and kept them: {said}"
    )

def test_an_engine_that_refuses_to_start_reports_why_and_not_its_usage():
    from dossier.runner import _why_it_failed

    report = [
        "dossier: `video` has no option `--meter` — see `dossier video --help`",
        "dossier video [OPTIONS] <replay.osr>",
        "Options:",
        "--game-sounds <dir>      osu!'s own sounds, for what a skin leaves out",
        "--kit <name>             click, soft, drum, glass or wood",
        "--pitch <x>              multiply every hit-sound frequency",
        "--decay <x>              multiply every hit-sound decay",
        "--level <x>              multiply hit-sound loudness",
        "-h, --help               this text",
    ]
    said = _why_it_failed(report)
    assert "has no option `--meter`" in said

    for listed in ("--kit", "--pitch", "--decay", "--level", "--game-sounds"):
        assert listed not in said, (
            f"the usage is an appendix, never a reason:\n{said}"
        )
    assert said.count("\n") == 0, f"one line was enough:\n{said}"

def test_an_engine_that_dies_mid_render_still_reports_its_last_words():
    from dossier.runner import _why_it_failed

    report = [
        "reading the replay",
        "judging",
        "drawing",
        "the map's audio would not decode: unsupported codec",
    ]
    assert "unsupported codec" in _why_it_failed(report)

def test_an_engine_that_says_nothing_useful_still_says_something():
    from dossier.runner import _why_it_failed

    assert _why_it_failed(["-h, --help    this text"]), (
        "dropping every line left nothing at all to report"
    )

def test_a_file_saved_by_notepad_is_read_whole(tmp_path):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    written.write_bytes(
        "﻿RENDER_SERVER=https://example.org\nRENDER_WORKER_TOKEN=abc\n".encode()
    )
    pairs = worker.read_pairs(str(written))
    assert pairs.get("RENDER_SERVER") == "https://example.org"
    assert pairs.get("RENDER_WORKER_TOKEN") == "abc"

def test_a_line_copied_out_of_a_browser_is_read(tmp_path):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    written.write_text(" RENDER_WORKER_TOKEN = abc\n")
    assert worker.read_pairs(str(written)).get("RENDER_WORKER_TOKEN") == "abc"

def test_the_check_says_which_keys_the_file_gave_it():
    import inspect

    worker = _worker_module()
    source = inspect.getsource(worker.check)
    assert "в этом файле" in source
    assert "не хватает: " in source

async def test_the_check_finds_a_token_that_lives_only_in_the_file(tmp_path, capsys):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    written.write_text(
        "RENDER_WORKER_TOKEN=from-the-file\n"
        "DOSSIER_CRF=42\nDOSSIER_PRESET=slow\n"
    )
    for key in ("RENDER_WORKER_TOKEN", "RENDER_SERVER"):
        os.environ.pop(key, None)

    options = types.SimpleNamespace(
        config=str(written), server="", name="w", polite=False, threads=0
    )
    await worker.check(options)
    said = capsys.readouterr().out

    assert "токен: нет" not in said, said

    assert "[+] токен: " in said and "знак" in said, said

def test_a_path_is_shown_with_this_systems_own_separators(monkeypatch):
    worker = _worker_module()
    shown = worker.where("~/.dossier/worker.env")
    assert "/" not in shown or "\\" not in shown, f"mixed separators: {shown}"

def test_a_token_can_be_compared_without_being_shown():
    worker = _worker_module()

    same = worker.fingerprint("a-shared-secret")
    assert same == worker.fingerprint("a-shared-secret"), "the same string, twice"
    assert same != worker.fingerprint("a-different-secret")
    assert "a-shared-secret" not in same, "the fingerprint is not the secret"

def test_a_stray_newline_or_quote_shows_up_in_the_length():
    worker = _worker_module()
    plain = worker.fingerprint("abc")
    assert plain.startswith("3 знака")
    assert worker.fingerprint("abc\n").startswith("4 знака")
    assert worker.fingerprint('"abc"').startswith("5 знаков")

def test_nothing_is_fingerprinted_as_nothing():
    assert _worker_module().fingerprint("") == "нет"

def test_the_refusal_carries_the_fingerprint_to_compare():
    import inspect

    worker = _worker_module()
    source = inspect.getsource(worker._ask_the_bot)
    assert "fingerprint(token)" in source
    assert "печатает при запуске" in source

async def test_the_check_says_when_the_environment_is_beating_the_file(
    tmp_path, capsys
):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    written.write_text("RENDER_WORKER_TOKEN=the-one-in-the-file\n")
    os.environ["RENDER_WORKER_TOKEN"] = "a-stale-one"
    os.environ.pop("RENDER_SERVER", None)
    try:
        options = types.SimpleNamespace(
            config=str(written), server="", name="w", polite=False, threads=0
        )
        await worker.check(options)
        said = capsys.readouterr().out
    finally:
        os.environ.pop("RENDER_WORKER_TOKEN", None)

    assert "из окружения, а не из файла" in said, said
    assert worker.fingerprint("the-one-in-the-file") in said, "and what the file holds"
    assert "the-one-in-the-file" not in said, "but never the token itself"

async def test_no_complaint_when_the_two_agree(tmp_path, capsys):
    worker = _worker_module()
    written = tmp_path / "worker.env"
    written.write_text("RENDER_WORKER_TOKEN=agreed\n")
    os.environ["RENDER_WORKER_TOKEN"] = "agreed"
    os.environ.pop("RENDER_SERVER", None)
    try:
        options = types.SimpleNamespace(
            config=str(written), server="", name="w", polite=False, threads=0
        )
        await worker.check(options)
        said = capsys.readouterr().out
    finally:
        os.environ.pop("RENDER_WORKER_TOKEN", None)

    assert "from the environment" not in said

async def test_a_job_that_names_its_map_needs_no_osu_account(monkeypatch, tmp_path):
    worker = _worker_module()
    asked = {"known": None, "looked_up": False}

    async def known(beatmap, checksum):
        asked["known"] = beatmap
        return beatmap

    async def looked_up(_api, _checksum):
        asked["looked_up"] = True
        return {}

    async def inspect_replay(_path):
        return {"beatmap_hash": "abc"}

    async def explode(*_a, **_kw):
        raise runner.DossierError("enough — the map was already fetched")

    monkeypatch.setattr(worker.maps, "ensure_known", known)
    monkeypatch.setattr(worker.maps, "ensure_map", looked_up)
    monkeypatch.setattr(worker.runner, "inspect", inspect_replay)
    monkeypatch.setattr(worker.maps, "songs_dir", lambda: str(tmp_path))
    monkeypatch.setattr(worker.runner, "video", explode)
    monkeypatch.setattr(worker, "POLL_SECONDS", 0)

    job = {
        "id": "j1", "title": "x", "assets": [],
        "settings": {"kind": "video", "beatmap": {"id": 7, "beatmapset_id": 42}},
    }

    await worker._render(FakeServer(), job, Capacity())

    assert asked["known"] == {"id": 7, "beatmapset_id": 42}
    assert not asked["looked_up"], "it asked osu! anyway"

async def test_a_job_it_cannot_do_is_handed_back_before_anything_is_fetched(
    monkeypatch, tmp_path
):
    worker = _worker_module()
    fetched = []

    class Watching(FakeServer):
        async def fetch_replay(self, job_id, into):
            fetched.append("replay")

        async def fetch_asset(self, job_id, name, into):
            fetched.append(name)

    monkeypatch.setattr(worker, "POLL_SECONDS", 0)
    server = Watching()
    job = {
        "id": "j1", "title": "x", "assets": ["a0"],

        "settings": {"kind": "video", "skin": "{{a0}}"},
    }
    await worker._render(server, job, Capacity())

    assert not fetched, f"it fetched {fetched} before finding out it could not"
    assert [job for job, _ in server.handed_back] == ["j1"]
    assert "older than this worker" in server.handed_back[0][1], (
        "and it says what to do about it — the message named osu! credentials, "
        "which is not the thing anybody needs to fix"
    )

def _a_cached_skin(root, name: str, megabytes: int, used_at: float):
    folder = root / name
    folder.mkdir(parents=True)
    (folder / "hitcircle.png").write_bytes(b"\0" * (megabytes * 1024 * 1024))
    os.utime(folder, (used_at, used_at))
    return folder

def test_the_skin_cache_is_kept_under_its_cap(tmp_path, monkeypatch):
    worker = _worker_module()
    monkeypatch.setattr(worker, "_skin_cache", lambda: str(tmp_path))

    import time

    now = time.time()
    _a_cached_skin(tmp_path, "old", 3, now - 90_000)
    _a_cached_skin(tmp_path, "newer", 3, now - 1_000)
    _a_cached_skin(tmp_path, "newest", 3, now)

    dropped = worker.prune_skins(cap=7 * 1024 * 1024)
    left = sorted(p.name for p in tmp_path.iterdir())

    assert dropped == 1
    assert left == ["newer", "newest"], f"the wrong one went: {left}"

def test_nothing_is_dropped_while_there_is_room(tmp_path, monkeypatch):
    worker = _worker_module()
    monkeypatch.setattr(worker, "_skin_cache", lambda: str(tmp_path))

    import time

    _a_cached_skin(tmp_path, "one", 1, time.time())
    assert worker.prune_skins(cap=100 * 1024 * 1024) == 0
    assert [p.name for p in tmp_path.iterdir()] == ["one"]

def test_a_skin_in_daily_use_outlives_an_older_arrival(tmp_path, monkeypatch):
    worker = _worker_module()
    monkeypatch.setattr(worker, "_skin_cache", lambda: str(tmp_path))

    import time

    now = time.time()

    kept = _a_cached_skin(tmp_path, "favourite", 3, now)

    _a_cached_skin(tmp_path, "tried-once", 3, now - 50_000)

    worker.prune_skins(cap=4 * 1024 * 1024)
    assert kept.exists() and not (tmp_path / "tried-once").exists()

def test_a_half_unpacked_skin_is_not_counted_as_one(tmp_path, monkeypatch):
    worker = _worker_module()
    monkeypatch.setattr(worker, "_skin_cache", lambda: str(tmp_path))

    import time

    _a_cached_skin(tmp_path, "abc.incoming", 5, time.time() - 90_000)
    worker.prune_skins(cap=1)
    assert (tmp_path / "abc.incoming").exists()

def _printed_service(monkeypatch, capsys, **options):
    worker = _worker_module()
    settings = types.SimpleNamespace(
        server="https://example.org", name="w", polite=False,
        threads=0, config=worker.CONFIG,
    )
    for name, value in options.items():
        setattr(settings, name, value)
    worker.service(settings)
    return worker, capsys.readouterr().out

def test_a_unit_from_a_checkout_names_the_launcher(monkeypatch, capsys):
    worker, said = _printed_service(monkeypatch, capsys)

    assert sys.executable in said, "it has to name the interpreter it ran under"
    launcher = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(worker.__file__))),
        "worker.py",
    )
    assert launcher in said
    assert os.path.isfile(launcher), "the unit names a file that is not there"

def test_a_unit_from_a_release_names_the_executable_and_not_a_temporary_file(
    monkeypatch, capsys
):
    worker = _worker_module()
    monkeypatch.setattr(worker.sys, "frozen", True, raising=False)
    monkeypatch.setattr(worker.sys, "executable", "/opt/dossier/dossier-worker")

    settings = types.SimpleNamespace(
        server="https://example.org", name="w", polite=False,
        threads=0, config=worker.CONFIG,
    )
    worker.service(settings)
    said = capsys.readouterr().out

    assert "/opt/dossier/dossier-worker" in said
    assert "worker.py" not in said, "a release has no script to run"

    assert "/opt/dossier" in said

def test_the_unit_carries_no_token(monkeypatch, capsys):
    monkeypatch.setenv("RENDER_WORKER_TOKEN", "a-secret-nobody-should-see")
    _worker, said = _printed_service(monkeypatch, capsys)
    assert "a-secret-nobody-should-see" not in said

def test_the_output_is_made_readable_before_anything_is_printed(monkeypatch):
    worker = _worker_module()
    asked = []

    class Stream:
        def reconfigure(self, **how):
            asked.append(how)

    monkeypatch.setattr(worker.sys, "stdout", Stream())
    monkeypatch.setattr(worker.sys, "stderr", Stream())
    worker._readable_output()

    assert asked == [{"encoding": "utf-8", "errors": "replace"}] * 2

def test_a_stream_that_cannot_be_reconfigured_is_not_a_crash(monkeypatch):
    worker = _worker_module()

    class Awkward:
        def reconfigure(self, **_how):
            raise AttributeError("not that kind of stream")

    monkeypatch.setattr(worker.sys, "stdout", Awkward())
    monkeypatch.setattr(worker.sys, "stderr", Awkward())
    worker._readable_output()
