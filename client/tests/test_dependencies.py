"""What the render client is allowed to need.

A worker runs on somebody else's laptop, and what it imports is what they have
to install. Every package named here is a step in an instruction somebody has
to follow on a machine you cannot see, and the two that are named are already
two more than nothing.

This used to be a test about not importing the bot — the client lived inside
the bot's repository, and `services/__init__.py` re-exporting the card renderer
meant that importing *anything* under `services` built Pillow, fontTools,
SQLAlchemy and the pp calculator, for a program whose whole job is to run a
native binary and hand back an `.mp4`. The bot is a repository away now, so the
question has changed: not "does it drag the bot in" but "does it need anything
it has not declared".

Which is the better question, because it has an answer that can be checked
against a file rather than against a list somebody maintains.
"""

import ast
import os
import re
import subprocess
import sys

CLIENT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PACKAGE = os.path.join(CLIENT, "dossier")

# Anything a render has no business needing. Every one of these was genuinely
# being loaded by the worker at some point, none of them on purpose.
FORBIDDEN = ("aiogram", "sqlalchemy", "aiosqlite", "PIL", "fontTools",
             "rosu_pp_py", "cryptography", "numpy", "db")


def _declared() -> set[str]:
    """The dependencies `pyproject.toml` names, as import names."""
    body = open(os.path.join(CLIENT, "pyproject.toml"), encoding="utf-8").read()
    block = body.partition("dependencies = [")[2].partition("]")[0]
    return {re.split(r"[<>=!\[ ]", line.strip().strip('",'))[0].lower()
            for line in block.splitlines() if line.strip().startswith('"')}


def _third_party_imports() -> dict[str, str]:
    """Every non-stdlib package the source imports, and where it does it.

    Read off the syntax tree rather than by importing anything, so a module
    that is only reached on Windows is checked on a Mac like any other.
    """
    found = {}
    for here, _, files in os.walk(PACKAGE):
        for name in files:
            if not name.endswith(".py"):
                continue
            path = os.path.join(here, name)
            tree = ast.parse(open(path, encoding="utf-8").read())
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    roots = [alias.name.split(".")[0] for alias in node.names]
                elif isinstance(node, ast.ImportFrom):
                    # `from . import x` has no module to speak of.
                    roots = [node.module.split(".")[0]] if node.module and not node.level else []
                else:
                    continue
                for root in roots:
                    if root in sys.stdlib_module_names or root == "dossier":
                        continue
                    found.setdefault(root, os.path.relpath(path, CLIENT))
    return found


def test_nothing_is_imported_that_is_not_declared():
    """A package added to the source and not to `pyproject.toml` is a worker
    that installs cleanly and then dies on the first render."""
    undeclared = {name: where for name, where in _third_party_imports().items()
                  if name.lower() not in _declared()}
    assert not undeclared, (
        "imported but not declared in pyproject.toml: "
        + ", ".join(f"{name} ({where})" for name, where in sorted(undeclared.items()))
    )


def test_nothing_is_declared_that_is_not_imported():
    """The other direction, which is how a dependency list grows things nobody
    needs — each one an install somebody waits for."""
    imported = {name.lower() for name in _third_party_imports()}
    idle = _declared() - imported
    assert not idle, f"declared and never imported: {', '.join(sorted(idle))}"


def test_the_client_is_only_two_dependencies():
    """The number itself, because it is the thing that makes this installable
    on a machine you are not standing in front of. aiohttp to talk to the bot,
    requests to fetch maps, and nothing else."""
    assert _declared() == {"aiohttp", "requests"}, _declared()


def _loaded_by_the_client() -> set[str]:
    """What a fresh interpreter loads to run the client.

    Run out of process on purpose: asking `sys.modules` from inside pytest
    answers about pytest, which has already imported half the tree.
    """
    probe = (
        "import sys, importlib.util\n"
        "before = set(sys.modules)\n"
        f"spec = importlib.util.spec_from_file_location('worker', {os.path.join(PACKAGE, 'worker.py')!r})\n"
        "module = importlib.util.module_from_spec(spec)\n"
        "spec.loader.exec_module(module)\n"
        "print(' '.join(sorted({n.split('.')[0] for n in set(sys.modules) - before})))\n"
    )
    done = subprocess.run(
        [sys.executable, "-c", probe], cwd=CLIENT,
        capture_output=True, text=True, check=False,
        env={**os.environ, "PYTHONPATH": CLIENT},
    )
    assert done.returncode == 0, f"the client would not import:\n{done.stderr}"
    return set(done.stdout.split())


def test_a_run_loads_none_of_the_heavy_things():
    loaded = _loaded_by_the_client()
    unwanted = sorted(loaded & set(FORBIDDEN))
    assert not unwanted, (
        f"the client imports {', '.join(unwanted)} — nothing about a render "
        f"needs any of it, and a worker on somebody's laptop installs what it "
        f"imports"
    )


def test_the_probe_is_actually_looking_at_something():
    """The guard above passes trivially if the client stopped importing at all
    — a broken probe reports an empty set and calls it clean."""
    loaded = _loaded_by_the_client()
    assert {"aiohttp", "dossier"} <= loaded, loaded


def test_nothing_is_deferred_past_the_probe():
    """An import inside a function is one this probe never reaches, and that
    is not a hypothetical: the client used to build the osu! API client inside
    `main`, so a guard that only loaded the module passed while a real run died
    on `No module named 'sqlalchemy'`.

    There are no deferred third-party imports left. This says so, rather than
    trusting that nobody adds one.
    """
    source = open(os.path.join(PACKAGE, "worker.py"), encoding="utf-8").read()
    # `[ \t]` and not `\s`: `\s` matches a newline, so `^\s+from` happily spans
    # a blank line and reports a top-level import as an indented one.
    deferred = set(re.findall(r"^[ \t]+(?:from|import) ([\w.]+)", source, re.M))
    outside = {name.split(".")[0] for name in deferred} - sys.stdlib_module_names
    assert not outside - {"dossier"}, (
        f"the client defers {', '.join(sorted(outside))}, which this probe "
        f"never loads — so the guard above is not looking at what a run would"
    )


def test_the_client_has_its_own_settings():
    """Ten values, read from the environment, declared here rather than
    borrowed. This is the seam that let the bridge leave the bot at all, and a
    test rather than a note because it would close again quietly."""
    from dossier import settings

    assert settings.__all__, "settings declares nothing"
    assert len(settings.__all__) == 10, settings.__all__
    body = open(os.path.join(PACKAGE, "settings.py"), encoding="utf-8").read()
    for name in settings.__all__:
        assert f"{name} = os.getenv" in body or f'"{name}"' in body, (
            f"{name} is exported without being read from the environment"
        )


def test_an_installed_package_does_not_look_for_the_engine_inside_a_venv():
    """`pip install dossier` puts this in site-packages, where "three
    directories up" is the virtual environment rather than a checkout — and
    `venv/target/release/dossier` is a path that has never existed anywhere.

    So an installed copy asks `PATH` instead, and the bot, which keeps its
    engine in a checkout of its own, says `DOSSIER_BIN` outright.
    """
    from dossier import settings

    beside = os.path.join(os.path.dirname(CLIENT), "target", "release")
    found = settings._find_engine()
    if os.path.isdir(beside):
        assert found.startswith(beside), found
    else:
        # Nothing built here: it may name PATH's copy or the unbuilt path, and
        # either way it must name something a person can act on.
        assert found.endswith("dossier") or found.endswith("dossier.exe"), found
