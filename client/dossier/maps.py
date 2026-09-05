import os

from typing import Optional

from dossier.settings import BEATMAP_STORE_DIR
from dossier.log import get_logger
from dossier.osu.beatmap_download import download_beatmap
from dossier.osu import beatmap_osu
from dossier.osu.beatmap_osu import download_osu

logger = get_logger("maps")

class MapUnavailable(RuntimeError):
    pass

def songs_dir() -> str:
    return BEATMAP_STORE_DIR

async def ensure_known(beatmap: dict, checksum: str) -> dict:
    beatmapset_id = beatmap.get("beatmapset_id")
    if beatmapset_id and await download_beatmap(int(beatmapset_id)):
        _drop_silent_copy(checksum)
        return beatmap

    if await download_osu(beatmap.get("id"), checksum):
        logger.warning(
            "beatmap %s came from osu! rather than a mirror — judging works, "
            "the render will be silent",
            beatmap.get("id"),
        )

        beatmap["_no_audio"] = True
        return beatmap

    raise MapUnavailable(
        f"карту {checksum} не удалось взять ни с зеркала, ни у osu! — "
        "возможно, она удалена или изменена после реплея"
    )

async def ensure_map(osu_api_client, checksum: str) -> dict:
    if not checksum:
        raise MapUnavailable("реплей не назвал карту (пустой хэш)")

    try:
        beatmap = await osu_api_client.lookup_beatmap_by_checksum(checksum)
    except Exception as exc:
        logger.warning("checksum lookup failed for %s: %s", checksum, exc)
        raise MapUnavailable(f"osu! API не ответил на запрос карты: {exc}") from exc

    if not beatmap:

        raise MapUnavailable(
            f"карта {checksum} не найдена в osu! — вероятно, она не залита или изменена локально"
        )

    return await ensure_known(beatmap, checksum)

def _drop_silent_copy(checksum: str) -> None:
    bare = beatmap_osu.path_for(checksum)
    if not os.path.isfile(bare):
        return
    try:
        os.remove(bare)
        logger.info("dropped the silent copy of %s — the archive supersedes it", checksum)
    except OSError as exc:
        logger.warning("could not drop the silent copy of %s: %s", checksum, exc)

def describe(beatmap: Optional[dict]) -> str:
    if not beatmap:
        return "неизвестная карта"
    beatmapset = beatmap.get("beatmapset") or {}
    artist = beatmapset.get("artist") or ""
    title = beatmapset.get("title") or ""
    version = beatmap.get("version") or ""
    if artist and title:
        return f"{artist} — {title} [{version}]".strip()
    return title or version or f"карта {beatmap.get('id', '?')}"
