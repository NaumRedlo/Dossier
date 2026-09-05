from importlib import import_module
from typing import TYPE_CHECKING

_EXPORTS = {
    "MapUnavailable": ("dossier.maps", "MapUnavailable"),
    "describe": ("dossier.maps", "describe"),
    "ensure_known": ("dossier.maps", "ensure_known"),
    "ensure_map": ("dossier.maps", "ensure_map"),
    "songs_dir": ("dossier.maps", "songs_dir"),
    "DossierError": ("dossier.runner", "DossierError"),
    "Moment": ("dossier.runner", "Moment"),
    "Selection": ("dossier.runner", "Selection"),
    "exhibit": ("dossier.runner", "exhibit"),
    "inspect": ("dossier.runner", "inspect"),
    "is_available": ("dossier.runner", "is_available"),
    "judge": ("dossier.runner", "judge"),
    "moments": ("dossier.runner", "moments"),
    "video": ("dossier.runner", "video"),
}

if TYPE_CHECKING:
    from dossier.maps import MapUnavailable, describe, ensure_known, ensure_map, songs_dir
    from dossier.runner import (
        DossierError,
        Moment,
        Selection,
        exhibit,
        inspect,
        is_available,
        judge,
        moments,
        video,
    )

def __getattr__(name: str):
    found = _EXPORTS.get(name)
    if found is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    where, called = found
    value = getattr(import_module(where), called)
    globals()[name] = value
    return value

def __dir__():
    return sorted(set(globals()) | set(_EXPORTS))

__all__ = list(_EXPORTS)
