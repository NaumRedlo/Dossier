"""Fetching the release the bot is on, when this machine is on another one.

A worker whose engine differs from the bot's is turned away, and rightly — a
stale binary renders something that looks right and is not. What it was told to
do about that was `git pull` and `cargo build`, in a checkout most people
running this do not have, because they downloaded a zip.

So the tests here are about the two halves of doing better: getting the right
thing off the internet without anybody watching, and not doing any of it to
somebody's computer without asking.
"""

import hashlib
import io
import os
import sys
import zipfile

import pytest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from dossier import update, worker  # noqa: E402


# ── which machine ────────────────────────────────────────────────────────────


@pytest.mark.parametrize("system, machine, expected", [
    ("linux", "x86_64", "linux-x64"),
    ("darwin", "arm64", "macos-arm64"),
    ("win32", "AMD64", "windows-x64"),
])
def test_each_machine_asks_for_the_release_built_for_it(
    monkeypatch, system, machine, expected
):
    monkeypatch.setattr(update.sys, "platform", system)
    monkeypatch.setattr(update.platform, "machine", lambda: machine)
    assert update.slug() == expected


def test_a_machine_nobody_builds_for_is_told_what_is_built(monkeypatch):
    """A Raspberry Pi. It compiles there; it is simply not shipped there, and
    a download that 404s says none of that."""
    monkeypatch.setattr(update.sys, "platform", "linux")
    monkeypatch.setattr(update.platform, "machine", lambda: "aarch64")
    with pytest.raises(update.Cannot) as refused:
        update.slug()
    assert "linux-x64" in str(refused.value)


def test_the_address_falls_back_to_the_release_page(monkeypatch):
    """Even a machine with no build of its own is better off with somewhere to
    look than with nothing."""
    monkeypatch.setattr(update.sys, "platform", "linux")
    monkeypatch.setattr(update.platform, "machine", lambda: "aarch64")
    assert update.where_to_get_it("v9.9.9").endswith("/releases/tag/v9.9.9")


def test_the_address_names_the_file_for_this_machine(monkeypatch):
    monkeypatch.setattr(update, "slug", lambda: "windows-x64")
    said = update.where_to_get_it("v9.9.9")
    assert said.endswith("/download/v9.9.9/dossier-v9.9.9-windows-x64.zip")


# ── taking it off the internet ───────────────────────────────────────────────


def _a_release(tag: str = "v9.9.9") -> bytes:
    made = io.BytesIO()
    with zipfile.ZipFile(made, "w") as archive:
        archive.writestr(f"dossier-{tag}-linux-x64/dossier", b"the engine")
        archive.writestr(f"dossier-{tag}-linux-x64/dossier-worker", b"the client")
        archive.writestr(f"dossier-{tag}-linux-x64/assets/fonts/keep.txt", b"a font")
    return made.getvalue()


@pytest.fixture
def somewhere(monkeypatch, tmp_path):
    monkeypatch.setattr(update, "HOME", str(tmp_path))
    monkeypatch.setattr(update, "ENGINES", str(tmp_path / "engines"))
    monkeypatch.setattr(update, "slug", lambda: "linux-x64")
    return tmp_path


def _served(monkeypatch, body: bytes, digest: str = ""):
    asked = []

    def fetch(url):
        asked.append(url)
        if url.endswith(".sha256"):
            told = digest or hashlib.sha256(body).hexdigest()
            return f"{told}  a-release.zip\n".encode()
        return body

    monkeypatch.setattr(update, "_fetch", fetch)
    return asked


def test_a_release_arrives_whole_and_runnable(somewhere, monkeypatch):
    _served(monkeypatch, _a_release())
    landing = update.fetch("v9.9.9", say=lambda _s: None)

    for name in ("dossier", "dossier-worker"):
        assert os.path.isfile(os.path.join(landing, name))
        assert os.access(os.path.join(landing, name), os.X_OK)
    # The font travels with it. Without one the engine draws the play and
    # leaves out the score, the accuracy and the combo, and says so only on
    # stderr.
    assert os.path.isfile(os.path.join(landing, "assets", "fonts", "keep.txt"))


def test_an_archive_that_does_not_match_its_hash_is_not_unpacked(
    somewhere, monkeypatch
):
    """This runs on somebody's own computer with nobody watching, and "it
    downloaded" is not "it downloaded the right thing"."""
    _served(monkeypatch, _a_release(), digest="0" * 64)
    with pytest.raises(update.Cannot) as refused:
        update.fetch("v9.9.9", say=lambda _s: None)
    assert "не совпало" in str(refused.value)
    assert not os.path.exists(os.path.join(str(somewhere), "engines", "v9.9.9-linux-x64"))


def test_no_hash_beside_it_stops_the_whole_thing(somewhere, monkeypatch):
    """The check that is skipped when it is inconvenient is not a check."""
    monkeypatch.setattr(
        update, "_fetch",
        lambda url: b"" if url.endswith(".sha256") else _a_release(),
    )
    with pytest.raises(update.Cannot) as refused:
        update.fetch("v9.9.9", say=lambda _s: None)
    assert "нечем проверить" in str(refused.value)


def test_it_does_not_replace_the_folder_it_is_running_from(somewhere, monkeypatch):
    """Which sounds tidier and is where this goes wrong: a running program
    cannot overwrite its own executable on Windows at all, and a half-replaced
    folder anywhere is a worker that starts and then cannot render."""
    _served(monkeypatch, _a_release())
    landing = update.fetch("v9.9.9", say=lambda _s: None)
    assert landing.startswith(str(somewhere / "engines"))
    assert os.path.dirname(os.path.abspath(worker.__file__)) not in landing


def test_handing_over_to_something_that_is_not_there_is_refused(somewhere, tmp_path):
    empty = tmp_path / "nothing"
    empty.mkdir()
    with pytest.raises(update.Cannot):
        update.hand_over(str(empty))


# ── and not doing any of it uninvited ────────────────────────────────────────


async def test_a_bot_too_old_to_say_which_release_is_left_alone(monkeypatch):
    monkeypatch.setattr(update, "fetch", _never)
    assert await worker._offer_the_right_build("") is False


async def test_a_checkout_is_told_to_pull_rather_than_overwritten(monkeypatch):
    """Downloading a zip over somebody's working copy would replace their
    source with a build."""
    monkeypatch.setattr(update, "from_a_checkout", lambda: True)
    monkeypatch.setattr(update, "fetch", _never)
    assert await worker._offer_the_right_build("v9.9.9") is False


async def test_it_does_not_hand_over_twice_in_a_row(monkeypatch):
    """A fault that leaves the two still disagreeing must not become two
    processes replacing each other for ever, with a download in the loop."""
    monkeypatch.setattr(update, "from_a_checkout", lambda: False)
    monkeypatch.setattr(update, "already_handed_over", lambda: True)
    monkeypatch.setattr(update, "fetch", _never)
    assert await worker._offer_the_right_build("v9.9.9") is False


async def test_a_service_is_told_the_address_rather_than_asked(monkeypatch, caplog):
    """It cannot answer a question, and updating one behind its owner's back is
    not something to do."""
    from dossier import console

    monkeypatch.setattr(update, "from_a_checkout", lambda: False)
    monkeypatch.setattr(update, "already_handed_over", lambda: False)
    monkeypatch.setattr(console, "interactive", lambda: False)
    monkeypatch.setattr(update, "fetch", _never)
    monkeypatch.setattr(update, "slug", lambda: "linux-x64")

    with caplog.at_level("WARNING"):
        assert await worker._offer_the_right_build("v9.9.9") is False
    assert "dossier-v9.9.9-linux-x64.zip" in caplog.text


async def test_saying_no_fetches_nothing(monkeypatch, capsys):
    from dossier import console

    monkeypatch.setattr(update, "from_a_checkout", lambda: False)
    monkeypatch.setattr(update, "already_handed_over", lambda: False)
    monkeypatch.setattr(console, "interactive", lambda: True)
    monkeypatch.setattr(console, "_ask", lambda _p, _d="": "нет")
    monkeypatch.setattr(update, "fetch", _never)

    assert await worker._offer_the_right_build("v9.9.9") is False
    assert "стою и жду" in capsys.readouterr().out


async def test_saying_yes_fetches_and_hands_over(monkeypatch):
    from dossier import console

    handed = []
    monkeypatch.setattr(update, "from_a_checkout", lambda: False)
    monkeypatch.setattr(update, "already_handed_over", lambda: False)
    monkeypatch.setattr(console, "interactive", lambda: True)
    monkeypatch.setattr(console, "_ask", lambda _p, _d="": "да")
    monkeypatch.setattr(update, "fetch", lambda tag, **_kw: f"/tmp/{tag}")
    monkeypatch.setattr(update, "hand_over", handed.append)

    assert await worker._offer_the_right_build("v9.9.9") is True
    assert handed == ["/tmp/v9.9.9"]


async def test_a_download_that_fails_leaves_the_worker_standing(monkeypatch, capsys):
    """Rather than ending the program. The machine was doing something useful
    before this and can go on doing it once somebody's internet comes back."""
    from dossier import console

    def refuses(*_a, **_kw):
        raise update.Cannot("не удалось скачать")

    monkeypatch.setattr(update, "from_a_checkout", lambda: False)
    monkeypatch.setattr(update, "already_handed_over", lambda: False)
    monkeypatch.setattr(console, "interactive", lambda: True)
    monkeypatch.setattr(console, "_ask", lambda _p, _d="": "да")
    monkeypatch.setattr(update, "fetch", refuses)

    assert await worker._offer_the_right_build("v9.9.9") is False
    assert "не удалось скачать" in capsys.readouterr().out


def _never(*_args, **_kwargs):
    raise AssertionError("it downloaded something it should not have")
