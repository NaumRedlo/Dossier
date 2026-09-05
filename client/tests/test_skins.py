import os
import zipfile

import pytest

from dossier import skins

@pytest.fixture(autouse=True)
def store(tmp_path, monkeypatch):
    monkeypatch.setattr(skins, "SKIN_STORE_DIR", str(tmp_path / "skins"))
    return tmp_path

def osk(tmp_path, entries: dict[str, bytes], name="pack.osk") -> str:
    path = tmp_path / name
    with zipfile.ZipFile(path, "w") as archive:
        for inner, body in entries.items():
            archive.writestr(inner, body)
    return str(path)

def kept(folder: str) -> list[str]:
    return sorted(name for name in os.listdir(folder) if not name.startswith("."))

def test_a_path_that_climbs_out_of_the_folder_lands_inside_it(tmp_path):
    archive = osk(tmp_path, {"../escaped.png": b"x", "hitcircle.png": b"y"})
    name = skins.import_osk(archive, "pack.osk")

    folder = skins.folder_of(name)
    assert kept(folder) == ["escaped.png", "hitcircle.png"]
    assert not os.path.exists(os.path.join(folder, "..", "escaped.png")), (
        "nothing was written beside the store"
    )

def test_an_archive_that_promises_more_than_we_hold_is_refused(tmp_path, monkeypatch):
    monkeypatch.setattr(skins, "MAX_UNPACKED_BYTES", 1024)
    archive = osk(tmp_path, {"hitcircle.png": b"0" * 4096})
    with pytest.raises(skins.SkinRejected, match="МБ"):
        skins.import_osk(archive, "pack.osk")

def test_a_declaration_is_not_taken_on_trust(tmp_path, monkeypatch):
    monkeypatch.setattr(skins, "MAX_UNPACKED_BYTES", 4096)
    archive = osk(tmp_path, {"hitcircle.png": b"0" * 2048, "cursor.png": b"0" * 2048})

    monkeypatch.setattr(skins, "MAX_UNPACKED_BYTES", 3000)
    with pytest.raises(skins.SkinRejected):
        skins.import_osk(archive, "pack.osk")

def test_something_that_is_not_an_archive_says_so(tmp_path):
    path = tmp_path / "not.osk"
    path.write_bytes(b"this is not a zip file")
    with pytest.raises(skins.SkinRejected, match="архив"):
        skins.import_osk(str(path), "not.osk")

def test_an_absurd_number_of_files_is_not_a_skin(tmp_path, monkeypatch):
    monkeypatch.setattr(skins, "MAX_FILES", 3)
    archive = osk(tmp_path, {f"hit{i}.png": b"x" for i in range(5)})
    with pytest.raises(skins.SkinRejected):
        skins.import_osk(archive, "pack.osk")

def test_an_archive_that_wraps_its_files_in_a_folder_still_works(tmp_path):
    archive = osk(tmp_path, {"my skin/hitcircle.png": b"x", "my skin/skin.ini": b"[General]"})
    name = skins.import_osk(archive, "pack.osk")
    files = kept(skins.folder_of(name))
    assert files == ["hitcircle.png", "skin.ini"]

def test_only_the_files_the_engine_reads_are_kept(tmp_path):
    archive = osk(tmp_path, {
        "hitcircle.png": b"x",
        "readme.txt": b"hello",
        "source.psd": b"big",
        "hitnormal.wav": b"w",
    })
    name = skins.import_osk(archive, "pack.osk")
    assert kept(skins.folder_of(name)) == ["hitcircle.png", "hitnormal.wav"]

def test_an_archive_with_nothing_we_read_is_refused(tmp_path):
    archive = osk(tmp_path, {"readme.txt": b"hello"})
    with pytest.raises(skins.SkinRejected):
        skins.import_osk(archive, "pack.osk")

def test_a_skin_is_named_after_its_file_and_can_be_found_again(tmp_path):
    archive = osk(tmp_path, {"hitcircle.png": b"x"})
    name = skins.import_osk(archive, "doki dt mix v3.osk")
    assert name == "doki dt mix v3"
    assert skins.available() == [name]
    assert skins.folder_of(name)

def test_a_name_that_is_not_in_the_store_resolves_to_nothing(tmp_path):
    assert skins.folder_of("../../etc") is None
    assert skins.folder_of("nothing here") is None

def test_sending_the_same_skin_again_replaces_it(tmp_path):
    first = osk(tmp_path, {"hitcircle.png": b"one"}, name="a.osk")
    second = osk(tmp_path, {"cursor.png": b"two"}, name="b.osk")
    skins.import_osk(first, "same.osk")
    skins.import_osk(second, "same.osk")
    assert skins.available() == ["same"]
    assert kept(skins.folder_of("same")) == ["cursor.png"]

def test_a_failed_import_leaves_the_skin_that_was_there(tmp_path):
    good = osk(tmp_path, {"hitcircle.png": b"one"}, name="a.osk")
    skins.import_osk(good, "same.osk")

    bad = osk(tmp_path, {"readme.txt": b"nothing we read"}, name="b.osk")
    with pytest.raises(skins.SkinRejected):
        skins.import_osk(bad, "same.osk")

    assert skins.available() == ["same"]
    assert kept(skins.folder_of("same")) == ["hitcircle.png"]

def test_a_skin_can_be_forgotten(tmp_path):
    skins.import_osk(osk(tmp_path, {"hitcircle.png": b"x"}), "gone.osk")
    assert skins.forget("gone") is True
    assert skins.available() == []
    assert skins.forget("gone") is False

def _sample(path: str) -> None:
    import shutil
    import subprocess

    if not shutil.which("ffmpeg"):
        pytest.skip("ffmpeg is not on PATH, and this fixture is made with it")

    subprocess.run(
        ["ffmpeg", "-nostdin", "-v", "error", "-y", "-f", "lavfi",
         "-i", "sine=frequency=440:duration=0.05", path],
        check=True, capture_output=True,
    )

def test_a_skins_hitsounds_are_converted_to_what_the_engine_reads(tmp_path):
    from dossier import skins

    _sample(str(tmp_path / "normal-hitnormal.mp3"))
    skins._to_wav(str(tmp_path))

    assert (tmp_path / "normal-hitnormal.wav").exists()
    assert (tmp_path / "normal-hitnormal.mp3").exists(), "the original is left alone"

def test_a_skins_own_wav_is_not_overwritten_by_its_ogg(tmp_path):
    from dossier import skins

    _sample(str(tmp_path / "soft-hitclap.mp3"))
    (tmp_path / "soft-hitclap.wav").write_bytes(b"the skin's own")
    skins._to_wav(str(tmp_path))

    assert (tmp_path / "soft-hitclap.wav").read_bytes() == b"the skin's own"

def test_a_sample_that_will_not_convert_leaves_no_wreckage(tmp_path):
    from dossier import skins

    (tmp_path / "nightcore-kick.ogg").write_bytes(b"")
    skins._to_wav(str(tmp_path))

    assert not (tmp_path / "nightcore-kick.wav").exists()

def test_pictures_are_left_alone(tmp_path):
    from dossier import skins

    (tmp_path / "hitcircle.png").write_bytes(b"not really a png")
    skins._to_wav(str(tmp_path))
    assert sorted(p.name for p in tmp_path.iterdir()) == ["hitcircle.png"]

def test_a_skin_stored_before_the_conversion_existed_is_swept(tmp_path, monkeypatch):
    from dossier import skins

    monkeypatch.setattr(skins, "store_dir", lambda: str(tmp_path))
    old = tmp_path / "vaxei"
    old.mkdir()
    _sample(str(old / "normal-hitnormal.mp3"))

    assert skins.convert_stored() == 1
    assert (old / "normal-hitnormal.wav").exists()

def test_the_sweep_does_nothing_on_the_second_run(tmp_path, monkeypatch):
    from dossier import skins

    monkeypatch.setattr(skins, "store_dir", lambda: str(tmp_path))
    folder = tmp_path / "azr8"
    folder.mkdir()
    _sample(str(folder / "soft-hitclap.mp3"))

    assert skins.convert_stored() == 1
    assert skins.convert_stored() == 0

def test_a_skin_of_pictures_alone_is_left_alone(tmp_path, monkeypatch):
    from dossier import skins

    monkeypatch.setattr(skins, "store_dir", lambda: str(tmp_path))
    folder = tmp_path / "pictures"
    folder.mkdir()
    (folder / "hitcircle.png").write_bytes(b"not really a png")

    assert skins.convert_stored() == 0

def test_a_missing_store_is_not_an_error(tmp_path, monkeypatch):
    from dossier import skins

    monkeypatch.setattr(skins, "store_dir", lambda: str(tmp_path / "nothing"))
    assert skins.convert_stored() == 0

def test_an_empty_sample_is_skipped_rather_than_asked_about(tmp_path):
    from dossier import skins

    (tmp_path / "nightcore-kick.ogg").write_bytes(b"")
    skins._to_wav(str(tmp_path))
    assert not (tmp_path / "nightcore-kick.wav").exists()

def test_a_skin_with_an_empty_sample_does_not_look_unfinished_for_ever(
    tmp_path, monkeypatch
):
    from dossier import skins

    monkeypatch.setattr(skins, "store_dir", lambda: str(tmp_path))
    folder = tmp_path / "azerino"
    folder.mkdir()
    _sample(str(folder / "normal-hitnormal.mp3"))
    (folder / "nightcore-kick.ogg").write_bytes(b"")

    assert skins.convert_stored() == 1, "the real sample is converted"
    assert skins.convert_stored() == 0, "and the empty one does not keep it coming back"
    assert (folder / "normal-hitnormal.wav").exists()
    assert (folder / "nightcore-kick.ogg").exists(), "the empty file is left where it was"

def _wav(data: bytes = b"\x00\x00", *, tag: int = 1, bits: int = 16, channels: int = 2) -> bytes:
    fmt = (
        tag.to_bytes(2, "little")
        + channels.to_bytes(2, "little")
        + (44100).to_bytes(4, "little")
        + (176400).to_bytes(4, "little")
        + (4).to_bytes(2, "little")
        + bits.to_bytes(2, "little")
    )
    body = b"WAVE" + b"fmt " + len(fmt).to_bytes(4, "little") + fmt
    body += b"data" + len(data).to_bytes(4, "little") + data
    return b"RIFF" + len(body).to_bytes(4, "little") + body

def test_what_the_engine_can_read_and_what_it_cannot(tmp_path):
    cases = {
        "plain.wav": (_wav(), True),

        "blank.wav": (_wav(b""), True),
        "eight-bit.wav": (_wav(b"\x80", bits=8, channels=1), True),

        "gsm.wav": (_wav(tag=49, bits=0), False),
        "float.wav": (_wav(tag=3, bits=32), False),
        "five-channel.wav": (_wav(channels=5), False),

        "renamed.wav": (b"OggS" + b"\x00" * 60, False),
    }
    for leaf, (body, readable) in cases.items():
        path = tmp_path / leaf
        path.write_bytes(body)
        assert skins._readable_wav(str(path)) is readable, leaf

def test_an_unreadable_wav_is_converted_even_though_a_wav_is_there(tmp_path):
    folder = tmp_path / "skin"
    folder.mkdir()
    (folder / "drum-slidertick.wav").write_bytes(_wav(tag=49, bits=0))
    (folder / "drum-slidertick.ogg").write_bytes(b"OggS" + b"\x00" * 60)

    work = skins._unconverted(str(folder))
    assert [(os.path.basename(a), os.path.basename(b)) for a, b in work] == [

        ("drum-slidertick.wav", "drum-slidertick.wav")
    ]

def test_a_blank_wav_is_left_alone_however_much_is_beside_it(tmp_path):
    folder = tmp_path / "skin"
    folder.mkdir()
    (folder / "soft-hitwhistle.wav").write_bytes(_wav(b""))
    (folder / "soft-hitwhistle.ogg").write_bytes(b"OggS" + b"\x00" * 60)
    assert skins._unconverted(str(folder)) == []

def test_a_sound_with_no_wav_at_all_is_still_the_ordinary_case(tmp_path):
    folder = tmp_path / "skin"
    folder.mkdir()
    (folder / "normal-hitnormal.ogg").write_bytes(b"OggS" + b"\x00" * 60)
    work = skins._unconverted(str(folder))
    assert [os.path.basename(b) for _, b in work] == ["normal-hitnormal.wav"]

def test_an_empty_file_is_never_work(tmp_path):
    folder = tmp_path / "skin"
    folder.mkdir()
    (folder / "nightcore-kick.ogg").write_bytes(b"")
    (folder / "drum-sliderwhistle.wav").write_bytes(b"")
    assert skins._unconverted(str(folder)) == []

def test_a_real_conversion_produces_something_the_engine_reads(tmp_path):
    import shutil as _shutil
    import subprocess as _subprocess

    from dossier.settings import DOSSIER_FFMPEG

    if not _shutil.which(DOSSIER_FFMPEG):
        pytest.skip("no ffmpeg here")

    folder = tmp_path / "skin"
    folder.mkdir()

    made = _subprocess.run(
        [DOSSIER_FFMPEG, "-nostdin", "-v", "error", "-y", "-f", "lavfi",
         "-i", "sine=frequency=440:duration=0.2", "-ac", "1",
         "-c:a", "pcm_s24le", str(folder / "drum-hitclap.wav")],
        capture_output=True,
    )
    assert made.returncode == 0, made.stderr[:200]
    assert not skins._readable_wav(str(folder / "drum-hitclap.wav"))

    assert skins.convert_folder(str(folder)) == 1
    assert skins._readable_wav(str(folder / "drum-hitclap.wav"))

    assert sorted(p.name for p in folder.iterdir()) == ["drum-hitclap.wav"]

def test_an_unpacked_skin_records_what_unpacked_it(tmp_path):
    archive = osk(tmp_path, {"hitcircle.png": b"x" * 40}, "stamped.osk")
    name = skins.import_osk(archive, "stamped.osk")

    stamp = skins.stamp_of(skins.folder_of(name))
    assert stamp["extract_version"] == skins.EXTRACT_VERSION
    assert stamp["source"] == "stamped.osk"
    assert not skins.is_stale(name)
    assert skins.stale() == []

def test_a_folder_from_an_older_unpacking_asks_to_be_sent_again(tmp_path):
    archive = osk(tmp_path, {"hitcircle.png": b"x" * 40}, "old.osk")
    name = skins.import_osk(archive, "old.osk")

    os.remove(os.path.join(skins.folder_of(name), skins.STAMP))
    assert skins.is_stale(name)
    assert skins.stale() == [name]

    skins.import_osk(archive, "old.osk")
    assert not skins.is_stale(name)

def test_a_skin_nobody_has_is_absent_rather_than_stale():
    assert not skins.is_stale("never-sent")

def test_a_file_that_was_never_audio_is_not_offered_to_ffmpeg(tmp_path):
    for name, body in (
        ("empty.ogg", b""),
        ("truncated.ogg", b"OggS\x00\x02"),
        ("renamed.ogg", b"this is a text file" * 8),
        ("short.mp3", b"\xff\xfb"),
    ):
        (tmp_path / name).write_bytes(body)
        assert not skins._has_bytes(str(tmp_path / name)), name

def test_real_audio_still_goes_through(tmp_path):
    for name, body in (
        ("song.ogg", b"OggS" + b"\x00" * 400),
        ("tagged.mp3", b"ID3" + b"\x00" * 400),
        ("bare.mp3", b"\xff\xfb" + b"\x00" * 400),
        ("sample.wav", b"RIFF" + b"\x00" * 400),
    ):
        (tmp_path / name).write_bytes(body)
        assert skins._has_bytes(str(tmp_path / name)), name

def test_a_silent_placeholder_is_left_alone_rather_than_converted(tmp_path):
    (tmp_path / "drum-hitnormal.ogg").write_bytes(b"")
    assert skins._unconverted(str(tmp_path)) == []

def test_one_line_for_a_skin_rather_than_one_per_file(tmp_path, monkeypatch, caplog):
    import logging
    import subprocess

    for at in range(13):

        (tmp_path / f"sound{at}.ogg").write_bytes(b"OggS" + b"\x00" * 400)

    def refuse(*_a, **_kw):
        return subprocess.CompletedProcess([], 1, b"", b"Invalid data found\n")

    monkeypatch.setattr(skins.subprocess, "run", refuse)
    with caplog.at_level(logging.WARNING):
        skins._to_wav(str(tmp_path))

    complaints = [r for r in caplog.records if "ffmpeg" in r.getMessage()]
    assert len(complaints) == 1, f"{len(complaints)} lines for one skin"
    assert "13 sample(s)" in complaints[0].getMessage()
