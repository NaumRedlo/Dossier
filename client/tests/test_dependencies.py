import ast
import os
import re
import subprocess
import sys

CLIENT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PACKAGE = os.path.join(CLIENT, "dossier")

FORBIDDEN = ("aiogram", "sqlalchemy", "aiosqlite", "PIL", "fontTools",
             "rosu_pp_py", "cryptography", "numpy", "db")

def _declared() -> set[str]:
    body = open(os.path.join(CLIENT, "pyproject.toml"), encoding="utf-8").read()
    block = body.partition("dependencies = [")[2].partition("]")[0]
    return {re.split(r"[<>=!\[ ]", line.strip().strip('",'))[0].lower()
            for line in block.splitlines() if line.strip().startswith('"')}

def _third_party_imports() -> dict[str, str]:
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

                    roots = [node.module.split(".")[0]] if node.module and not node.level else []
                else:
                    continue
                for root in roots:
                    if root in sys.stdlib_module_names or root == "dossier":
                        continue
                    found.setdefault(root, os.path.relpath(path, CLIENT))
    return found

def test_nothing_is_imported_that_is_not_declared():
    undeclared = {name: where for name, where in _third_party_imports().items()
                  if name.lower() not in _declared()}
    assert not undeclared, (
        "imported but not declared in pyproject.toml: "
        + ", ".join(f"{name} ({where})" for name, where in sorted(undeclared.items()))
    )

def test_nothing_is_declared_that_is_not_imported():
    imported = {name.lower() for name in _third_party_imports()}
    idle = _declared() - imported
    assert not idle, f"declared and never imported: {', '.join(sorted(idle))}"

def test_the_client_is_three_dependencies_and_they_are_named():
    assert _declared() == {"aiohttp", "requests", "certifi"}, _declared()

def _loaded_by_the_client() -> set[str]:
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
    loaded = _loaded_by_the_client()
    assert {"aiohttp", "dossier"} <= loaded, loaded

def test_nothing_is_deferred_past_the_probe():
    source = open(os.path.join(PACKAGE, "worker.py"), encoding="utf-8").read()

    deferred = set(re.findall(r"^[ \t]+(?:from|import) ([\w.]+)", source, re.M))
    outside = {name.split(".")[0] for name in deferred} - sys.stdlib_module_names

    allowed = {"dossier", "certifi"}
    assert not outside - allowed, (
        f"the client defers {', '.join(sorted(outside - allowed))}, which this "
        f"probe never loads — so the guard above is not looking at what a run "
        f"would"
    )

def test_the_client_has_its_own_settings():
    from dossier import settings

    assert set(settings.__all__) == {
        "DOSSIER_BIN",
        "DOSSIER_FONT",
        "DOSSIER_FFMPEG",
        "DOSSIER_CRF",
        "DOSSIER_PRESET",
        "DOSSIER_ENCODER_THREADS",
        "DOSSIER_SKIN",
        "DOSSIER_GAME_SOUNDS",
        "BEATMAP_STORE_DIR",
        "SKIN_STORE_DIR",
        "MAX_SKIN_MB",
    }, settings.__all__

    body = open(os.path.join(PACKAGE, "settings.py"), encoding="utf-8").read()
    for name in settings.__all__:
        assert f"{name} = os.getenv" in body or f'"{name}"' in body, (
            f"{name} is exported without being read from the environment"
        )

def test_an_installed_package_does_not_look_for_the_engine_inside_a_venv():
    from dossier import settings

    beside = os.path.join(os.path.dirname(CLIENT), "target", "release")
    found = settings._find_engine()
    if os.path.isdir(beside):
        assert found.startswith(beside), found
    else:

        assert found.endswith("dossier") or found.endswith("dossier.exe"), found
