# Dossier

An osu! replay engine, written from scratch: it reads an `.osr`, replays it
against the beatmap, decides what every click was worth, and draws the result
as a video.

![A frame the engine drew, with nothing behind it but its own HUD](docs/render.png)

[![ci](https://github.com/NaumRedlo/Dossier/actions/workflows/ci.yml/badge.svg)](https://github.com/NaumRedlo/Dossier/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/NaumRedlo/Dossier?include_prereleases&label=release)](https://github.com/NaumRedlo/Dossier/releases)
[![downloads](https://img.shields.io/github/downloads/NaumRedlo/Dossier/total?label=downloads)](https://github.com/NaumRedlo/Dossier/releases)
[![last commit](https://img.shields.io/github/last-commit/NaumRedlo/Dossier/global?label=last%20commit)](https://github.com/NaumRedlo/Dossier/commits/global)
[![licence](https://img.shields.io/badge/licence-AGPL--3.0-blue)](LICENSE)
![platforms](https://img.shields.io/badge/runs%20on-Windows%2C%20macOS%2C%20Linux%2C%20ARM-lightgrey)

Nothing here wraps the game. The replay parser, the beatmap parser, the
hit-object simulation, the judgement, the renderer and the audio mixer are all
in this repository, in Rust, with no native dependencies at all — which is why
it builds on a Raspberry Pi as readily as on a laptop.

## What is in here

| Where | What is there |
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

| Piece | What is modelled |
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

**A build to run.** [The newest release](https://github.com/NaumRedlo/Dossier/releases)
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

`native/` is what most people actually run. It finds the replays on your
machine, plays them back and turns them into videos. Around that it has grown
into a companion for osu!:

- **Signing in** takes a Telegram account or just an osu! one. You do not need
  Telegram to have a profile, titles, pools, news, or to trade videos with
  other Dossier users; a chat with the bot adds its feed and its ratings on
  top.
- **Community** is where the people are: your chat and everyone else who uses
  Dossier, what they are playing right now, what happened this week, who
  climbed, who earned which title, and a side-by-side comparison of any few
  players.
- **Pools** is an editor for tournament map pools and plain collections. Drop
  in links, beatmap IDs or the game's own collections, let it suggest a map for
  an empty slot, publish the result under a short code, save somebody else's
  pool and write it straight into the game as a collection.
- **Videos** go to a chat or to another player, and theirs come back to you.
- **News** gathers osu!'s own news and changelog, r/osugame and any public
  Telegram channel you add.
- **Witness** is a small separate program that watches a running osu!stable
  client. It only ever reads the client's memory. It can tell your chat about a
  play the moment it ends, keep a replay of every play, and draw an
  experimental overlay in its own windows over the game.
  [`docs/witness.md`](docs/witness.md) lists exactly what it reads.

Everything is in English and in Russian.

| The feed of a chat | The pools page |
|---|---|
| ![Live plays and what happened this week](docs/screens/feed.png) | ![Drafts as cards](docs/screens/pools.png) |

| The pool editor | A rendered frame |
|---|---|
| ![Categories one under another, tiles in rows](docs/screens/pool-editor.png) | ![A frame of a rendered replay](docs/render.png) |

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
