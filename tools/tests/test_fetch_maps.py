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
