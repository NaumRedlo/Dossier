"""What a worker says while it is running, and what it keeps afterwards.

Neither of these was working. The client had never switched its logging on, so
it printed warnings and nothing else — no "took a job", no "delivered", no
tally — and kept no copy at all, which made "пришли лог" a request nobody could
answer. And a bot that had gone away was announced once a second for as long as
it took to come back: twenty identical lines to scroll past, the last of them
no more informative than the first.

So: a file that is always being written, and one line that moves.
"""

import io
import logging
import os
import sys
import types

import pytest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from dossier import console, log  # noqa: E402


@pytest.fixture(autouse=True)
def unhandled():
    """Handlers are added to a module-level logger, so a test that adds one
    would otherwise be writing into every test after it."""
    root = logging.getLogger(log.ROOT)
    was = list(root.handlers)
    root.handlers.clear()
    yield root
    root.handlers[:] = was


# ── the line that moves ──────────────────────────────────────────────────────


class Screen(io.StringIO):
    def isatty(self):
        return True


def test_a_shorter_line_does_not_leave_the_tail_of_a_longer_one():
    """`Повторяю подключение...` followed by `Готово` would otherwise read as
    `Готово подключение...` — which is a sentence, and a wrong one."""
    screen = Screen()
    line = console.Line(screen)
    long, short = "a very long message indeed", "short"
    line.say(long)
    line.say(short)
    assert screen.getvalue() == f"\r{long}\r{short}{' ' * (len(long) - len(short))}"


def test_clearing_leaves_the_screen_as_it_was():
    screen = Screen()
    line = console.Line(screen)
    line.say("something")
    line.clear()
    assert screen.getvalue().endswith("\r" + " " * len("something") + "\r")
    assert not line.showing


def test_clearing_a_line_that_was_never_shown_writes_nothing():
    screen = Screen()
    console.Line(screen).clear()
    assert screen.getvalue() == ""


def test_a_service_gets_no_carriage_returns_at_all():
    """A journal full of `\\r` is a line nobody can read and a file nobody can
    grep. Everything this line has to say is worth nothing after the moment it
    said it, so when there is nobody watching it says nothing."""
    quiet = io.StringIO()  # a StringIO is not a tty
    line = console.Line(quiet)
    line.say("Потеряно соединение с ботом...")
    line.clear()
    assert quiet.getvalue() == ""
    assert not line.showing


def test_a_stream_that_has_gone_is_not_a_crash():
    """`sys.stderr` can be `None` in a frozen program started without a
    console, and a worker must not die of having nowhere to draw a dot."""
    line = console.Line(None)
    line.say("anything")
    line.clear()


# ── the file that is kept ────────────────────────────────────────────────────


def test_what_is_logged_reaches_the_file(tmp_path):
    path = str(tmp_path / "worker.log")
    assert log.to_file(path) == path
    log.get_logger("test").info("взял задачу")
    assert any("взял задачу" in line for line in log.tail(10, path))


def test_asking_twice_does_not_write_everything_twice(tmp_path):
    path = str(tmp_path / "worker.log")
    log.to_file(path)
    log.to_file(path)
    log.get_logger("test").info("однажды")
    assert sum("однажды" in line for line in log.tail(10, path)) == 1


def test_the_tail_is_the_end_of_it(tmp_path):
    path = str(tmp_path / "worker.log")
    log.to_file(path)
    for at in range(60):
        log.get_logger("test").info("строка %d", at)
    said = log.tail(5, path)
    assert len(said) == 5
    assert "строка 59" in said[-1]


def test_no_log_yet_is_an_empty_list_rather_than_an_error(tmp_path):
    """The menu shows this before a worker has ever run."""
    assert log.tail(10, str(tmp_path / "never-written.log")) == []


def test_a_home_it_cannot_write_to_does_not_stop_the_worker(tmp_path, caplog):
    """A read-only home or a full disk. Worth saying once and not worth
    refusing to render over."""
    taken = tmp_path / "in-the-way"
    taken.write_text("not a directory")
    with caplog.at_level(logging.WARNING):
        assert log.to_file(str(taken / "worker.log")) == ""
    assert "no log file" in caplog.text


def test_the_log_lives_beside_the_settings_and_not_beside_the_program():
    """A release is a folder somebody may move or replace, and a log that goes
    with it is a log that is gone exactly when it is wanted."""
    assert log.FILE.endswith(os.path.join(".dossier", "worker.log"))


# ── the menu's view of it ────────────────────────────────────────────────────


def test_the_journal_screen_shows_the_end_and_names_the_file(
    monkeypatch, tmp_path, capsys
):
    """Shown rather than only pointed at: the answer is usually in the last few
    lines, and opening a file in a folder that begins with a dot is a thing
    people ask how to do. The path is printed anyway — what gets sent to
    somebody who can help is the file."""
    path = str(tmp_path / "worker.log")
    log.to_file(path)
    log.get_logger("test").info("что-то произошло")
    monkeypatch.setattr(log, "FILE", path)
    monkeypatch.setattr(console, "_pause", lambda: None)

    console.journal()
    said = capsys.readouterr().out
    assert "что-то произошло" in said
    assert path in said


def test_an_empty_journal_says_so_rather_than_showing_nothing(
    monkeypatch, tmp_path, capsys
):
    monkeypatch.setattr(log, "FILE", str(tmp_path / "nothing.log"))
    monkeypatch.setattr(console, "_pause", lambda: None)
    console.journal()
    assert "Пока пусто" in capsys.readouterr().out


# ── losing the bot, and getting it back ──────────────────────────────────────


def _a_worker():
    from tests.test_worker import _worker_module

    return _worker_module()


async def test_an_outage_is_one_log_line_however_long_it_lasts(monkeypatch, caplog):
    """It used to be one a second. Twenty identical lines to scroll past, and
    the last of them no more informative than the first — while the one thing
    somebody sends afterwards is the log."""
    worker = _a_worker()
    import aiohttp

    tries = {"n": 0}

    class Server:
        async def __aenter__(self):
            return self

        async def __aexit__(self, *_):
            return False

        async def claim(self, engine, capacity=None):
            tries["n"] += 1
            if tries["n"] <= 5:
                raise aiohttp.ClientError("no route to host")
            if tries["n"] >= 9:
                raise SystemExit("seen enough")
            # Back, and with nothing to do — which is what a bot that has just
            # come up answers, and which the loop has to get past for the
            # coming-back to be noticed at all.
            return None

    monkeypatch.setattr(worker, "Server", lambda *_a, **_k: Server())
    monkeypatch.setattr(worker.engine_build, "local", lambda **_kw: _said())
    monkeypatch.setattr(worker.machine, "capacity", lambda _c, **_kw: _capacity())
    monkeypatch.setattr(worker, "POLL_SECONDS", 0)

    options = types.SimpleNamespace(server="x", name="w", once=False,
                                    polite=False, threads=0, config="/nonexistent")
    with caplog.at_level(logging.INFO), pytest.raises(SystemExit):
        await worker._watch(options, "token")

    lost = [r for r in caplog.records if "lost the bot" in r.message]
    back = [r for r in caplog.records if "the bot is back" in r.message]
    assert len(lost) == 1, [r.message for r in caplog.records]
    assert len(back) == 1, "coming back is worth exactly one line too"


async def _said():
    return "dossier 0.1.0 (abc1234)"


def _capacity():
    from dossier.machine import Capacity

    return Capacity(take=True, reason="idle", threads=4, encoder_threads=2,
                    code="idle", detail="", polite=False)


def test_the_settled_message_does_not_sit_there_being_untrue():
    """Five seconds: long enough to be read by somebody who looked up at the
    right moment, short enough not to still be on the screen an hour later
    claiming something was restored."""
    worker = _a_worker()
    assert 2 <= worker.SETTLED_SECONDS <= 10


