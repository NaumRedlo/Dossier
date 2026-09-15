# The application's design

Written 2026-09-15, before the first line of the native application, so that
the code reads its decisions from here rather than inventing them screen by
screen. The web application in `app/` was drawn the other way round — one
screen at a time, each with its own idea of a colour — and that is the reason
it is being replaced.

## The reference

One picture set the tone: a task panel, near-black, a single warm accent, a
headline saying what is happening now and how far along it is — *Restart
AirDrop · 5/24* — and beneath it a monospaced ledger where finished lines carry
a quiet tick and the current line is the only bright thing on the screen. As
lines finish, the list advances smoothly; nothing jumps, nothing shouts.

What it does right, in the order that matters:

1. **One thing at a time.** The headline is the whole status. Everything else
   is history or detail.
2. **Progress is a ledger, not a bar.** Steps are named; the eye reads what was
   done, not how much of a rectangle is filled.
3. **Two voices.** A sans headline, a monospaced ledger. Nothing else.
4. **One hue.** The accent is the only colour. Done and current share it; they
   differ in brightness, not in hue.
5. **Nothing moves suddenly.** Lines slide, they do not appear.

Dossier's application is built from these five, and a screen that breaks one
of them is wrong even if it looks fine.

## Tokens

These live once, in `native/src/theme.rs`. No colour, size or duration is
written anywhere else.

### Colour

| token | value | for |
|---|---|---|
| `ground` | black into burgundy — see *The background* | the window |
| `raised` | white 4.5% | cards, the sidebar |
| `sunk` | black 26% | fields, wells, the stage behind a frame |
| `line` | white 8% | every border; there is no other border |
| `line-high` | white 16% | a border under the pointer |
| `ink` | `#ece7e2` | the current line, headlines, values |
| `muted` | `#a9a29b` | finished lines, captions, secondary text |
| `faint` | `#6b655f` | what may be ignored |
| `accent` | `#e24848` | the mark's own red: the dot, the tick, the primary button (white on it) |
| `accent-soft` | accent 16% | a selected row, the halo of the dot |
| `danger` | `#e24848` | failure and destruction — the same red, told apart by glyph and words, never by hue |

One red, chosen 2026-09-15 over a warm gold and a rose: it is the colour the
mark already owns. It means "alive" on a dot and "done" on a tick; it means
"wrong" only together with a cross and a red sentence. A running line is a red
dot with `ink` text; a failed line is a red cross with red text. The glyph and
the words carry the difference, so a person who cannot tell the reds apart
still can.

Green does not exist. A finished step is a red tick, not a green one.

### The background

The window is black that flows into burgundy towards the top — the ground the
web application had, kept on purpose:

```
radial-gradient(115% 95% at 50% -18%,
  #3a1015 0%, #26090f 30%, #17070b 55%, #0d0508 78%, #070304 100%)
```

It gives the screen a top and a bottom the way a lit room does, and the
accent has somewhere to come from. Cards are glass on it — `raised` is a white
tint, not a paint — so the burgundy shows through them and nothing on the
screen is a grey slab. The *Background* setting keeps its four levels: the
slow drift of colour behind the content for *rich* and *live*, a still
gradient for *quiet*, and a flat `#0d0508` for *still*, which computes nothing.
The gradient is the only one in the application besides that drift; nothing
else fades from one colour to another.

### Type

| face | for |
|---|---|
| **Commissioner** 400 / 600 | everything that is a sentence: headlines, labels, buttons, captions |
| **JetBrains Mono** 400 / 700 | everything that is a record: ledgers, numbers, names of files and devices, tags |
| **M PLUS Rounded 1c** | fallback for kana and kanji, via the text stack's own fallback |

Varela Round leaves the application. It has no Cyrillic, and an interface in
Russian that switches face mid-sentence is the kind of seam this document
exists to remove. The engine's HUD keeps it.

Four sizes and no fifth:

| | px | line |
|---|---:|---:|
| `caption` | 12 | 16 |
| `body` | 14 | 20 |
| `lead` | 16 | 22 |
| `title` | 22 | 28 |

Numbers are tabular everywhere.

### Space and shape

- Grid of **8 px**; the allowed gaps are 4, 8, 12, 16, 24, 32, 48.
- Cards: radius **12**, padding **24**, border `line`, no shadow.
- Controls: height **32**, radius **8**, padding 0 12.
- The sidebar is **220 px**; content has **40 px** on each side.
- Shadows exist only under things that float — a sheet, a menu, a toast — and
  only one: `0 16px 40px black 45%`.

### Motion

- One curve, `cubic-bezier(0.22, 0.9, 0.28, 1)`; **200 ms** for a state
  change, **320 ms** for something entering or leaving, **450 ms** for the
  ledger advancing.
- A line that finishes fades to `muted` and the list slides up; a new line
  fades in from below. Nothing pops in at full opacity.
- A change of screen crossfades; the sidebar and the headline never move.
- With reduced motion, every duration is zero. Nothing depends on an animation
  having happened.

## Language

The application speaks **English first and Russian second**, and both are
first-class: every screen is drawn and approved in both, every layout fits the
longer of the two strings, and a string that exists in one language and not the
other fails the build. The engine's users are the international community; the
bot's are Russian-speaking; neither is an afterthought.

- Strings live in Fluent files, `native/lang/en-US.ftl` and `native/lang/ru-RU.ftl`,
  and nowhere in the code. Fluent is chosen for Russian plurals — *1 карта,
  2 карты, 5 карт* — which no format string handles honestly.
- The system locale picks the language the first time; the first screen of the
  first run lets the person change it, and Settings keeps that choice.
- This document writes copy in English and gives the Russian beside it where
  the wording matters. Golden frames are named with the locale.

## The ledger

The application has one way to show that something is going on, and it is the
panel from the reference. It is used for a render, a farm job, a download of
maps, the first-run checks, an update — anything with steps.

```
● Rendering for the bot · 4 of 6          ● Рисую для бота · 4 из 6

  ✓ Replay received                         ✓ Реплей получен
  ✓ Map on disk                             ✓ Карта на месте
  ✓ Judgement agrees                        ✓ Судейство сошлось
  ● Drawing · 4,512 of 7,280                ● Рисую · 4 512 из 7 280
    Encoding                                  Кодирую
    Delivering                                Отдаю боту
```

- The headline is the current step and the count, parted by a middle dot
  that stands as far from the one as from the other; the same dot parts a
  line's name from its detail.
- When there is a fraction worth knowing, it sits on the current line, not
  in the headline.
- Finished steps are `muted` with a red tick. The current step is `ink` with
  the red dot. Steps to come are `faint` with nothing.
- At most five lines are visible; older finished lines scroll out of the top
  as new ones finish. The scroll is the 450 ms slide.
- A failed step turns `danger` and the ledger stops there, with one line of
  reason beneath it and the button that undoes or retries. Nothing else on the
  screen goes red.
- There is no percent bar. If a step is long, its own line carries the
  fraction and, if it is known, the time left.

Every producer of progress in the engine and the application reports steps
into this shape. A screen never invents its own spinner.

## Screens, and the least each may show

The rule from the person this is for: *minimise the information, or it is
easy to lose the thread and to see the flaws.* Each screen below lists
everything it shows. Adding a line to a screen means adding it here first.

**The mark** is the letter D with four slits through it, evenly spaced, red.
The application icon puts it on a rounded tile that runs from the burgundy at
the top of the ground to the black at its bottom; inside the window it stands
bare beside the word, centred above whatever the screen is about, and it is
the one thing that reacts to the application's own moments — a check passing,
the bot saying yes — with a beat of 420 ms. No rings, no border, and the very
same letter, from the same drawing, sits in the heart of the QR.

**Sidebar.** The mark and the word *Dossier*. Three items: Реплеи, Ферма,
Настройки. At the bottom, one ledger headline for whatever is running, or the
device's state in one line — *Готов брать работу*, *Работу не беру*, *Не
готов*. Nothing else.

**Реплеи.** Title with the count. A search field and a sort. Three chips: Все,
С картой, Без карты, each with a count. Cards. That is the screen.

A card: the frame; the client and the outcome as two small tags on it; the
player; the map in one line; the accuracy; mods as lettered badges, the combo,
the date. Hovering shows one button. Nothing else is on a card.

**The replay.** Opens over the list as a sheet. Two tags. The frame with play,
a scrub bar and the time. Map, then player · date · mods. Three tiles:
accuracy, combo of possible, score. The four counts with their dots. One line
saying whether the judgement agrees with the replay's own header. Three
buttons: Отрендерить, Судейство, Ролик. Three quiet links: show in folder,
open, delete.

**Ферма.** Title. One switch: take work from the bot. Two cards: the device
(state line, three tiles: speed, delivered, handed back; then only the checks
that fail, each with its fix) and the ledger of what is being drawn now. Below,
who else is online: device, state, threads, delivered.

**Настройки.** A rail of eight: Подключение, Папки, Видео, Звук, Скины, Игра,
Клиент, Авторство. Each row is a name, at most one short line under it, and
its control on the right. No row explains what the person can see for
themselves.

**First run** is built first, and it is the ledger applied to setting up. No
sidebar; one column of 560 px in the middle of the window; the mark and the
word above it. The headline reads *Setting up · 2 of 4* and the ledger beneath
it names the four steps — Language, osu! folder, This device, The bot — with
the current one bright. Under the ledger, one card for the current step: a
title, at most one line of why, the control, and a row of two buttons —
*Continue* and a quiet *Back* or *Skip*. Nothing else is on the screen.

1. **Language.** Two options, *English* and *Русский*, the system's one
   preselected. This is the only step that cannot be skipped, and it is first
   so that every word after it is in the right language. Choosing the other
   one retypes every word on the screen over 640 ms: the old words are
   erased from the right first, then the new ones are typed from the left —
   the whole interface, not the card alone.

2. **osu! folder.** The application has already looked, and it looks the way
   the game does rather than by guessing folder names:

   - **stable** is a folder with `osu!.exe`, or with `Songs/` beside a
     `osu!.<user>.cfg`. That file is read: `BeatmapDirectory` may point the
     songs elsewhere, and the application follows it rather than assuming
     `Songs/`. `Skins/` and `Replays/` are checked separately, and one that
     is missing is said so — *replays: none yet* — not treated as a failure.
     Roots tried: `%LOCALAPPDATA%\osu!` and `%PROGRAMFILES%\osu!` on
     Windows; `~/osu!`, `~/osu`, `~/Games/osu!` and every Wine prefix's
     `AppData/Local/osu!` elsewhere.
   - **lazer** is a data folder with `client.realm` and `files/`:
     `%APPDATA%\osu` on Windows, `~/.local/share/osu` on macOS and Linux —
     and its `storage.ini` is read first, because lazer lets the person move
     that folder and writes the new place there. Lazer keeps everything in a
     content-addressed store the application cannot name files in without
     the database, so it takes from lazer only what it can recognise by
     content: **replays** — every file in the store that parses as an `.osr`,
     and everything in `exports/` — and **skins** exported as `.osk`. Maps
     are not taken from lazer at all: a replay's map is found in stable's
     folder when there is one, and otherwise fetched from the mirrors the
     application already uses. The card says exactly that in one line.
   - Both found: two rows, one per client, each with its path, its counts
     and a switch that is on — *Use both* is the button, and turning one
     switch off is how a person chooses. See *Sources* below for what "both"
     means. Neither found: *Couldn't find osu! on this device*, one line
     saying the application can keep maps, skins and replays in a folder of
     its own, a *Browse…* button that accepts either kind of folder, and a
     quiet *Keep everything in its own folder*, which makes `~/.dossier` a
     source like any other — with its own Songs, Skins and Replays — and
     shows it as one, tagged *dossier*. Nothing after that step changes: the
     bot stays optional, replays dropped into that folder render locally.

   Found: the client's name as a tag, the path in a well, and three tiles —
   maps, skins, replays — with *Use this* and a quiet *Add another…*, which
   accepts a second install, a lazer folder or a plain folder of replays.

   **Sources.** The application does not have a songs folder, a skins folder
   and a replays folder; it has a list of *sources*, each a place it knows
   how to read: a stable install (maps, skins and replays, through the
   game's own config), a lazer data folder (replays and exported skins), or
   a plain folder of `.osr` files. Everything shown is the union of the
   sources. A replay or a skin remembers which source it came from and wears
   the client's tag; the same replay in two sources — the same replay hash —
   is shown once. A replay's map is looked for in every stable source and
   then on the mirrors. Downloaded maps land in the application's own
   folder, never inside a game's. The first run adds what it found; Settings
   → Folders is the same list, with *Add…* and the switch per source, and
   nothing else on that page.

3. **This device.** One field, prefilled with the machine's name. One line:
   *This is how the farm will see it.* (*Так это устройство увидят в ферме.*)

4. **The bot.** No address and no token: the application knows the bot, and
   the person only has to be recognised. The card shows a code — `K7QN-M4XZ`,
   eight letters from an alphabet without `0`/`O`/`1`/`I`, good for ten
   minutes, single use — a QR beside it that encodes
   `https://t.me/<bot>?start=pair-K7QNM4XZ`, and a button *Open Telegram*
   that opens the same link on this machine. Under the code, one line with
   the red dot: *Waiting for Telegram…*. The person scans or taps; the bot
   answers with the card from the roadmap — *Add „MacBook Pro" — macOS on
   ARM, 12 cores, build 0.11.0?* — and a button; on yes the application's
   next poll comes back linked. The line becomes *Linked to the bot* with a
   red tick, the code and the QR go, and *Continue* lights up. One quiet
   link, *Later — just my own replays*, and nothing else: there is no field
   to type a code into. The code on the screen is not for typing — the bot's
   card repeats it, and the person holds the two against each other before
   pressing yes.

   The QR is drawn, not pasted: round modules, finders rounded only as far
   as a reader still finds them, the mark in the middle at error-correction
   level H, and a test that reads the drawn code back into the link.

   Linking is also what lets the application show the person their own side
   of the bot — what they have queued, what was rendered for them, what
   their device has done — which is a screen of the main menu and is
   designed with it.

Then the ledger runs on its own — *Checking · 3 of 4*: osu! folder, ffmpeg,
engine, bot — each line ticking as it passes. Everything passed: one button,
*Open Dossier*. A line failed: that line turns `danger` with its reason and
its fix — *ffmpeg is not installed · Where to get it* — and the button becomes
*Continue anyway*, because a missing ffmpeg stops rendering, not judging. A
bot that was skipped with *Later* is not a failure: its line stays quiet,
*not linked*, and counts as done.

The other way round — osu! found, bot skipped with *Later* — is the ordinary
case for someone who only wants their own replays drawn. Then the farm is
not hidden but idle: its screen shows the same code-and-QR card instead of
the device's status, and the sidebar's bottom line reads *Not linked to the
bot* rather than a readiness. Nothing about the bot appears anywhere else.

The main screen is designed after this flow is approved, not before.

**Судейство and Студия** are deferred. They are the engine's own views and
deserve their own document once the five above are real.

### Two renderers, one look

The window draws with `wgpu`, which blends colours in linear light; the
software fallback, `tiny-skia`, blends in sRGB. The translucent tokens above
are written the way a stylesheet would write them — white 4.5 % — and read
that way on the software renderer; the code carries their linear-light
equivalents (white 0.77 %, black 48 %, and so on) so the window shows the same
glass. The approved frames are always taken with the renderer the window
uses on that machine, and a frame's file name says which; a machine without a
GPU takes its own set rather than comparing against another renderer's.

## What is checked by a machine

Every screen above is drawn as a pure function of its state, so every state of
it can be drawn without a window and compared with a picture that was approved.

- `tests/golden/<screen>-<state>-<size>.png` — approved frames. A frame is
  approved once, by a person; after that any difference fails the build.
- States: empty, loading, ready, busy, failed, first run. Sizes: 980×720,
  1280×800, 1920×1080.
- Invariants checked on every frame, with no picture needed: no text is
  clipped by its box, nothing lies outside the window, every clickable thing
  is at least 28 px tall, every text meets 4.5:1 against what it sits on, no
  size is NaN.
- `native --gallery <dir>` draws every screen in every state at every size to
  PNG, which is how the frames are reviewed before anyone runs the window.

## The stack this assumes

Rust throughout; `iced` on `wgpu`, with its software renderer as the fallback;
text through `cosmic-text`, which does the shaping and the fallback the
application needs; `iced_test` for the frames above. The engine stays on the
CPU and on `tiny-skia`, because a video for the farm must be identical on
every machine and a GPU does not promise that. What Tauri used to provide —
tray, dialogs, links, bundles — comes from `tray-icon`, `rfd`, `open` and
`cargo-bundle`.
