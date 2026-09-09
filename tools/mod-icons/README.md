# Making the mod badges

`assets/mods/*.png` is what `dossier-render` bakes into the binary, and this is
what makes those files out of `assets/mods/svg/*.svg`. Run it after changing a
source and commit both sides:

```bash
cargo run --release --manifest-path tools/mod-icons/Cargo.toml
```

It takes the source folder and the output folder as arguments and defaults to
those two, so it wants to be run from the root of the checkout.

## What it does to a drawing

The badge is a coloured plate with white ink on it, and the icons arrive as
black artwork on nothing, drawn at whatever size and with whatever padding
their author left inside the `viewBox`. So each one is rendered eight times
larger than it will be used, every pixel is repainted white at the alpha the
renderer gave it, anything fainter than 8/255 is dropped — that is where the
`fill-opacity="0.01"` spacer rectangle some sets carry goes — and what is left
is cropped to the ink, fitted into a 66-pixel box and centred on the 120×84
plate the renderer expects.

Fitting the ink rather than the `viewBox` is the point: two icons that were
drawn with different margins come out the same visual weight, which is the
thing the eye notices when a row of badges sits under a replay.

Only `resvg` and `tiny-skia` are needed, and it is deliberately its own
workspace: the engine does not read SVG at runtime and should not carry a
parser for it.
