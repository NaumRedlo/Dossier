"""Fetching the map a replay was played on.

Two sources for two reasons, and `maps` is what decides between them: the
`.osu` from osu! itself, which always answers and is what judging needs, and
the `.osz` from a mirror, which carries the song.

Nothing here is imported eagerly — the modules are small, but a render client
that has been handed the map's numbers already may never touch either.
"""

__all__ = ["beatmap_download", "beatmap_osu"]
