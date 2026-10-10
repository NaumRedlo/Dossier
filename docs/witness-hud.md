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

Plates are laid out for a window 720 high and scaled by `height / 720`. The
`scale` of the viewport is not used yet. The height of the game's hit error meter
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

## Not done

No host draws these sprites inside a client. Dossier does not build a `Context`,
so the record a play is compared with is not known yet. Sending a result to the chat is only
drawn: no key is listened for and the bot has no such request. Avatars and map
covers are placeholders.
