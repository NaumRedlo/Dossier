# Dossier

An osu! replay engine, written from scratch: it reads an `.osr`, replays it
against the beatmap, decides what every click was worth, and draws the result
as a video.

> **Рендерите реплеи для бота?** Пошаговая инструкция на русском —
> [onenineeightfour.ignorelist.com/guide](https://onenineeightfour.ignorelist.com/guide).
> Всё остальное здесь по-английски, но по той ссылке этого знать не нужно.

Nothing here wraps the game. The replay parser, the beatmap parser, the
hit-object simulation, the judgement, the renderer and the audio mixer are all
in this repository, in Rust, with no native dependencies at all — which is why
it builds on a Raspberry Pi as readily as on a laptop.

```
crates/dossier-replay     .osr — the compressed cursor track and what was pressed
crates/dossier-beatmap    .osu and .osz — hit objects, timing, slider curves
crates/dossier-sim        replay against beatmap: what actually happened
crates/dossier-assay      what it was worth: 300s, 100s, misses, the note lock
crates/dossier-render     frames — skins, cursor, sliders, judgements, the HUD
crates/dossier-audio      hit sounds, the song, and the mix of the two
crates/dossier-exhibit    picking the moments worth showing
crates/dossier-cli        the `dossier` program the bridge runs
client/                   the Python bridge, and the render client
```

## Building it

Rust, and nothing else:

```
cargo build --release
```

That writes `target/release/dossier`. Try it on a replay:

```
./target/release/dossier judge path/to/replay.osr
```

`ffmpeg` on `PATH` is needed to render video — the engine draws the frames and
hands them to ffmpeg to encode.

## Rendering for the bot

The bot at [NaumRedlo/1984](https://github.com/NaumRedlo/1984) takes render
requests from a chat and hands them out to whoever is offering a machine. The
render client in `client/` is the machine's side of that: it polls, claims a
job, renders it and uploads the video. It needs two lines in
`~/.dossier/worker.env`:

```
RENDER_SERVER=https://onenineeightfour.ignorelist.com
RENDER_WORKER_TOKEN=...
```

and then:

```
python client/worker.py --check    # says what is and is not ready
python client/worker.py            # then run it
```

`--check` answers every question at once instead of one failure at a time, and
it reaches the bot without claiming anybody's replay.

It pulls rather than listens, so nothing has to be reachable from outside: no
port is opened and the address may move. How hard it works is not the client's
decision either — it reads the battery, the energy mode, whether anyone is at
the keyboard and whether the machine is hot, and will refuse work rather than
make a laptop unpleasant to sit in front of.

Windows, macOS and Linux, including ARM.

## Licence

AGPL-3.0-only. See [LICENSE](LICENSE).
