# assets

What the engine reads off disk rather than out of a skin.

## `fonts/`

Two faces, both under the SIL Open Font License 1.1, both with their licence
text beside them:

| | |
|---|---|
| `Huninn-Regular.ttf` | everything the engine writes over the play — the score, the accuracy, the combo, the counters, the key labels, the signature |
| `JetBrainsMono-Bold.ttf` | what Huninn has no glyph for, so a sign missing from one face does not take the whole line down to nothing |

`dossier-produce` looks for them beside the program, in `assets/fonts` beside
it, and in the checkout, so a render started from either finds them with
nothing configured. `--font <path>` names a different face for the front of
that pair, and `$DOSSIER_FONT` does the same for a whole machine — **it wins
over the search**, which is worth knowing, because a stale one set years ago
will quietly draw every render in a face nobody chose.

Without any font the play still draws — the numbers simply do not.

`JetBrainsMono-ExtraBold.ttf` is baked into the binary rather than read, because
it letters the mod badges and a badge with no lettering on it is not a badge.

### Torus is gone

Until 2026-09-09 the HUD was set in Torus Notched, which is osu!'s own face:
© 2018 Paulo Goode, all rights reserved. It was here because the engine draws
osu! and nothing else looked right, and it was never ours to relicense or to
put in a release — which is exactly what the release workflow was doing, and
what a public repository had been doing for longer.

Huninn replaces it outright. The file is out of the tree, out of the release,
and out of the Python client's default, which had been pointing `$DOSSIER_FONT`
at it and so overruling the very lookup that was meant to have retired it.

## `mods/`

The badges the renderer draws for a play's mods, and the SVG they are made
from. See the README there: they are other people's drawings and it names every
author.

## `hitsounds/`

Empty, and documented rather than filled: see the README there. A skin carries
its own samples, `--samples <dir>` names a folder, and `--kit <name>` synthesises
a set instead.
