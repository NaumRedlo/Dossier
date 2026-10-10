# Witness overlay plates

`crates/dossier-hud` turns what the overlay knows into pictures. It takes the
current `dossier_overlay::View`, a `Context` with what Dossier knows beyond the
snapshot, and a `Stage` that holds the animation, and returns sprites: small
premultiplied RGBA bitmaps with their place in the window. A host inside a client
only has to upload each sprite as a texture and draw it at `x`, `y`.

The look was agreed on the canvas page «Оверлей в игре» on 10 October 2026:
dark glass plates, one common tile in the bottom-right corner on every screen.
There is no blur of the game picture behind a plate; the glass is a translucent
dark fill with a light edge.

## What is shown

- **Every screen:** the common tile. «Witness на связи» normally, «Ведётся
  запись» while a map is played, «Игра сохранена» on the results screen once the
  play is kept.
- **Menu:** «Результаты за день» (PP gained, world rank and its movement, the
  best play of the day, plays, time in game, new records) and up to two Dossier
  notices.
- **Song select:** one compact card with the pool the map is in, its stars and
  three places «в беседе».
- **Playing:** the PP reached so far and the unstable rate above the game's own
  hit error meter. While notes are played the PP plate is only a live counter and
  the tile shrinks to the logo; in a break both unfold, and the PP plate adds the
  player's record and the value without misses.
- **Results:** the offer to send the result to the chat with its key, or what was
  sent once the bot has posted the card.

Nothing is drawn without a view. A watched replay is not a play: it gets neither
the counter nor the rate.

## Context

`Context` is plain serde data, every part optional; a missing part means its plate
is not drawn. `told` fills `Play::resting`, `Play::unstable_rate`, `Play::meter`,
`Play::pp` and `Play::clean` from what Witness tells in a snapshot; without PP
(another ruleset, a map whose file could not be read) the PP plate is not drawn.
`Play::record`, the day, the pool, the chat places and the outcome come from
Dossier.

## Animation

`Stage::advance` is called once per drawn frame. It moves the fold between the
playing and the break form in `UNFOLD_SECONDS`, pulls the shown unstable rate
towards the latest value with a half-life of `RATE_HALF_LIFE`, and runs the pulse
of the recording dot. `Stage::moving` tells whether another frame is needed.
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

## Shown by Witness, without entering the game

`witness.exe --serve --hud` shows the plates over the game from outside it.
Nothing is loaded into the client: every plate is a small window of Witness's own
that is layered, lets clicks through, never takes the focus and stays above
other windows (`crates/dossier-witness/src/panes.rs`). A plate's window is
redrawn only when its picture or place changes; all of them are hidden while
the game's window is not the one in front, and follow its picture area when it
moves or changes size. The state behind them is `crates/dossier-witness/src/hud.rs`:
it takes the same snapshots the overlay protocol carries, so the PP, the unstable
rate, the rests and the meter are the live ones.

- `--lang en` turns the wording to English.
- `--offer` shows the offer to send after a passed play that was kept, and reads
  the hold of Tab by the rule above (`dossier_hud::hold`), asking the system for
  the state of the keys, with no hook. A full hold takes the offer away and
  writes a line to the error stream; nothing is sent yet.
- `--keeps` says that whoever started Witness keeps the replays, so the tile may
  say «Игра сохранена» after a play; without it (and without `--offer`) a kept
  take changes nothing on the screen.
- What the HUD does is told on the error stream in lines that begin with
  `witness hud:`; the ordinary lines for Dossier stay on the output.

In Dossier this is the switch «Оверлей в игре» in the Witness tile of the
settings, marked «эксперимент» and off by default (`Settings::witness_overlay`).
When it is on, Dossier starts Witness with `--hud`, with `--lang en` for the
English interface and with `--keeps` when replays are saved to the journal;
changing any of the three starts Witness again. Dossier never passes `--offer`:
nothing can be sent yet.

The outcome of a play lives until the player leaves the results screen, so an
older score opened from song select is not offered.

Not known until it is tried on Windows: whether these windows are seen over a
game in exclusive full screen (over a window and a borderless window they
should be), and whether they cost the game any frames. In the CrossOver bottle
they were drawn but, by the user's report, were not seen over the game.

## Not done

No host draws these sprites inside a client: Witness shows them from outside.
Dossier does not build a `Context`, so the day, the pool, the chat places and the
record a play is compared with are not shown yet. The send key is read, but the
bot has no request for posting a result. Avatars and map covers are
placeholders.
