# Dossier: interface rules

This is the working contract for new screens and for gradually updating existing
ones. It keeps the direction the owner chose: dark glass, a saturated red accent,
map covers, compact difficulty pills, as few grey frames as possible. Not every
existing screen follows the contract yet.

## Character and surfaces

- The ground is `theme::GROUND`, main text is `INK`, secondary text is `MUTED`.
  `FAINT` is for minor captions, never for important values.
- The red `ACCENT` marks the main action or the chosen section. Do not fill
  everything with it. An error is also explained in words and with an icon.
- The carousel and the replay view keep their expressive covers and live background.
- Tables, forms and editors use the calm surface `theme::workspace::surface`.
  Content behind it must not get in the way of reading.
- An ordinary panel stands apart by tone and spacing, without a decorative grey
  frame. Frames stay for selection, focus, categories and diagnostic states.
- Real blur comes from the existing GPU compositor. A translucent fill alone is
  not called blur. Do not blur every cell.

## Text and sizes

- Commissioner: body text and semibold headings. IBM Plex Mono: times and
  technical numbers. Nunito ExtraBold: difficulty values in pills.
- Main steps: 12 for metadata, 14 for text, 16 for a row's emphasis, 22 for a
  screen title. Do not shrink text to fit a table.
- Spacing is built on 4: 8 between related items, 16 between blocks, 20 inside a
  working panel, 24–40 at the screen edges.
- Shared radii: 8 for controls, 12 for cards. Pills are for values and compact
  tags; do not turn every container into a pill.
- A working control is 36 logical pixels high by default. Smaller icons keep a
  large enough hit area. The user's scale is kept.

## Actions and states

- One main button per working step; the other actions are quieter.
- Avoid visibly grey button fills and keep dark neutral tones. Use
  `workspace::ITEM` and `HOVER`, not a white translucent fill: on the GPU it can
  look much lighter.
- Tabs and toggles: `workspace::choice`. The choice is always visible, hover
  gives a light highlight, pressing does not change the element's size.
- An unavailable action does not react to hover. If the reason is not obvious,
  explain it next to the action.
- Keyboard focus must look different from selection. Interactive elements are
  moving gradually to controls that the keyboard can activate.
- Loading, an empty result and an error are different states. A server error is
  not called "no internet" when the server answered.
- Listening starts only from a button. Finishing or an error resets playback.
  Never use hover as a hidden command.

## Adaptive layout

- First rearrange blocks, then spacing; do not squeeze the whole canvas.
- A catalogue cell's colour grows with its map count: red when empty, amber
  under five, green darkening up to twenty. Totals per slot and per star range
  sit on the right and at the bottom.
- In the balance desk's catalogue, cells keep a minimum size. The table scrolls
  horizontally, and the scroll must be noticeable.
- The chosen slot's panel and the add map form stay to the right of the table,
  even in a narrow window. The table scrolls on both axes by itself, and the
  side panel's content scrolls vertically on its own.
- Tabs wrap instead of running past the window's right edge.
- Map titles stay on one line with the existing long text scrolling;
  instructions and messages may wrap.
- A section is never hidden just for lack of width. Side panels can be closed
  and return to where the person was.

## Motion

- The response to input starts at once. Animation never delays an action.
- A panel's appearing and closing use the shared duration of existing
  transitions; do not create separate update loops for each control.
- Nothing animates in the background or in the tray where nobody sees it.
- Respect turning animations off where such a setting is already supported.
- Everything animates in place: fading and scaling around its own centre.
  Panels and screens do not lift or slide. The exception is a menu that grows
  out of its button.

A page fades out as a whole. Panels, marks, canvases and backings fade together
with the text:

- `ui::fading(k, …)` multiplies the current fade; `ui::fade()` is not passed
  into it.
- A widget's style runs at draw time, when the page's fade is no longer known.
  Faded colours are computed before the closure: `let fill = ui::faded(CARD)`
  and `.style(move |_| …fill…)`. `theme::slab` and other shared styles without
  a fade are not used on pages; `slab_faded` and `ui::box_at` exist for that.
- A canvas gets `alpha: ui::fade()` as a field and multiplies every colour by it.

Two tests hold these rules: `a_style_takes_its_fade_while_it_is_built` checks the
sources, and `a_page_leaves_nothing_behind_once_it_has_faded_out` compares a
faded page with the screen without it on the GPU. A new page is added to its list.

## Reference screens and checks

Small icons and picture edges are also checked at fractional scale:
`DOSSIER_PREVIEW_SCALE=95` with the size `1512x982` (automatic on the built-in
display) and `DOSSIER_PREVIEW_SCALE=119`. At 100% size differences do not show.

| Screen | Gallery | What to check |
|---|---|---|
| Carousel | `main-rest` | rows, selection, headings, cover contrast |
| Pool editor | `main-pools-editor` | map density, categories, forms and the side panel |
| Balance desk | `main-community-balance-catalog` | table, tabs, cell states, labelling form |
| Balance desk, adding a map | `main-community-balance-catalog-add` | link field, mirror preview, slot choice, warnings |

Every interface drawing check, golden PNG and preview runs only through `wgpu`
on a hardware GPU:

```sh
DOSSIER_PREVIEW_SIZE=1100x760 cargo run --manifest-path native/Cargo.toml --example ui_preview -- main-community-balance-catalog /private/tmp/dossier-design
```

The full set of references in Russian and English:

```sh
bash tools/preview-design.sh /private/tmp/dossier-design
```

For a single iteration set `DOSSIER_DESIGN_FRAMES`, `DOSSIER_DESIGN_SIZES` and
`DOSSIER_DESIGN_LANGUAGES` (space separated lists). The script stops on a GPU
error, with no software fallback.

Check a narrow (980×720), ordinary (1280×800) and wide (1600×1000) window,
Russian and English text, long titles, chosen and empty states.
There is no software fallback renderer. wgpu CPU adapters are refused.
Tests never update goldens on their own; PNGs are reviewed before they replace
anything. Logic and parser tests draw no interface and run as usual.

## Tooltips

Every hover tooltip goes through `native/src/tip.rs`: a dense dark backing with a
soft shadow, the regular face, a muted second line, the key chord on the right.
They appear after 350 ms and fade in over 120 ms; a neighbouring tooltip within
400 ms opens at once. A press dismisses the tooltip, and near the window's edge
it flips. Do not build tooltips on `iced::widget::tooltip`.

Standing explanations are not written as text under a heading or a switch: they
go into the question mark `ui::told` or `ui::explained` next to the label. The
first sentence is semibold, the rest a muted line. Warnings, statuses,
confirmations and empty states stay as text.

## A picture under a veil

A bright picture must not reach a rounded edge: on the antialiased border it
shows through as a thin light arc. A picture under a fill goes only through
`ui::under_veil` (any veil, any corners), `ui::veiled` (a vertical shade) or
`ui::inset_picture`: it is 2 px smaller than its panel, and the fill is 1 px
larger than the picture. A dim picture with nothing over it keeps its own edge. The test
`a_bright_picture_never_shows_as_a_thin_light_edge` checks this on a white
picture.

## The close button

The cross sits on an unobtrusive square backing, `ui::close_square`. The corner
backing `ui::close_corner`, which hugs a corner and repeats its radius, stays
only on notification cards. Small remove buttons on chips stay without a backing.

## The left menu

The menu hides behind the window's left edge and takes no room on the page: at
rest only the sections' ticks show at the edge, the chosen one red. A tick and a
tab are one shape: on hover a tick grows into a tab, starting from the chosen
one, over the page; the page neither moves nor shrinks. The tabs' ends form an
arc, and the tab under the cursor reaches further. Changing the chosen tab and
the highlight are smooth, with no colour jumps. The menu closes 110 ms after the
cursor leaves, and the tabs draw back into ticks along the same curve without a
bounce. The backing is solid; there is no real blur of the page behind it.

## Background and scrolling

The chosen map's background behind the interface is for subscribers only and can
be turned off in the settings. Without it the shared dark red background stays
behind the screen with no veil drawn over it, so it does not change in
transitions.

At a scroll edge the rows fade by themselves: `ui::top_faded` cuts a row into
strips with their own opacity. The replays and the feed work this way. Where the
rows' positions are unknown, `glide::brim` fills the edge with the colour of the
ground behind the page (`ui::page_ground`), not the near-black `GROUND`,
otherwise a dark rectangle shows on a light map.

The hover highlight (`ui::hover`) takes its card's radius: a smaller radius
leaves light arcs in the corners.

## Order of adoption

1. Shared values and styles in `native/src/theme/workspace.rs`; the balance desk
   is their first user. The catalogue gets the form moved aside and cells that
   do not shrink.
2. The pool editor: move shared controls over without losing category colours or
   the compact cards. Check every state of adding a map.
3. The carousel: keep its own composition; unify search, filters, loading and
   messages instead of turning the feed into a table.
4. Apply the checked components to the other working screens. Changes to the
   global theme are checked on all three references.
