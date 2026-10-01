# Vendored crates

## iced_wgpu 0.14.0

The renderer of iced 0.14, unchanged except for how it draws canvas geometry
(meshes). It replaces the crates.io copy through `[patch.crates-io]` in
`native/Cargo.toml`. The upstream licence is in `iced_wgpu/LICENSE`.

### Why

With antialiasing on, stock `iced_wgpu` draws every layer that holds meshes
like this: it ends the frame's render pass, draws the layer's meshes into a
multisampled texture the size of the whole window, resolves it, draws the
result over the frame in a second full-screen pass, and begins the frame's
pass again. On a Retina screen that is about 1.3 ms for the first such layer
and 0.4 ms for each next one, however small the canvases are. `stack!`
children after the first, scrollables and clipped containers are layers, so
the application's screens had 13 to 22 of them: a screen took 25 ms where an
empty one takes 8 (frames measured with readback at 1440×900 at 2×).

### What changed

Only `src/lib.rs` (`render_meshes`, and the mesh branch of `render`),
`src/triangle.rs` and `src/triangle/msaa.rs`.

- Before the frame's pass, each layer's meshes get their footprint: the
  snapped clip rectangles of its meshes, overlapping ones merged into one.
- A layer goes into the first *group* none of whose footprints overlap its
  own. Each group is one pass into the shared multisampled texture, resolved
  into a texture of its own, and the multisampled samples are discarded.
- In the frame's pass, at the place where the layer's meshes used to be drawn
  (after its quads, before its primitives, images and text), the layer's
  footprint rectangles are copied from its group's texture under a scissor.
  The frame's pass is never ended, so a layer still draws exactly where it
  did.
- Without antialiasing the stock path is kept.

Overlapping layers are in different groups, so nothing is ever copied twice
and nothing leaks from one layer into another's pixels. A screen needs 2 to 4
groups, and the same screens cost 8 to 10 ms where they cost 17 to 25.

### Checked

- All 48 `main-*` states, drawn with `ICED_TEST_BACKEND=wgpu` before and
  after, are byte for byte the same.
- `tests/layers.rs` stacks nine layers, canvases and translucent quads, in a
  fixed order and requires the canvases to land where quads of the same order
  land. It fails with the grouping broken (a pixel 34 off), and passes on both
  renderers. Run it with `ICED_TEST_BACKEND=wgpu` after touching this crate.

### Upgrading iced

Port the three files to the new `iced_wgpu`, or delete `vendor/` and the
`[patch]` section if upstream groups mesh layers itself.
