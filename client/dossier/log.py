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


__all__ = ["ROOT", "get_logger", "under", "to_console"]
