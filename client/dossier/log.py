"""Where this package's log lines go — which is not this package's decision.

The bridge used to call the bot's `utils.logger`, which sets up files under
`logs/`, names the tree "Bot" and configures itself at import. That is right
for an application and wrong for a library: a package that attaches handlers
the moment it is imported takes over the logging of whatever imported it.

So this names loggers and does nothing else. Every module here asks for its
own child of `dossier`, and the two things that run this package say where
those lines should end up:

    the bot     `under(logging.getLogger("Bot"))` — its own handlers, its own files
    the client  `to_console()` — one line per event on the terminal

Left unsaid, Python's own last resort applies: warnings and worse to stderr,
nothing else. Which is a reasonable thing for an unconfigured library to do.
"""

import logging
import os
from logging.handlers import RotatingFileHandler

# The one name the whole package hangs under, so a host can reach all of it
# with a single `getLogger`.
ROOT = "dossier"


def get_logger(name: str) -> logging.Logger:
    """The logger for one module — `maps`, `skins`, `osu.beatmap`."""
    return logging.getLogger(ROOT).getChild(name)


def under(parent: logging.Logger) -> None:
    """Hang these logs under a tree the host has already set up.

    For the bot, whose handlers, formats and files were decided long before
    this package existed and should go on deciding.
    """
    logging.getLogger(ROOT).parent = parent


def to_console(level: int = logging.INFO) -> logging.Logger:
    """One line per event on the terminal, for the render client.

    Idempotent: called twice it does not print everything twice, which is the
    usual way a small helper like this goes wrong.
    """
    root = logging.getLogger(ROOT)
    root.setLevel(level)
    for handler in root.handlers:
        if getattr(handler, "_dossier_console", False):
            return root
    handler = logging.StreamHandler()
    handler.setLevel(level)
    handler.setFormatter(logging.Formatter(
        fmt="%(asctime)s | %(levelname)-8s | %(name)s | %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    ))
    handler._dossier_console = True  # so a second call finds it
    root.addHandler(handler)
    return root


# Where a worker's log is kept, so that "пришли лог" has something to answer.
# Beside the settings rather than beside the program: a release is a folder
# somebody may move or replace, and a log that moves with it is a log that is
# gone exactly when it is wanted.
FILE = os.path.expanduser("~/.dossier/worker.log")

# Three files of two megabytes. Enough to hold the evening a problem happened
# in, and small enough to send.
MOST_BYTES = 2 * 1024 * 1024
KEEP = 3


def to_file(path: str = "", level: int = logging.INFO) -> str:
    """Keep a copy on disk. Returns where it is being kept.

    Always on, including for a service, because the moment somebody needs a log
    is never the moment they had thought to start keeping one.

    Idempotent, like `to_console`: called twice it does not write everything
    twice.
    """
    path = path or FILE
    root = logging.getLogger(ROOT)
    root.setLevel(min(root.level or level, level))
    for handler in root.handlers:
        if getattr(handler, "_dossier_file", "") == path:
            return path

    try:
        os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
        handler = RotatingFileHandler(
            path, maxBytes=MOST_BYTES, backupCount=KEEP, encoding="utf-8",
        )
    except OSError as exc:
        # A read-only home, a full disk. Worth saying once and not worth
        # refusing to render over.
        logging.getLogger(ROOT).warning("no log file at %s: %s", path, exc)
        return ""

    handler.setLevel(level)
    handler.setFormatter(logging.Formatter(
        fmt="%(asctime)s | %(levelname)-8s | %(name)s | %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    ))
    handler._dossier_file = path
    root.addHandler(handler)
    return path


def tail(lines: int = 40, path: str = "") -> list[str]:
    """The last few lines, for showing somebody without opening an editor.

    Read whole rather than seeked backwards: two megabytes is nothing to a
    computer that renders video, and seeking backwards through a text file for
    `n` newlines is four lines of code that go wrong on the boundary.
    """
    try:
        with open(path or FILE, encoding="utf-8", errors="replace") as handle:
            return handle.read().splitlines()[-lines:]
    except OSError:
        return []


__all__ = ["ROOT", "FILE", "get_logger", "under", "to_console", "to_file", "tail"]
