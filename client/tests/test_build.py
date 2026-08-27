"""The build stamp: reading it, what it covers, and what it decides.

Two machines running the engine have to be running the *same* engine, because
a render is a comparison — the same replay, judged the same way — and two
builds that differ have no business splitting the work between them. The stamp
is how each says which one it is, and `build.agree` is what refuses.

It is folded from the crates and the lock file and nothing else, and that is a
thing this test walks real history to check: the farm once stopped for an hour
because the stamp was the repository's commit, and a worker was refused over a
markdown file.
"""

import re
import subprocess
from pathlib import Path

import pytest

from dossier import build as engine_build

# `client/tests` up two: the checkout, which is what gets stamped.
REPO = Path(__file__).resolve().parents[2]


def _saying(version):
    """A stand-in for the local engine that answers one fixed line."""

    async def local(*_args, **_kwargs):
        return version

    return local


class TestReadingTheStamp:
    def test_the_id_is_taken_out_of_the_line_the_engine_prints(self):
        assert engine_build.build_of("dossier 0.1.0 (15abdf1)") == "15abdf1"

    def test_a_build_from_an_edited_tree_keeps_its_mark(self):
        assert engine_build.build_of("dossier 0.1.0 (15abdf1+)") == "15abdf1+"
        allowed, _ = engine_build.agree("d 0.1.0 (15abdf1+)", "d 0.1.0 (15abdf1)")
        assert not allowed

    def test_two_edited_trees_are_cannot_tell_rather_than_a_refusal(self):
        # Same reasoning as two `unknown`s below: neither can say what it is,
        # so this is ignorance rather than disagreement.
        allowed, why = engine_build.agree("d 0.1.0 (15abdf1+)", "d 0.1.0 (15abdf1+)")
        assert allowed and "edited tree" in why

    @pytest.mark.parametrize("line", [None, "", "dossier 0.1.0", "dossier ("])
    def test_anything_unreadable_is_unknown_rather_than_a_guess(self, line):
        assert engine_build.build_of(line) == engine_build.UNKNOWN
class TestWhatTheStampCovers:
    """The farm once stopped because the stamp was the repository's commit.

    `drejk-starsij.local` was refused with "the bot renders with 8aae009 and
    this worker with 6054b39", and the whole difference between those two
    commits was one markdown file — two identical programs, and the work went
    back to the bot. The inputs are read out of `build.rs` rather than repeated
    here, so this test cannot drift from what actually gets stamped.
    """

    @staticmethod
    def _inputs():
        source = (REPO / "crates/dossier-cli/build.rs").read_text()
        declared = re.search(r"const INPUTS: \[&str; \d+\] = \[(.*?)\];", source, re.S)
        assert declared, "build.rs no longer declares INPUTS"
        return re.findall(r'"([^"]+)"', declared.group(1))

    @staticmethod
    def _git(*args):
        done = subprocess.run(
            ("git", *args), cwd=REPO, capture_output=True, text=True, check=False
        )
        return done.stdout.strip() if done.returncode == 0 else None

    def test_the_documents_are_not_among_them(self):
        assert "docs" not in self._inputs()
        assert "crates" in self._inputs()

    def test_a_commit_that_only_touched_documents_does_not_move_the_stamp(self):
        """Walked over real history, because that is where the bug came from.

        A synthetic pair of commits would only prove the rule this test already
        knows. The repository's own documentation commits are the thing that
        stopped the farm, so they are what gets checked.
        """
        history = self._git("log", "--format=%H", "-40")
        if not history:
            pytest.skip("no git history to read")

        checked = 0
        for commit in history.split():
            parent = self._git("rev-parse", f"{commit}^")
            if parent is None:
                continue
            touched = self._git("diff", "--name-only", parent, commit) or ""
            if not touched or not all(f.endswith(".md") for f in touched.split()):
                continue
            before, after = (
                [self._git("rev-parse", f"{rev}:dossier/{i}") for i in self._inputs()]
                for rev in (parent, commit)
            )
            assert before == after, f"{commit[:7]} moved the stamp with only documents"
            checked += 1

        if not checked:
            pytest.skip("no documents-only commit in the last 40")
class TestDeciding:
    def test_two_of_the_same_build_may_work_together(self):
        allowed, why = engine_build.agree("d 0.1.0 (abc1234)", "d 0.1.0 (abc1234)")
        assert allowed and "abc1234" in why

    def test_two_different_builds_may_not(self):
        allowed, why = engine_build.agree("d 0.1.0 (abc1234)", "d 0.1.0 (def5678)")
        assert not allowed
        assert "abc1234" in why and "def5678" in why

    def test_an_edited_tree_is_told_to_commit_rather_than_to_pull(self):
        # The refusal that started this: same source on both sides, the worker
        # with edits on top. It was told `git pull`, which does nothing about
        # uncommitted changes, so the operator pulled and rebuilt in a loop.
        allowed, why = engine_build.agree("d 0.1.0 (023f7e7)", "d 0.1.0 (023f7e7+)")
        assert not allowed
        assert "023f7e7" in why and "this worker" in why
        assert "git pull" not in why
        # Rebuilding comes first because the mark outlives the edits: the stamp
        # is fixed when the binary is linked, so a tree tidied up but not built
        # again still says `+` with nothing left to stash.
        assert why.index("rebuild") < why.index("stash")

    def test_it_says_which_side_has_the_edits(self):
        _, why = engine_build.agree("d 0.1.0 (023f7e7+)", "d 0.1.0 (023f7e7)")
        assert "the bot" in why and "this worker" not in why

    def test_two_different_builds_are_told_to_pull(self):
        allowed, why = engine_build.agree("d 0.1.0 (abc1234)", "d 0.1.0 (def5678)")
        assert not allowed
        assert "git pull" in why and "stash" not in why

    def test_a_build_that_cannot_say_what_it_is_is_let_through(self):
        allowed, why = engine_build.agree(None, "d 0.1.0 (abc1234)")
        assert allowed and "cannot say" in why
        allowed, _ = engine_build.agree("d 0.1.0 (abc1234)", None)
        assert allowed
