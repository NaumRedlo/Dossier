# assets

What the engine reads off disk rather than out of a skin.

## `fonts/TorusNotched-Bold.ttf`

The typeface the HUD and the combo numbers are set in — osu!'s own, so a render
looks like the game rather than like a debug overlay. `dossier-cli` looks for
it beside the working directory, so a render started from a checkout finds it
with nothing configured; `--font <path>` or `$DOSSIER_FONT` names another.

Torus Notched is © 2018 Paulo Goode, all rights reserved. It is here because
the engine is drawing osu! and nothing else looks right; it is not part of what
this repository's licence covers, and it is not ours to relicense. Building
against a different face is one flag away and changes nothing else.

Without any font the play still draws — the numbers simply do not.

## `hitsounds/`

Empty, and documented rather than filled: see the README there. A skin carries
its own samples, `--samples <dir>` names a folder, and `--kit <name>` synthesises
a set instead.
