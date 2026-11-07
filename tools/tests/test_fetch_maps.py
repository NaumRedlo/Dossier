import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile


spec = importlib.util.spec_from_file_location("fetch_maps", Path(__file__).parents[1] / "fetch-maps.py")
fetch_maps = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fetch_maps)


class FetchMapsTests(unittest.TestCase):
    def test_default_and_audio_download_only_verified_mirror_archives(self):
        content = b"osu file format v14\n[Metadata]\nTitle:Test\n"
        digest = hashlib.md5(content).hexdigest()
        archive = io.BytesIO()
        with zipfile.ZipFile(archive, "w") as z:
            z.writestr("test.osu", content)
        for audio in [False, True]:
            with self.subTest(audio=audio), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                manifest = root / "corpus.tsv"
                manifest.write_text(f"{'a' * 32}\t{digest}\t123\t0\t0\t0\ttest.osr\n")
                urls = []

                def respond(request, **kwargs):
                    urls.append(request.full_url)
                    if request.full_url == fetch_maps.MIRRORS_MD5[0].format(hash=digest):
                        return io.BytesIO(json.dumps({"beatmapset_id": 456}).encode())
                    if request.full_url == fetch_maps.MIRRORS_OSZ[0].format(set_id=456):
                        return io.BytesIO(archive.getvalue())
                    self.fail(f"Unexpected network destination: {request.full_url}")

                args = ["fetch-maps", "--manifest", str(manifest), "--songs", str(root)]
                if audio:
                    (root / f"{digest}.osu").write_bytes(content)
                    args.append("--with-audio")
                with patch("sys.argv", args), patch.object(fetch_maps.urllib.request, "urlopen", side_effect=respond):
                    self.assertEqual(fetch_maps.main(), 0)
                self.assertEqual(len(urls), 2)
                self.assertEqual((root / f"{digest}.osz").read_bytes(), archive.getvalue())
                self.assertFalse((root / f"{digest}.osu").exists())

    def test_mismatched_revision_is_not_saved(self):
        archive = io.BytesIO()
        with zipfile.ZipFile(archive, "w") as z:
            z.writestr("wrong.osu", b"another revision")
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "map.osz"
            self.assertIsNotNone(fetch_maps.keep_set(archive.getvalue(), "a" * 32, target))
            self.assertFalse(target.exists())

    def test_bare_keeps_every_map_of_a_set_once_and_drops_the_archive(self):
        first = b"osu file format v14\n[Metadata]\nVersion:Easy\n"
        second = b"osu file format v14\n[Metadata]\nVersion:Hard\n"
        hashes = [hashlib.md5(body).hexdigest() for body in (first, second)]
        archive = io.BytesIO()
        with zipfile.ZipFile(archive, "w") as z:
            z.writestr("easy.osu", first)
            z.writestr("hard.osu", second)
            z.writestr("audio.mp3", b"music")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "corpus.tsv"
            manifest.write_text("".join(f"{str(n) * 32}\t{h}\t123\t0\t0\t0\t{n}.osr\n" for n, h in enumerate(hashes)))
            urls = []

            def respond(request, **kwargs):
                urls.append(request.full_url)
                if request.full_url in [fetch_maps.MIRRORS_MD5[0].format(hash=h) for h in hashes]:
                    return io.BytesIO(json.dumps({"beatmapset_id": 456}).encode())
                if request.full_url == fetch_maps.MIRRORS_OSZ[0].format(set_id=456):
                    return io.BytesIO(archive.getvalue())
                self.fail(f"Unexpected network destination: {request.full_url}")

            with patch("sys.argv", ["fetch-maps", "--manifest", str(manifest), "--songs", str(root), "--bare"]), \
                    patch.object(fetch_maps.urllib.request, "urlopen", side_effect=respond), \
                    patch.object(fetch_maps, "SPACING", 0):
                self.assertEqual(fetch_maps.main(), 0)
            self.assertEqual(urls.count(fetch_maps.MIRRORS_OSZ[0].format(set_id=456)), 1)
            self.assertEqual(sorted(p.name for p in root.iterdir() if p.suffix != ".tsv"), sorted(f"{h}.osu" for h in hashes))
            self.assertEqual((root / f"{hashes[1]}.osu").read_bytes(), second)

    def test_a_busy_mirror_is_asked_again_after_the_wait_it_names(self):
        busy = fetch_maps.urllib.error.HTTPError("https://osu.direct/x", 429, "busy", {"Retry-After": "2"}, None)
        waits = []
        with patch.object(fetch_maps.urllib.request, "urlopen", side_effect=[busy, io.BytesIO(b"{}")]), \
                patch.object(fetch_maps.time, "sleep", side_effect=waits.append), \
                patch.object(fetch_maps, "SPACING", 0):
            self.assertEqual(fetch_maps.get("https://osu.direct/api/v2/md5/" + "c" * 32), b"{}")
        self.assertEqual(waits, [2.0])

    def test_an_unreachable_mirror_is_not_taken_for_an_unknown_map(self):
        refused = fetch_maps.urllib.error.URLError("certificate verify failed")
        with tempfile.TemporaryDirectory() as directory, \
                patch.object(fetch_maps.urllib.request, "urlopen", side_effect=refused), \
                patch.object(fetch_maps, "SPACING", 0):
            fetch_maps.answers.clear()
            why, kept = fetch_maps.fetch_set("d" * 32, Path(directory))
        self.assertIn("certificate verify failed", why)
        self.assertNotIn("no mirror knows", why)
        self.assertEqual(kept, set())

    def test_dry_run_does_not_access_network(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "corpus.tsv"
            manifest.write_text(f"{'a' * 32}\t{'b' * 32}\t123\t0\t0\t0\ttest.osr\n")
            with patch("sys.argv", ["fetch-maps", "--manifest", str(manifest), "--songs", str(root), "--dry-run"]), patch.object(fetch_maps.urllib.request, "urlopen") as network:
                self.assertEqual(fetch_maps.main(), 0)
                network.assert_not_called()


if __name__ == "__main__":
    unittest.main()
