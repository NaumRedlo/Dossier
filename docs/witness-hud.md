# Witness overlay plates

The experimental in-process Windows transport lives in
[`tools/overlay-host`](../tools/overlay-host/README.md). It composites the common
tile and available live PP/UR plates into one cropped transparent texture, with
persistent animations and no uploads for unchanged static scenes. It is separate
from Witness Overlay's windows below. An optional context file produced by
Dossier supplies language and the matching loaded pool; it expires after 15
seconds and is scoped to the process and map; Dossier writes it only while the
overlay switch is on. Day summaries have a transport field
but no producer yet. Saved replay outcomes are not yet transported to this host.
The external HUD now consumes the protocol receiver: expired telemetry, disconnect
and a new session clear the visible state and any pending send-key hold.

`crates/dossier-hud` turns what the overlay knows into pictures. It takes the
current `dossier_overlay::View`, a `Context` with what Dossier knows beyond the
snapshot, and a `Stage` that holds the animation, and returns sprites: small
premultiplied RGBA bitmaps with their place in the window. A host inside a client
only has to upload each sprite as a texture and draw it at `x`, `y`.

The look was agreed on the canvas page «Оверлей в игре» on 10 October 2026 and
made plainer on 30 October 2026: every plate is the same flat dark panel with
one corner radius and no edge, labels are in sentence case, the colour of a
medals of the chat leaderboard are the only colours beside the accent, and
there are no avatars, mod chips or pulsing dots. One common tile stays in the
bottom-right corner on every screen.

## What is shown

- **Every screen:** the common tile, only the Dossier mark. When the game is
  first seen in a session it says «Witness подключился» for five seconds and folds
  back to the mark. When a play is kept, a green tick grows in place of the
  mark and its stroke draws itself; the mark comes back after three seconds.
  Nothing on the tile says a play is being recorded.
- **Menu:** up to two Dossier notices in the top-right corner, drawn like the
  notices in Dossier: a round mark (a green tick, a grey «i» or a red «!»), a
  title and one line.
- **Song select:** «Топ беседы», the first three places of the map in the
  chat: a medal-coloured place, the name and the accuracy, with the player's own
  row tinted. Without places there is no card; the pool is not shown.
- **Playing:** the PP reached so far and the unstable rate above the game's own
  hit error meter. While notes are played the PP plate is only a live counter and
  the tile shrinks to the logo; in a break both unfold, and the PP plate adds the
  player's record and the value without misses.
- **Results:** the offer to send the result to the chat with its key. Once the
  hold is done the offer goes away; nothing more is shown.

Nothing is drawn without a view. A watched replay is not a play: it gets neither
the counter nor the rate.

## Look and the keys in the game

`Context::look` (`dossier_hud::Look`) holds the size of every plate in percent,
one of `SCALES` (70, 80, 90, 100, 115, 130), and whether the notices, the
leaderboard in song select, the PP counter and the unstable rate are shown. The size
multiplies the unit every plate is drawn in, positions included, so the plates
keep their corners.

The overlay listens for chords with Ctrl and Shift held and no Alt or Win, only
while the game is in front (`dossier_hud::hold::Chords`, one action per press):

- **Ctrl+Shift+O** opens or closes the tuning panel at the top centre: the size
  and the chords. It closes by itself after eight seconds.
- **Ctrl+Shift+=** and **Ctrl+Shift+−** (or the keypad's + and −) step the size
  up or down and show the panel. The overlay uses the new size at once and
  writes `{"event":"look","scale":N}` on its output; Dossier keeps it in the
  settings and sends it back with the packet. Until that packet arrives, at most
  three seconds, the overlay keeps its own value.
- **Ctrl+Shift+H** hides every plate and brings them back. A short note says
  how to bring them back; while hidden, the send key does nothing.

None of them works during a play but the hiding, because stable reads + and −
there for the offset.

## Context

`Context` is plain serde data, every part optional; a missing part means its plate
is not drawn. `told` fills `Play::resting`, `Play::unstable_rate`, `Play::meter`,
`Play::pp` and `Play::clean` from what Witness tells in a snapshot; without PP
(another ruleset, a map whose file could not be read) the PP plate is not drawn.
`Play::record`, the chat places and the outcome come from Dossier.

## Animation

`Stage::advance` is called once per drawn frame. It moves the fold between the
playing and the break form in `UNFOLD_SECONDS`, folds the greeting of the tile the same way, draws the tick in
`TICK_SECONDS` and pulls the shown unstable rate
towards the latest value with a half-life of `RATE_HALF_LIFE`. The PP counter runs to each new value the same way (`Stage::pp`).
`Stage::moving` tells whether another frame is needed.
`Stage::settled` gives the resting state for a still picture.

## Sizes

Plates are laid out for a window 720 high and scaled by `height / 720 * SIZE`.
`SIZE` is 0.86: the first size looked slightly too big over the real game (the
user's word, 10 October 2026), and the sketches on the canvas were reduced by
the same share. The `scale` of the viewport is not used yet. The height of the game's hit error meter
(`METER_HIGH`) is a guess that needs measuring in the real clients.

## Trying it

```
cargo run -p dossier-hud --example frames -- /tmp/hud
```

writes six sample frames over a made-up backdrop. `cargo test -p dossier-hud`
compares the same frames with the approved pictures in `tests/golden`; set
`DOSSIER_HUD_REVIEW` to a folder to get the current ones.

## The send key

The user chose holding Tab on 10 October 2026: it lies under the left hand, stable
does not use it on the results screen, and F8 opens the chat there. The result is
sent after the key has been held for `HOLD_SECONDS`; the offer shows the hold as a
bar filling up (`Outcome::holding`, from 0 to 1). Whoever listens for the key must
count only a bare hold: no Alt, Ctrl, Shift or Win down, and the game window in
front for the whole time. A short press, Alt+Tab and a nickname completed in the
chat then send nothing.

## The model: tosu

tosu is the reference for how the whole thing works (the user's word, 10 October
2026). What was read from its repository and documents on that day:

- One process beside the game reads its memory and works everything else out
  itself: PP for the hits made so far and PP for a full combo, the unstable rate
  from the list of hit errors, the game's own settings such as the type and size
  of the score meter.
- Its in-game overlay is a separate part built on the asdf-overlay library (Rust,
  MIT or Apache-2.0). That library loads a signed DLL into the game process,
  finds the renderer by itself (OpenGL and DirectX 9 for stable, DirectX 11 and
  OpenGL for lazer) and shows a surface shared on the GPU; it can also listen to
  and block the game's input. tosu fills the surface from an off-screen browser.

What follows for Dossier: Witness is the process beside the game and should
supply `Play` the same way (the unstable rate, the rests, the meter and both PP
values already come with a snapshot); the plates painted by this crate take the place of tosu's
browser and go into one shared surface; loading a DLL into the game is a new
step for Witness, which until now only read memory, and `docs/witness.md` has to
say so before it ships.

## What the Mac cannot show

Measured on 10 October 2026 in the CrossOver 26.3 bottle with osu! stable, by a
small program that asked for the two things asdf-overlay stands on, as a 64-bit
and as a 32-bit process:

- A Direct3D 11 device is made and a texture marked as shared is created, but
  asking for its shared handle answers `E_NOTIMPL` (0x80004001), with and
  without a keyed mutex. A surface cannot pass from one process to another.
- The game's OpenGL (Apple's 2.1 over Metal) has neither
  `GL_EXT_memory_object_win32` nor `WGL_NV_DX_interop2`, the two ways
  asdf-overlay brings that surface into an OpenGL game.

So an overlay drawn inside the game through asdf-overlay cannot be seen on this
machine at all: it can only be tried on Windows. Its DLL also needs the MSVC
toolchain (its hooks link Frida's devkit, which does not link with MinGW), so it
cannot be built here either. Nothing was loaded into the game for this
measurement.

## Witness Overlay, apart from the core

Witness is two programs. **Witness Core** (`witness.exe`, crate
`dossier-witness`) reads the game, keeps replays and talks to Dossier. **Witness
Overlay** (`witness-overlay.exe`, crate `dossier-witness-overlay`) shows the
plates over the game from outside it and never opens the game's process: it
finds the game's window by the PID the frames name, asks the system which keys
are down, and draws. Nothing is loaded into the client.

With `--frames` the core writes the overlay protocol's frames on its output
beside the lines meant for Dossier: the greeting, a snapshot each tick, `kept`
when a replay is saved and `disconnected` when the game is gone. A frame's line
begins with `{"event":"overlay"`, which is how Dossier tells the two apart.
Each snapshot carries `display`, read by the core from the two configuration
keys described below, so the overlay does not read the game's files either.
`--overlay` alone still writes frames only, for `tools/overlay-host`.

Dossier always starts the core with `--frames`, and while the overlay switch is
on it starts Witness Overlay and hands it the frames on its input: the
session's greeting first when the overlay starts mid-session, then each frame
as it comes. A writer thread with a short queue stands between them, so an
overlay that falls behind loses frames rather than holding up the core. Closing
the overlay's input is how it is stopped; it hides its plates and leaves, and
is ended by force only if it is still there two seconds later. A failed start
is tried again after five seconds. Switching the overlay never restarts the
core.

Every plate is a small window of the overlay's own that is layered, lets clicks
through, never takes the focus and stays above other windows
(`crates/dossier-witness-overlay/src/panes.rs`). A plate's window is redrawn
only when its picture or place changes; all of them are hidden while the game's
window is not the one in front, and follow its picture area when it moves or
changes size. The state behind them is `crates/dossier-witness-overlay/src/hud.rs`.

- `--lang en` turns the wording to English.
- `--offer` shows the offer to send after a passed play that was kept, and reads
  the hold of Tab by the rule above (`dossier_hud::hold`), asking the system for
  the state of the keys, with no hook. A full hold takes the offer away and
  writes `{"event":"send"}` on the overlay's output; Dossier reads it and asks
  the bot to post the play.
- `--keeps` says that whoever started Witness keeps the replays, so the tile may
  say «Игра сохранена» after a play; without it (and without `--offer`) a kept
  take changes nothing on the screen. A failed play counts as kept only with
  `--keeps-failed`.
- At its start the overlay writes `{"event":"ready","version":2}` on its output.
  Dossier sends context lines only to an overlay that said it: an older one
  would take them for broken frames. The overlay itself passes over any line on
  its input that is neither a frame nor a context line, without a word in the
  log.
- What the overlay does is told on its error stream in lines that begin with
  `witness overlay:`. Dossier writes that stream to `witness-overlay.log` in
  its own folder, anew at every start of the overlay, so a tester can send it.
  The note that the game's window rose over the plates is written the first
  time and then at most once every 30 seconds, with a count.
- At its start the log names the machine, because whether windows over a full
  screen game are seen depends on it (one tester sees the plates in OpenGL full
  screen, another does not): the Windows edition and build, each graphics
  adapter with its driver version and date, each monitor with its link,
  resolution, position, refresh rate and HDR, the full screen optimization,
  game mode, graphics and GPU scheduling settings from the registry, and the
  compatibility flags and graphics preference set for `osu!.exe`. When the
  game's window is found or moves, a line tells whether it is borderless or has
  a title bar, whether it is kept on top, and whether it covers its monitor
  exactly; if the monitors' modes changed meanwhile (a full screen game may
  change the resolution), they are written again. All of it is read from the
  system and the window; the game's process is still not opened.

In Dossier the overlay has its own tile in the settings, «Оверлей в игре»,
with the switch «Показывать поверх игры», marked «эксперимент» and off by
default (`Settings::witness_overlay`). It says whether the plates are being
drawn or the overlay waits for Witness to see the game. Below it are the size
slider, a switch for each plate and the chords. Dossier passes `--lang en` for
the English interface, `--keeps` when replays are saved to the journal,
`--keeps-failed` when failed plays are saved too and `--offer` when the player
is linked to the bot; a change of any of them starts the overlay again. The
size and the plate switches travel in the packet instead and apply at once.

Failed plays are not saved to the journal unless «Сохранять зафейленные игры»
is on (`Settings::witness_keep_failed`, off by default); they still reach
the bot like before.

### What Dossier tells the overlay

Beside the frames, Dossier writes the context packet on the overlay's input as
one line, `{"event":"context","packet":{…}}` (`dossier_hud::context::Line`),
whenever it changes and every five seconds, so it never expires while Dossier
runs. The packet is the same one `tools/overlay-host` reads from its file:

- `notices`: up to two unseen Dossier notices of the last day, told when the
  game is found and after every kept play;
- `card`: the first three places of the map in the chat
  (`GET /render/maps/{id}/board`), asked when song select shows another map;
- `look`: the size and the plate switches from the settings.

### Sending a play

After the hold Dossier waits until Witness has delivered the play to the bot,
then asks `POST /render/me/play/send` with the replay's hash and the chat chosen
in Dossier (the pinned or first group when none is). The bot draws the same
card as its recent score command: from the official score when the osu! API has
it within ten minutes of the play, otherwise from what Witness kept. The
overlay shows nothing about it; a play the bot cannot post leaves a notice in
Dossier.

The outcome of a play lives until the player leaves the results screen, so an
older score opened from song select is not offered.

### How stable shows itself

Read from the decompiled client of 1 October 2026 (ILSpy; the source stays
outside the repository). The game window is an ordinary WinForms form holding
an OpenTK context, and two keys of the player's `osu!.<user>.cfg` decide how it
reaches the screen:

| `Fullscreen` | `CompatibilityContext` | What the client does | Plates over it |
|---|---|---|---|
| 0 | any | A framed window; one at least as large as the screen loses its frame and is made one pixel taller than the screen | Yes |
| 1 | 0 | OpenGL in a frameless window exactly the size of the screen, the display mode changed with `ChangeDisplaySettingsEx` when the resolution differs; the window is made topmost whenever the game is active | No with NVIDIA |
| 1 | 1 | ANGLE over Direct3D 9, left for a window on Alt+Tab through ppy's own `eglToggleWindowed` | Yes |

The client sets its window topmost only when that state changes, so a plate
raised above it once stays above until the game is activated again.

What a tester saw on 17 October 2026 (Windows 10 Pro 22H2, build 19045.5965,
NVIDIA RTX 3070) turned the guess around: the plates were seen in every case
but full screen without compatibility mode. ANGLE's Direct3D 9 full screen is
composed by Windows like any other window, while NVIDIA's OpenGL driver sends a
window that covers the screen straight to the display, past the composition,
so no window over it can be seen however often it is raised.

What Witness Overlay does with this: every frame it looks whether the game's window
lies above a shown plate and, if so, raises the plate again without taking
the focus (`Panes::keep_above`), saying so on the error stream. It no longer
hides itself in any mode. The core reads the two keys when the configuration
changes and sends them as `display`; Dossier reads `display` from the frames it
forwards, and while the overlay is on and the game runs OpenGL full screen it
says under the switch to turn on compatibility mode or windowed mode.

Not known yet: whether AMD and Intel drivers hide the plates over OpenGL full
screen as NVIDIA's does, and whether the plates cost the game frames where
they are seen (PresentMon's present mode tells). In the CrossOver bottle the plates were drawn but, by
the user's report, were not seen over the game, which ran in a window there.

## Not done

No host draws these sprites inside a client: Witness shows them from outside.
Avatars and map covers are placeholders.
