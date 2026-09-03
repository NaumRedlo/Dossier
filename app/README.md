# The Dossier client, with a face

The terminal worker in `../client` claims a job from the bot, fetches what it
needs, runs the engine and sends the video back. This is that, in a window, and
it replaces it rather than wrapping it.

Two things are different by construction rather than by decoration:

- **The engine is linked in, not shelled out to.** "Is the engine there?" and
  "do the builds agree?" were the two commonest ways somebody's first evening
  went, and neither question can be asked of an application that *is* the
  engine. They are gone from the readiness list, not answered faster.
- **One file.** No Python, no virtual environment, no `pip install` — the thing
  people were asked to do before they could lend a machine.

## Its own workspace

`Cargo.toml` here opens a workspace of its own. Tauri brings several hundred
crates with it, and inside the engine's workspace every `cargo test` would have
to look at all of them — the engine's tests are run dozens of times a day. The
path dependency reaches the engine from here and the traffic does not go back.

## Running it

```
cargo run           # from this directory
```

No Node and no `tauri` CLI: the interface is static files under `ui/`, and
`withGlobalTauri` puts `window.__TAURI__` on the page without a bundler. The CLI
is only wanted for building installers, which is a later problem than making the
thing work.

## What is not ported yet, and why it matters

**The hours its owner allowed** — `RENDER_HOURS`, `RENDER_PAUSE` — are not read
yet. The rest of the policy is: `machine.rs` decides whether this computer
should be rendering at all and how hard, from the battery, the thermal
pressure and whether somebody is at the keyboard, with the same thresholds the
terminal client measured. Idle time is read on macOS only; elsewhere an unknown
means somebody is here, which costs speed rather than somebody's machine.

**Hardware on the farm tab.** The bot knows what a worker *told* it — its name,
what it is doing, which build it runs, how many threads it offered — and nothing
about what the machine is made of, because nothing has ever sent that. Showing
processors and memory next to each worker is a change in three places at once:
this sends `hardware` with its claim, the bot stores it, `/render/farm` returns
it. Worth doing, and not something this side can do alone.

**Fetching a map nobody has.** The bot sends what it found when it drew the
card, and the terminal client downloads the beatmap when the machine does not
have it. Here a job whose map is not in the songs folder is handed straight
back, which is honest and not yet useful.

## What it can say about the machine

`machine::profile()` answers three questions at once: what this computer is
(processor, cores, memory, which hardware encoders `ffmpeg` admits to), what the
policy will currently allow, and — the one that matters — **how fast it actually
draws**. That last is measured, not assembled from a specification sheet: a
small map built in memory, a made-up replay, a few dozen frames and a clock.
Two machines running it are drawing the same picture, so the numbers can be
compared. On the machine this was written on: an M4 Pro, twelve cores, on
battery at 71% and therefore six threads, 663 frames a second on one of them.

A farm should sort by that. The other two say what a machine *is*; only the
measurement says what it will do.

## The window

Four tabs and a setup wizard.

**Рендер** — the first steps and then the work. Four cards say what is still
missing (a replays folder, a songs folder, ffmpeg, a skin) and each carries the
button that fixes it; when none is missing they are not there at all, because a
hint that never leaves stops being a hint and becomes furniture. Under them:
size, frame rate, what to draw behind, and this person's own replays, newest
first, each saying whether its map is here.

**Реплей** — the play, in two halves. On the left it is judged and drawn as a
timing graph: every object as a dot, high for late and low for early, coloured
by what it was worth, misses as ticks along the bottom. It takes no frames and
no ffmpeg — the engine already knows what each click was worth, so a play can be
looked at in the time it takes to read the file. The head scrubs, and the
readout says what the combo and the accuracy *were* at that instant. On the
right, the montage: spans cut by hand on a timeline, dragged and stretched, and
then rendered and put end to end into one file.

**Библиотека** — what the application is made of and what it needs. Six engine
crates built in, ffmpeg and a font from outside, three shelves of files, and
what is only planned saying so. Anything missing carries the way to stop being
missing.

**Настройки** — the server, the token and the name in three columns rather than
three paragraphs; the three folders as one line each with a chooser, because
typing a path is the thing people get wrong first; four things about the window
itself; and, folded in beside each other, the readiness list and what this
machine is. The farm is at the bottom and asks the bot only when asked.

The wizard opens only when the server or the token is missing: those are the two
nobody can guess, and asking about anything else is how a setup screen becomes
eleven questions nobody reads. It writes the same `~/.dossier/worker.env` the
terminal client uses — carefully, keeping every line it does not understand,
because that file may hold somebody's `RENDER_HOURS` and a settings screen that
drops it is the worst kind of helpful.

## How it looks, and why

Dark, and only dark, in the bot's own red from `services/image/colors.py` —
three patches of it drifting very slowly behind everything, or standing still if
somebody says so in `Настройки → Окно`. No blur filter and no canvas: a radial
gradient is already soft at the edges, and this has to be affordable on a
machine whose whole job is to be rendering something else. It stops on its own
while the window is not being looked at.

There is a cover, too — the mark, the name and one line — at startup and again
after five minutes of silence. Any movement takes it away.

**The panel floats and grows.** It is a pill over the content rather than a bar
above it, and it can live along the top or down the left — `Настройки → Окно`,
remembered by the window in `localStorage` rather than sent to the bot, which
has no opinion about where somebody keeps their tabs. Under the cursor it
behaves like the dock in macOS: the nearest icon grows most, its neighbours
less, and the tabs move apart by exactly as much as the icons grew. The maths is
in `magnify()` — a Gaussian by distance, then a running sum of the new widths —
and it is done in points rather than in percentages, or the long word
«Готовность» would shove its neighbours twice as far as the short «Ферма».

**One lozenge, not five lights.** The white pill behind the open tab is a single
element that moves; the movement *is* the transition. It reads its place from
where the tab lies plus how far it was pushed, never from the tab's drawn
position — a tab in mid-transition would leave the lozenge a step behind, and
standing still next to it is worse than not having it.

**The icons move only when there is a reason to look at them.** Each tab's icon
animates when that tab is open or under the cursor, and not otherwise: five
things moving at once is a screensaver, not an application. The check draws
itself, the chip's core beats, the film's rails run, the farm's lights blink in
turn, the sliders slide.

Somebody who does not want any of it says so in `Настройки → Окно`, and
`prefers-reduced-motion` is honoured without being asked.

## Updates

Two kinds of machine run this, and an update means a different thing on each.
One has the repository — the application was built from it and `cargo` is right
there. The other has only the application. Which one this is gets decided by
whether the source it was compiled from is still on the disk:
`CARGO_MANIFEST_DIR` is a compile-time path, so in a bundle it names a directory
on somebody else's machine and simply is not there.

Where the source is, a chip appears in the corner saying how far behind this
copy has fallen — and pressing it lists **which parts** change, by crate, from
the diff against `origin`. Then `git pull` and `cargo build --release` run with
every line of both put on the screen as it arrives, and each crate in the list
turns green as its own `Compiling` line goes past. It is the one moment where
watching a build is the point rather than a distraction.

A build that fails offers its log, and nothing leaves the machine until somebody
says so. `Настройки → Обновления` holds the three answers — ask, send, never —
and **ask** is the one it starts on. What travels is the version, the system,
the processor and the log with the home directory cut out of every path in it;
`update::tidy` does that and has a test that says so. A working copy with
changes of its own is told about and left alone.

## The two corners

**The beetle, top right.** Hovering it opens a box for what went wrong, and
three routes out: by mail, as a GitHub issue, or into Telegram. Nothing is ever
sent from here — the mail client or the page opens with the letter already
written, and whether it goes is somebody's own key. What rides along is the
version, the system and the processor; no paths, no token, no name. The Telegram
route puts the text in the clipboard first, because a personal chat cannot be
opened with a message already in it.

The addresses live in one place, `CONTACT` at the top of `ui/app.js`. A route
whose contact is empty does not appear, which is why there is no Telegram button
until somebody writes the handle in.

**The repository, bottom right.** A GitHub mark that says its name when you go
near it. Both corners open links through `link.rs`, which hands the URL to
`open`, `xdg-open` or `cmd /C start` after checking the scheme — `https`, `http`
and `mailto`, and nothing else. Not `tauri-plugin-opener`: that is another
dependency and another permission file for a call each system spells in one
line, and the scheme list is a line worth drawing before the day something
builds a link out of what a server said.

## Where it is

Readiness, the machine, settings and the wizard, drawing a replay of one's own,
and the farm as the bot reports it. What is still in `../client` is the loop
that keeps claiming work unattended: `work_once` here does one turn, on purpose,
so that nothing runs on somebody's machine that they did not press. Until that
loop moves, the Python worker is the one that renders for the bot.
