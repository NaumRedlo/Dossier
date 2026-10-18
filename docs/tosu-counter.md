# Plates through tosu

Witness Overlay draws its plates in windows of its own over the game. On
17 October 2026 a tester showed that this cannot work in one mode: with
osu! stable in full screen without compatibility mode, NVIDIA's OpenGL driver
sends the game straight to the display and no window over it is seen (see
`witness-hud.md`). Dossier does not load anything into the game. tosu does:
it is a separate, open program that many players already run, and its in-game
overlay shows ordinary web pages ("counters") inside the game in every mode.
So the plates can be a tosu counter, which the player adds to tosu's in-game
overlay themselves.

What follows was read from tosu's source at commit `a4da331` (7 October 2026),
licensed LGPL-3.0. Dossier takes no code from it and only talks to it over the
network, so the licence asks nothing of Dossier.

## What tosu is

One program (`tosu.exe`) reads the memory of osu! stable or lazer and serves
what it reads on `http://127.0.0.1:24050` (`SERVER_IP`, `SERVER_PORT` in
`tosu.env`). Its dashboard lists counters and their settings.

- `/websocket/v2` streams the whole state about every 150 ms (`POLL_RATE`).
- `/websocket/v2/precise` streams key presses and the hit errors every 10 ms
  (`PRECISE_DATA_POLL_RATE`).
- `/json/v2` answers the same state once.
- `/files/beatmap/background`, `/files/beatmap/file` and the like give the
  current map's files.

## What the plates need, and where it is in `/websocket/v2`

| Plate needs | Field |
|---|---|
| Which screen the game shows | `state` (`number`, `name`) |
| Whether the game window is in front, paused | `game.focused`, `game.paused` |
| PP so far and PP without misses | `play.pp.current`, `play.pp.fc` |
| Unstable rate | `play.unstableRate`; raw errors in `play.hitErrorArray` |
| The game's own hit error meter: kind and size | `settings.scoreMeter.type`, `settings.scoreMeter.size` |
| A break | `beatmap.isBreak` |
| The map, for the pool and the chat places | `beatmap.checksum`, `beatmap.id`, `beatmap.set` |
| A pass or a fail | `play.failed`, `resultsScreen` |
| The player | `profile`, `play.playerName` |
| Ruleset and client | `play.mode`, `client` (stable or lazer) |

What tosu does not know is Dossier's: the pool the map is in, its stars as
Dossier counts them, the places in the chat, the day's summary, the player's
record on the map and the outcome of a kept replay. `dossier_hud::context::Packet`
already carries the first four for `tools/overlay-host` (`Card` and `Day`);
Dossier does not fill the day yet, and the record and the outcome have no
field yet.

## What a counter is

A counter is a folder under tosu's `static` folder (`STATIC_FOLDER_PATH`,
`./static` beside `tosu.exe` on Windows) holding at least `index.html`. tosu
serves it at `http://127.0.0.1:24050/<folder>/`. An optional `metadata.txt`
names it (`Name`, `Version`, `Author`, `Usecase: ingame`, `Resolution:
<width> x <height>`, `CompatibleWith: tosu`), and an optional `settings.json`
describes settings the player can change in the dashboard.

## The in-game overlay

It is off by default and turned on with `ENABLE_INGAME_OVERLAY=true` in
`tosu.env`. It is an Electron program that draws through asdf-overlay
(OpenGL for stable and lazer, Direct3D 9 for stable's compatibility mode,
Direct3D 11 for lazer). The player opens its editor in the game with
`Ctrl+Shift+Space` (`INGAME_OVERLAY_KEYBIND`), picks counters from the local
list, and places and scales each one. It keeps that layout in its own
settings, by profile.

## The plan

1. **Dossier serves its part.** A small server in Dossier on `127.0.0.1`
   answers `GET /tosu/context` with the same packet it writes to
   `witness-context.json` today, and lets tosu's origin read it
   (`Access-Control-Allow-Origin: http://127.0.0.1:24050`).
2. **The counter.** `dossier/index.html` with its script and styles: it listens
   to `/websocket/v2`, asks Dossier for the context, and draws the plates agreed
   on the canvas (dark glass, the common tile in the bottom-right corner, the
   unstable rate above the game's meter, PP instead of misses). It covers the
   whole game window, so the player adds it once and stretches it over the
   screen.
3. **Putting it in place.** In its settings Dossier asks where tosu lives and
   copies the counter into its `static` folder, and says how to turn the
   in-game overlay on.
4. **Trying it.** In the CrossOver bottle with stable first, then a tester on
   Windows with NVIDIA in full screen without compatibility mode.

Witness keeps its other work: keeping a replay of every play in the journal,
which tosu does not do.

## Not known yet

- Whether tosu's in-game overlay runs in the CrossOver bottle on this Mac; if
  not, step 4 starts on Windows.
- Whether the counter's page may read Dossier's server from inside tosu's
  in-game browser without more than the header in step 1.
- How the counter should tell the player that Dossier is not running: most
  likely by showing only what tosu knows.
