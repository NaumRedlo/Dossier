import logging
import os
from logging.handlers import RotatingFileHandler

ROOT = "dossier"

def get_logger(name: str) -> logging.Logger:
    return logging.getLogger(ROOT).getChild(name)

def under(parent: logging.Logger) -> None:
    logging.getLogger(ROOT).parent = parent

def to_console(level: int = logging.INFO) -> logging.Logger:
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
    handler._dossier_console = True
    root.addHandler(handler)
    return root

FILE = os.path.expanduser("~/.dossier/worker.log")

MOST_BYTES = 2 * 1024 * 1024
KEEP = 3

def to_file(path: str = "", level: int = logging.INFO) -> str:
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
    try:
        with open(path or FILE, encoding="utf-8", errors="replace") as handle:
            return handle.read().splitlines()[-lines:]
    except OSError:
        return []

__all__ = ["ROOT", "FILE", "get_logger", "under", "to_console", "to_file", "tail"]
