import os
import zipfile

import dossier.skins as skins

def _archive(path, files):
    with zipfile.ZipFile(path, "w") as z:
        for name, body in files:
            z.writestr(name, body)
    return path

def test_the_root_wins_over_a_copy_in_a_folder(tmp_path, monkeypatch):
    archive = _archive(tmp_path / "s.osk", [

        ("num/default-0.png", b"the folder's"),
        ("hitsound/normal-hitnormal.wav", b"the folder's"),
        ("default-0.png", b"the root's"),
        ("normal-hitnormal.wav", b"the root's"),
    ])
    monkeypatch.setattr(skins, "store_dir", lambda: str(tmp_path / "store"))
    name = skins.import_osk(str(archive), "s.osk")
    folder = os.path.join(str(tmp_path / "store"), name)

    assert (open(os.path.join(folder, "default-0.png"), "rb").read()) == b"the root's"
    assert (open(os.path.join(folder, "normal-hitnormal.wav"), "rb").read()) == b"the root's"

def test_a_file_only_a_folder_has_is_still_kept(tmp_path, monkeypatch):

    archive = _archive(tmp_path / "s.osk", [
        ("num/berlin-0.png", b"the only one"),
        ("default-0.png", b"unrelated"),
    ])
    monkeypatch.setattr(skins, "store_dir", lambda: str(tmp_path / "store"))
    name = skins.import_osk(str(archive), "s.osk")
    folder = os.path.join(str(tmp_path / "store"), name)

    assert (open(os.path.join(folder, "berlin-0.png"), "rb").read()) == b"the only one"

def test_an_archive_that_wraps_itself_in_one_folder_still_unpacks(tmp_path, monkeypatch):

    archive = _archive(tmp_path / "s.osk", [
        ("my skin/default-0.png", b"digits"),
        ("my skin/normal-hitnormal.wav", b"sound"),
    ])
    monkeypatch.setattr(skins, "store_dir", lambda: str(tmp_path / "store"))
    name = skins.import_osk(str(archive), "s.osk")
    folder = os.path.join(str(tmp_path / "store"), name)

    assert os.path.isfile(os.path.join(folder, "default-0.png"))
    assert os.path.isfile(os.path.join(folder, "normal-hitnormal.wav"))
