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
- **Song select:** one compact card with the pool slot of the map, its stars and
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
is not drawn. The overlay protocol does not carry it yet. `Play::resting` (the map
is in a break), `Play::unstable_rate`, `Play::meter` (the game shows its hit error
meter, and at which scale) and `Play::pp` have to be supplied by whoever reads the
client; the day, the pool slot, the chat places and the outcome come from Dossier.

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

## Not done

No host draws these sprites inside a client. Nothing reads the unstable rate, the
break state or the meter settings from a client, and nothing computes PP during
play. Dossier does not build a `Context`. Sending a result to the chat is only
drawn: no key is listened for and the bot has no such request. Avatars and map
covers are placeholders.
