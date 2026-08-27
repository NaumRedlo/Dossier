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

A replay file records where the cursor was and which buttons were down. It does
**not** record what each click hit — that has to be reconstructed, and doing so
is the difference between rendering a replay and animating a beatmap.

### What it models

| Piece | |
|---|---|
| **Judgement** | Notelock, hit windows, slider heads, ticks, reverses and tails, spinner rotations, combo and accuracy |
| **Tracking** | The follow circle only opens once a slide has started, and closes the moment the cursor leaves — as stable does it |
| **Rendering** | Playfield transform, combo colours and numbers, approach circles, reverse arrows, sliders that grow in and retract behind the ball, a HUD |
| **Audio** | The map's own track, plus hit sounds that follow the *judgement* — a missed note is audible by its silence |

### How it is checked

Synthetic tests only say the engine does what its author intended. The thing
that says it is *right* is the `.osr` header, because osu! wrote it: every
replay carries the score it earned, and the engine's totals are held up against
that figure. Where they disagree, the CLI is built to say **where** — which
slider part was dropped, how hits fall around a window edge, which object the
game's extra combo break must have landed on.

Every judgement rule that changed was measured over a corpus of real replays
before and after, and several plausible-sounding changes were reverted because
the corpus got worse. Six rendering optimisations were measured and rejected the
same way; the numbers are kept as `#[ignore]` benchmarks so nobody builds them
twice.

### CLI

```
dossier inspect [--json] <replay.osr>...     read the header alone, no map needed
dossier judge   [OPTIONS] <replay.osr>...    judge, and compare with the header
dossier corpus  [OPTIONS] <replay.osr>...    judge a folder of them, against expectations
dossier sliders [OPTIONS] <replay.osr>...    break slider verdicts down by part
dossier errors  [OPTIONS] <replay.osr>...    how hits fall around the windows
dossier score   [OPTIONS] <replay.osr>...    the score, term by term
dossier health  [OPTIONS] <replay.osr>...    where the drain would have killed the play
dossier debug   [OPTIONS] --from <ms> --to <ms> <replay.osr>   one span, object by object
dossier frame   [OPTIONS] --at <ms> <replay.osr>   one frame to PNG
dossier video   [OPTIONS] <replay.osr>       the whole play to MP4
dossier exhibit [OPTIONS] <replay.osr>       the few seconds worth watching, and why
dossier sounds  [OPTIONS] [-o kit.wav]       audition a hit-sound kit
dossier skin    [OPTIONS] -o <folder>        write the skin out for osu! itself
```

Video encoding shells out to `ffmpeg`; frames are piped to it already converted
to YUV, never touching the disk. The rest is 57 crates deep and not one of them
builds C — no `-sys`, no `cc`, no `nasm`, no `pkg-config` — which is the whole
of why the claim above about a Raspberry Pi is a fact rather than a hope.

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
