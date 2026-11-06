# Player skins for renders through the bot

In the `1984` bot, a member of a group shared with the bot sends an `.osk` file or a ZIP with one skin folder. Once checked, the skin is saved for the sender and chosen automatically for their next replays. `sts` → *Render skin* offers the shared skins, the person's own uploads and Dossier Default.

Every upload is stored by the hash of its contents. A new version under the same original name does not overwrite an archive a queued job already uses. Uploading a skin does not start a render by itself: send an `.osr` after it.

## The job contract

The reply to `POST /render/claim` keeps its earlier shape; with a skin chosen it carries:

```json
{"skin": {"name": "Player skin", "hash": "<SHA-256 of the archive, 64 hex characters>", "size": 123456}}
```

The worker downloads the binary archive from `GET /render/job/{id}/skin` with the usual Bearer authorisation and `X-Render-Worker`. Only the worker holding the job can fetch it. A missing skin is `skin: null` and means Dossier Default.

The worker checks the size (when given) and the hash of the downloaded archive; SHA-256 and the older MD5 are both accepted. The archive is unpacked into a temporary folder first, then checked to hold exactly one skin, possibly inside a nested folder. The finished cache is published only after the check passes; an older incomplete cache is downloaded again. Colours and sprites go to the renderer, sounds to the skin's sound loader.

## Leaderboard, loudness and the graphics card

Since 30 October 2026 a job carries `leaderboard`: up to fifty best results on the map with the name, classic score, accuracy in percent and mods without `CL`. The person picks the source in `sts` → *Leaderboard in the video*: the global top from the osu! API (`GET /beatmaps/{id}/scores`) or, since 31 October, the top of the chat they pinned, for osu!standard and only while they are still in that chat. When nobody in the chat has played the map, the global top is used. Without a map or an API answer the list is empty. The worker hands the list to the engine, and a leaderboard on the left of the video lets the player climb it as in the game. The player is left out of their own rivals. An older Dossier simply does not read the field.

Videos from the bot are normalised to −16 LUFS instead of −14, about 80% of the earlier loudness. Music and hitsound levels would not help here, because the mix is levelled to the target afterwards anyway.

The *Encode video on the graphics card* switch in the *Render* tile applies to the bot's jobs too. Before the first render Dossier tries to encode two frames with a hardware encoder of its system: VideoToolbox on macOS, NVENC, AMF and Quick Sync on Windows, NVENC and Quick Sync on Linux. The first one that works is remembered until restart; when none works, x264 encodes on the processor as before. The processor still draws the frames.

## Limits and updating

By default the bot takes archives up to 64 MiB (`RENDER_SKIN_MOST`, at most 256 MiB), with up to 512 MiB stored per player (`RENDER_SKIN_STORAGE_MOST`). Both the bot and the worker limit unpacking to 512 MiB and 8192 entries and refuse paths outside the folder and symbolic links. The download limits hold even without an HTTP Content-Length or a declared Telegram document size.

The feature needs the bot's code updated on the server and the bot restarted, then the apps with workers on updated. Local checks do not replace a real download through the configured Telegram Bot API and sending a finished video on the server. User data, tokens and the bot's live database were not touched while testing.
