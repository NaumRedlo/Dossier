"""The menu: what it writes, what it refuses to interrupt, and what it applies.

Everything a person does in this program now goes through here, including the
one step that has actually gone wrong in the wild — a token that differed from
the server's by a character nobody could see. So the tests are about the file
it writes and the moments it decides something, not about how the screens look.

`_ask` is replaced throughout rather than feeding stdin: what is being tested
is what the program does with an answer, and a test that also has to get the
prompts right breaks every time a word changes.
"""

import os
import sys
import types

import pytest

# The package sits one directory up, beside these tests.
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from dossier import console  # noqa: E402
from dossier.worker import read_pairs  # noqa: E402


def _options(**over):
    settings = types.SimpleNamespace(
        check=False, service=False, once=False, server="",
        config="~/.dossier/worker.env", name="w", polite=False, threads=0,
    )
    for name, value in over.items():
        setattr(settings, name, value)
    return settings


def _answers(monkeypatch, *said):
    """Queue the answers, in order. Anything past the end takes the default."""
    queue = list(said)

    def ask(_prompt, default=""):
        return queue.pop(0) if queue else default

    monkeypatch.setattr(console, "_ask", ask)
    monkeypatch.setattr(console, "_pause", lambda: None)
    monkeypatch.setattr(console, "_clear", lambda: None)
    return queue


# ── when the menu appears at all ─────────────────────────────────────────────


def test_a_service_never_sees_a_menu(monkeypatch):
    """The failure this prevents is a machine that reboots, starts the worker
    from a unit file, and sits at a prompt nobody will ever answer."""
    monkeypatch.setattr(console, "interactive", lambda: True)
    for named in ("check", "service", "once"):
        assert not console.wanted(_options(**{named: True}), []), named


def test_naming_a_bot_on_the_command_line_is_not_a_question(monkeypatch):
    """`--server` says which bot, which is the one thing the setup screen asks.
    Somebody who has said it is not asking."""
    monkeypatch.setattr(console, "interactive", lambda: True)
    assert not console.wanted(_options(), ["--server", "https://x"])
    assert not console.wanted(_options(), ["--server=https://x"])


def test_saying_how_to_work_is_not_saying_whether_to_start(monkeypatch):
    monkeypatch.setattr(console, "interactive", lambda: True)
    assert console.wanted(_options(polite=True), ["--polite", "--threads", "4"])


def test_a_pipe_is_not_a_person(monkeypatch):
    """Both directions: output being captured should not have a menu drawn
    into it, and input from a pipe cannot answer — it would read end-of-file
    for ever."""
    class NotATerminal:
        def isatty(self):
            return False

    monkeypatch.setattr(console.sys, "stdin", NotATerminal())
    assert not console.interactive()
    monkeypatch.setattr(console.sys, "stdin", sys.__stdin__)
    monkeypatch.setattr(console.sys, "stdout", NotATerminal())
    assert not console.interactive()


def test_a_closed_stream_is_not_a_crash(monkeypatch):
    """Frozen on Windows without a console, `sys.stdin` can be `None`."""
    monkeypatch.setattr(console.sys, "stdin", None)
    assert not console.interactive()


# ── the file it writes ───────────────────────────────────────────────────────


def test_what_it_writes_is_what_it_reads_back(tmp_path):
    path = str(tmp_path / "worker.env")
    console.write_pairs(path, {
        "RENDER_SERVER": "https://example.org",
        "RENDER_WORKER_TOKEN": "a-token",
        "RENDER_POLITE": "1",
        "RENDER_HOURS": "22-6",
    })
    back = read_pairs(path)
    assert back["RENDER_SERVER"] == "https://example.org"
    assert back["RENDER_WORKER_TOKEN"] == "a-token"
    assert back["RENDER_POLITE"] == "1"
    assert back["RENDER_HOURS"] == "22-6"


def test_a_setting_it_does_not_know_about_survives(tmp_path):
    """Somebody may have put `DOSSIER_FFMPEG` in here by hand. Losing it
    because a menu had never heard the name would be the menu doing harm."""
    path = str(tmp_path / "worker.env")
    console.write_pairs(path, {
        "RENDER_WORKER_TOKEN": "t",
        "DOSSIER_FFMPEG": "/opt/homebrew/bin/ffmpeg",
    })
    assert read_pairs(path)["DOSSIER_FFMPEG"] == "/opt/homebrew/bin/ffmpeg"


def test_an_empty_limit_is_written_as_a_comment(tmp_path):
    """So the knob is visible to somebody reading the file, without being set.
    `RENDER_THREADS=` and no `RENDER_THREADS` line mean the same to the reader
    and different things to a person."""
    path = str(tmp_path / "worker.env")
    console.write_pairs(path, {"RENDER_WORKER_TOKEN": "t"})
    body = open(path, encoding="utf-8").read()
    assert "# RENDER_THREADS=" in body
    assert "RENDER_THREADS" not in read_pairs(path)


@pytest.mark.skipif(sys.platform == "win32", reason="modes do not mean this there")
def test_the_file_with_the_token_in_it_is_not_readable_by_everybody(tmp_path):
    path = str(tmp_path / "worker.env")
    console.write_pairs(path, {"RENDER_WORKER_TOKEN": "a-secret"})
    assert oct(os.stat(path).st_mode)[-3:] == "600"


def test_it_makes_the_folder_it_was_pointed_at(tmp_path):
    """`~/.dossier` does not exist on a machine that has never run this, and
    telling somebody to create a dot-directory was a step in the old way."""
    path = str(tmp_path / "not" / "yet" / "worker.env")
    console.write_pairs(path, {"RENDER_WORKER_TOKEN": "t"})
    assert os.path.isfile(path)


# ── the limits screen ────────────────────────────────────────────────────────


def test_politeness_toggles_and_is_saved_at_once(monkeypatch, tmp_path):
    """Saved on each change rather than on the way out: somebody who pauses
    their machine and closes the window has paused their machine."""
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "1", "0")
    after = console.limits(path, {"RENDER_WORKER_TOKEN": "t"})
    assert after["RENDER_POLITE"] == "1"
    assert read_pairs(path)["RENDER_POLITE"] == "1"


def test_politeness_toggles_back(monkeypatch, tmp_path):
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "1", "0")
    after = console.limits(path, {"RENDER_WORKER_TOKEN": "t", "RENDER_POLITE": "1"})
    assert not after["RENDER_POLITE"]


def test_a_thread_count_that_is_not_a_number_is_no_limit(monkeypatch, tmp_path):
    """Rather than a limit of zero, which `machine` would read as "no threads"
    and which would stop the machine working entirely."""
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "2", "сколько-нибудь", "0")
    after = console.limits(path, {"RENDER_WORKER_TOKEN": "t"})
    assert after["RENDER_THREADS"] == ""


def test_a_thread_count_of_zero_is_no_limit_too(monkeypatch, tmp_path):
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "2", "0", "0")
    after = console.limits(path, {"RENDER_WORKER_TOKEN": "t"})
    assert after["RENDER_THREADS"] == ""


def test_a_real_thread_count_is_kept(monkeypatch, tmp_path):
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "2", "4", "0")
    assert console.limits(path, {"RENDER_WORKER_TOKEN": "t"})["RENDER_THREADS"] == "4"


def test_an_answer_that_is_not_on_the_menu_changes_nothing(monkeypatch, tmp_path):
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "нет такого пункта", "0")
    was = {"RENDER_WORKER_TOKEN": "t", "RENDER_POLITE": "1"}
    assert console.limits(path, was) == was


# ── the connection screen, which is the one that has gone wrong in the wild ──


async def test_a_token_the_bot_refuses_is_not_saved_by_accident(monkeypatch, tmp_path):
    """Two friends once ran for days against a token that differed from the
    server's, and the only symptom was work that never arrived. It is checked
    before it is written, and a refusal takes an explicit "да" to keep."""
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "https://example.org", "wrong-token", "нет")

    async def refuses(_server, _token, _name):
        return False, "the bot does not know this token"

    monkeypatch.setattr(console, "_try_the_bot", refuses)
    after = await console.connection(path, {})
    assert "RENDER_WORKER_TOKEN" not in after
    assert not os.path.exists(path), "nothing should have been written"


async def test_a_refused_token_can_still_be_kept_on_purpose(monkeypatch, tmp_path):
    """The bot may be down, or behind. Refusing to save is a worse answer than
    saying so and letting somebody decide."""
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "https://example.org", "f" * 64, "да")

    async def refuses(_server, _token, _name):
        return False, "could not reach the bot"

    monkeypatch.setattr(console, "_try_the_bot", refuses)
    after = await console.connection(path, {})
    assert after["RENDER_WORKER_TOKEN"] == "f" * 64
    assert read_pairs(path)["RENDER_WORKER_TOKEN"] == "f" * 64


async def test_a_token_the_bot_accepts_is_saved(monkeypatch, tmp_path):
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "https://example.org", "e" * 64)

    async def accepts(_server, _token, _name):
        return True, "reached, and the builds agree"

    monkeypatch.setattr(console, "_try_the_bot", accepts)
    after = await console.connection(path, {})
    assert after["RENDER_WORKER_TOKEN"] == "e" * 64
    assert read_pairs(path)["RENDER_SERVER"] == "https://example.org"


async def test_an_empty_token_is_refused_without_asking_the_bot(monkeypatch, tmp_path):
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "https://example.org", "")

    async def never(_server, _token, _name):
        raise AssertionError("it asked the bot about nothing")

    monkeypatch.setattr(console, "_try_the_bot", never)
    assert "RENDER_WORKER_TOKEN" not in await console.connection(path, {})


async def test_the_token_is_never_printed(monkeypatch, tmp_path, capsys):
    """It is shown as a fingerprint. Somebody comparing theirs with the
    server's needs to see that they differ, not to see the string — and a
    screen is a thing people photograph and paste into chats."""
    path = str(tmp_path / "worker.env")
    secret = "a-secret-nobody-should-ever-see-in-full"
    _answers(monkeypatch, "https://example.org", secret)

    async def accepts(_server, _token, _name):
        return True, "fine"

    monkeypatch.setattr(console, "_try_the_bot", accepts)
    await console.connection(path, {"RENDER_WORKER_TOKEN": secret})
    assert secret not in capsys.readouterr().out


# ── the menu loop ────────────────────────────────────────────────────────────


async def test_a_first_run_asks_for_the_connection_before_anything_else(
    monkeypatch, tmp_path
):
    """A menu of things that cannot work yet is a worse first screen than the
    one question that has to be answered."""
    path = str(tmp_path / "worker.env")
    asked = []

    async def connection(_path, pairs, _name="w"):
        asked.append("connection")
        return {**pairs, "RENDER_WORKER_TOKEN": "t", "RENDER_SERVER": "https://x"}

    monkeypatch.setattr(console, "connection", connection)
    monkeypatch.setattr(console, "_standing", _no_standing)
    _answers(monkeypatch, "1")

    assert await console.run(_options(config=path)) == "work"
    assert asked == ["connection"]


async def test_starting_work_applies_the_token_that_was_just_typed(
    monkeypatch, tmp_path
):
    """`load_config` will not overwrite a value already in the environment —
    the real environment beats the file on purpose. But "проверить" calls it,
    so the *old* token may be sitting there, and the one just typed would be
    quietly ignored. A setting the program itself changed is changed.
    """
    path = str(tmp_path / "worker.env")
    console.write_pairs(path, {
        "RENDER_SERVER": "https://new", "RENDER_WORKER_TOKEN": "new-token",
    })
    monkeypatch.setenv("RENDER_WORKER_TOKEN", "stale-token")
    monkeypatch.setattr(console, "_standing", _no_standing)
    _answers(monkeypatch, "1")

    assert await console.run(_options(config=path)) == "work"
    assert os.environ["RENDER_WORKER_TOKEN"] == "new-token"
    assert os.environ["RENDER_SERVER"] == "https://new"


async def test_quitting_says_so(monkeypatch, tmp_path):
    path = str(tmp_path / "worker.env")
    console.write_pairs(path, {"RENDER_WORKER_TOKEN": "t"})
    monkeypatch.setattr(console, "_standing", _no_standing)
    _answers(monkeypatch, "0")
    assert await console.run(_options(config=path)) == "quit"


async def test_work_cannot_be_started_without_a_token(monkeypatch, tmp_path):
    """The menu is reachable with no token — somebody may have cleared it — and
    starting then would end in the refusal this whole screen exists to avoid."""
    path = str(tmp_path / "worker.env")
    console.write_pairs(path, {"RENDER_WORKER_TOKEN": "t"})
    monkeypatch.setattr(console, "_standing", _no_standing)
    # Pretend the file lost its token between the write and the read.
    monkeypatch.setattr("dossier.worker.read_pairs", lambda _path: {"RENDER_SERVER": "x"})

    async def connection(_path, pairs, _name="w"):
        return pairs  # the person backed out without giving one

    monkeypatch.setattr(console, "connection", connection)
    _answers(monkeypatch, "1", "0")
    assert await console.run(_options(config=path)) == "quit"


async def _no_standing(_pairs):
    """The four lines at the top run the engine and read the battery. Neither
    is what any of these tests is about."""
    return ["  (состояние)"]


def test_the_stand_in_options_answer_everything_the_bot_check_will_ask():
    """`_try_the_bot` hands `_ask_the_bot` a namespace it builds by hand, and a
    missing attribute there fails as "не удалось спросить бота" — which reads
    as the bot being unreachable rather than as this line being wrong. It cost
    a real run to find, because every test had replaced the function.

    Read off the source rather than listed here, so an attribute added to
    `_ask_the_bot` is one this starts asking about on its own.
    """
    import inspect
    import re as _re

    from dossier import worker

    wants = set(_re.findall(r"options\.(\w+)", inspect.getsource(worker._ask_the_bot)))
    built = set(_re.findall(
        r"(\w+)=", inspect.getsource(console._try_the_bot).split("SimpleNamespace(")[1]
    ))
    missing = wants - built
    assert not missing, (
        f"_ask_the_bot reads options.{', options.'.join(sorted(missing))} and "
        f"_try_the_bot does not supply it"
    )


# ── the window that closes with the program ──────────────────────────────────


def test_nothing_is_held_open_on_a_system_that_does_not_do_that(monkeypatch):
    """Only Windows makes a console for a double-clicked program and destroys
    it with the process. Holding a terminal anywhere else is a keypress in the
    way of somebody who was never going to lose the output."""
    monkeypatch.setattr(console.sys, "platform", "darwin")
    assert not console.own_console()
    monkeypatch.setattr(console.sys, "platform", "linux")
    assert not console.own_console()


def test_a_console_with_a_shell_in_it_is_not_held(monkeypatch):
    """Two processes attached means somebody started this from a terminal, and
    that terminal was there first and stays."""
    import types

    monkeypatch.setattr(console.sys, "platform", "win32")
    kernel = types.SimpleNamespace(GetConsoleProcessList=lambda _slots, _n: 2)
    monkeypatch.setitem(
        sys.modules, "ctypes", types.SimpleNamespace(
            c_uint=int, windll=types.SimpleNamespace(kernel32=kernel),
        ),
    )
    assert not console.own_console()


def test_holding_is_never_a_reason_to_fail(monkeypatch):
    """A program that will not finish because it could not wait for a keypress
    is worse than one whose last line was missed."""
    monkeypatch.setattr(console, "own_console", lambda: True)
    monkeypatch.setattr(console, "interactive", lambda: True)

    def refuses(_prompt):
        raise EOFError

    monkeypatch.setattr("builtins.input", refuses)
    console.hold_the_window()  # the assertion is that this returns


def test_a_window_is_never_held_when_there_is_no_terminal(monkeypatch):
    """Belt and braces over `own_console`. No arrangement of pipes should be
    able to leave a build waiting for a keypress that will not come — a run
    that hangs is worse than a message that scrolled."""
    monkeypatch.setattr(console, "own_console", lambda: True)
    monkeypatch.setattr(console, "interactive", lambda: False)

    def never(_prompt):
        raise AssertionError("it waited with nobody there")

    monkeypatch.setattr("builtins.input", never)
    console.hold_the_window()


# ── the code, which is what anybody actually types now ───────────────────────
#
# A sixty-four character token pasted out of a chat was the step that went
# wrong twice. The bot hands out eight characters instead, good for ten minutes
# and one machine, and the program swaps them for a token nobody ever sees.


@pytest.mark.parametrize("said, is_code", [
    ("ABCD-EFGH", True),
    ("abcd efgh", True),
    ("K7M2QPRS", True),
    ("f" * 64, False),
    ("", False),
])
def test_short_is_a_code_and_long_is_a_token(said, is_code):
    """One field and no question about which they have — which is a question
    that means nothing to the person being asked."""
    assert console.looks_like_a_code(said) is is_code


async def test_a_code_is_swapped_for_a_token_and_the_token_is_saved(
    monkeypatch, tmp_path
):
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "https://example.org", "ABCD-EFGH")
    asked = {}

    async def swapped(server, code, name):
        asked.update(server=server, code=code)
        return "t" * 64, ""

    async def accepts(_server, _token, _name):
        return True, "fine"

    monkeypatch.setattr(console, "redeem", swapped)
    monkeypatch.setattr(console, "_try_the_bot", accepts)

    after = await console.connection(path, {})
    assert asked["code"] == "ABCD-EFGH", "the code goes as typed; the bot tidies it"
    assert after["RENDER_WORKER_TOKEN"] == "t" * 64
    assert read_pairs(path)["RENDER_WORKER_TOKEN"] == "t" * 64


async def test_a_code_the_bot_will_not_take_saves_nothing(monkeypatch, tmp_path):
    """Wrong, used or expired all read the same from here, and none of them is
    something to write down."""
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "https://example.org", "ABCD-EFGH")

    async def refused(_server, _code, _name):
        return "", "код не подошёл"

    async def never(*_a):
        raise AssertionError("it went on to ask about a token it never got")

    monkeypatch.setattr(console, "redeem", refused)
    monkeypatch.setattr(console, "_try_the_bot", never)

    assert "RENDER_WORKER_TOKEN" not in await console.connection(path, {})
    assert not os.path.exists(path)


async def test_keeping_the_existing_key_redeems_nothing(monkeypatch, tmp_path):
    """Enter on what is already there means "leave it", and a token is not a
    code however short somebody's old one happens to be."""
    path = str(tmp_path / "worker.env")
    _answers(monkeypatch, "https://example.org", "kept")

    async def never(*_a):
        raise AssertionError("it tried to redeem the token it already had")

    async def accepts(_server, _token, _name):
        return True, "fine"

    monkeypatch.setattr(console, "redeem", never)
    monkeypatch.setattr(console, "_try_the_bot", accepts)

    after = await console.connection(path, {"RENDER_WORKER_TOKEN": "kept"})
    assert after["RENDER_WORKER_TOKEN"] == "kept"
