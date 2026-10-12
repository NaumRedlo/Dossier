# Dossier

An osu! replay engine, written from scratch: it reads an `.osr`, replays it
against the beatmap, decides what every click was worth, and draws the result
as a video.

![A frame the engine drew, with nothing behind it but its own HUD](docs/render.png)

[![ci](https://github.com/NaumRedlo/Dossier/actions/workflows/ci.yml/badge.svg)](https://github.com/NaumRedlo/Dossier/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/NaumRedlo/Dossier?label=release)](https://github.com/NaumRedlo/Dossier/releases/latest)
[![licence](https://img.shields.io/badge/licence-AGPL--3.0-blue)](LICENSE)
![platforms](https://img.shields.io/badge/runs%20on-Windows%20%C2%B7%20macOS%20%C2%B7%20Linux%20%C2%B7%20ARM-lightgrey)

Nothing here wraps the game. The replay parser, the beatmap parser, the
hit-object simulation, the judgement, the renderer and the audio mixer are all
in this repository, in Rust, with no native dependencies at all — which is why
it builds on a Raspberry Pi as readily as on a laptop.

## What is in here

| | |
|---|---|
| `crates/dossier-replay` | `.osr` — the compressed cursor track and what was pressed |
| `crates/dossier-beatmap` | `.osu` and `.osz` — hit objects, timing, slider curves, storyboards |
| `crates/dossier-sim` | replay against beatmap: what actually happened, verdict by verdict |
| `crates/dossier-assay` | star rating and pp, after osu!'s own difficulty and performance calculators |
| `crates/dossier-render` | frames — skins, cursor, sliders, judgements, the HUD |
| `crates/dossier-audio` | hit sounds, the song, and the mix of the two |
| `crates/dossier-produce` | the scene: finding the map and the skin, drawing it, encoding it |
| `crates/dossier-exhibit` | picking the moments worth showing |
| `crates/dossier-cli` | the `dossier` program |
| `crates/dossier-witness` | Witness: reads what a running osu! client holds in memory, and writes nothing |
| `crates/dossier-overlay` | the messages Witness tells an overlay with, and the rules for live values |
| `crates/dossier-hud` | lays out and paints the overlay's plates |
| `native/` | the application — the engine linked in, with a window |
| `tools/` | the harnesses: danser's own judge, the corpus check, and experimental overlay hosts |

A replay file records where the cursor was and which buttons were down. It does
**not** record what each click hit — that has to be reconstructed, and doing so
is the difference between rendering a replay and animating a beatmap.

## What it models

| Piece | |
|---|---|
| **Judgement** | Note lock, hit windows, slider heads, ticks, reverses and tails, combo and accuracy — for stable and for lazer, which disagree about several of these |
| **Scoring** | Stable v1 and v2, lazer's own, the difficulty multiplier, and the mod multipliers as they stand after the June 2026 change |
| **Spinners** | Stable's acceleration model, counted in half turns the way `osu!.exe` counts them; lazer's frame-rate-independent spin history; RPM, bonus and Spun Out |
| **Health** | The drain, where a play would have failed, and Easy's two spare lives |
| **Tracking** | The follow circle only opens once a slide has started, and closes the moment the cursor leaves — as stable does it |
| **Rendering** | Playfield transform, combo colours and numbers, approach circles, reverse arrows, sliders that grow in and retract behind the ball, the spinner in osu! pixels, key overlay, storyboards, and a HUD taken off the client's own source |
| **Audio** | The map's own track, plus hit sounds that follow the *judgement* — a missed note is audible by its silence |

## How closely it matches the game

Synthetic tests only say the engine does what its author intended. The thing
that says it is *right* is the `.osr` header, because osu! wrote it: every
replay carries the score it earned, and the engine's totals are held up against
that figure. Where they disagree, the CLI is built to say **where** — which
slider part was dropped, how hits fall around a window edge, which object the
game's extra combo break must have landed on.

As of 11 October 2026, over a corpus of real replays:

```
181 exact of 191 (12 lazer), total count error 20, none skipped
score compared on 187, worst 0.57%, within 0.5% on 186
```

"Exact" means every count and the max combo agree with the header, verdict for
verdict; it says nothing about pixels, sound or timing of sprites.
[`docs/corpus-status.md`](docs/corpus-status.md) has the measurement and how to
repeat it. Every judgement rule that changed was measured over that corpus before
and after, and several plausible-sounding changes were reverted because it got
worse. Six rendering optimisations were measured and rejected the same way; the
numbers are kept as `#[ignore]` benchmarks so nobody builds them twice.
[`docs/stable-fidelity.md`](docs/stable-fidelity.md) is where the disagreements
that remain are written down.

## Getting it

**A build to run.** [The latest release](https://github.com/NaumRedlo/Dossier/releases/latest)
is a folder rather than an installer — Windows, macOS on ARM and on Intel, Linux on x64.
`ffmpeg` is the one thing still to install.

**From source.** Rust, and nothing else:

```
cargo build --release
```

That writes `target/release/dossier`. Try it on a replay:

```
./target/release/dossier judge path/to/replay.osr
```

The application is its own workspace, because a window brings several hundred
crates and the engine's tests should not have to look at them:

```
cd native && cargo run --release
```

## CLI

```
dossier assay   [OPTIONS] <map.osu>          star rating and pp of a map, with mods and a play
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

## The application

`native/` is what most people run. It finds the replays on the device, plays
them back and turns them into videos, and around that it is a companion for
osu!:

- **Sign in** through Telegram or through osu! alone. Without Telegram a person
  still has a profile, titles, pools, news and videos exchanged with other
  Dossier users; a chat adds its feed and ratings.
- **Community** — the people of your chat and other Dossier users, live plays,
  what happened this week, ratings, titles and a comparison of players.
- **Pools** — an editor for tournament map pools and collections: slots by
  category, maps from links, IDs, the game's collections or suggestions,
  publishing under a code, saving other people's pools and writing them into
  the game as collections.
- **Videos** — send a finished video to a chat or to another player, and
  receive theirs.
- **News** — osu!'s own news and changelog, r/osugame, and public Telegram
  channels.
- **Witness** — a separate small program that watches a running osu!stable
  client by reading its memory, never writing to it: it tells results to the
  chat feed as soon as a play ends, can keep a replay of every play, and has an
  experimental overlay drawn in its own windows on top of the game.
  [`docs/witness.md`](docs/witness.md) says exactly what it reads.

The interface is in English and Russian.

| | |
|---|---|
| ![The feed of a chat: live plays and what happened this week](docs/screens/feed.png) | ![The pools page: drafts as cards](docs/screens/pools.png) |
| ![The pool editor: categories one under another, tiles in rows](docs/screens/pool-editor.png) | ![A frame of a rendered replay](docs/render.png) |

The pictures show sample data the application draws for its own tests.

## Rendering for the bot

The bot at [NaumRedlo/1984](https://github.com/NaumRedlo/1984) takes render
requests from a chat and hands them out to whoever is offering a machine. The
application pairs with the bot, sends finished videos to a chat, and is where a
machine offers itself as a worker.

## Licence, and what in here is not ours

The code is **AGPL-3.0-only**. See [LICENSE](LICENSE).

Some files in this repository are somebody else's work and keep their own
terms:

- **The typefaces.** Varela Round, Commissioner, IBM Plex Mono, M PLUS Rounded 1c
  and JetBrains Mono, all under the SIL Open Font License 1.1, each with its licence text
  beside it in `assets/fonts/`.
- **The application's marks.** The system logos from Simple Icons (CC0 1.0), the
  flags from flag-icons (MIT) and the player's glyphs from Lucide (ISC), each
  with a note beside it in `native/assets/`.

Mod badges are the mods' own names on a coloured plate, drawn by the engine;
there are no pictures of them any more.
