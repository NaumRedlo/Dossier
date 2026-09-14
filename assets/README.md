# assets

What the engine reads off disk rather than out of a skin.

## `fonts/`

Four faces, every one of them under the SIL Open Font License 1.1, every one
with its licence text beside it. They are a **chain rather than a choice**: the
engine asks each in turn for the character it is about to draw and takes the
first that has it.

| | |
|---|---|
| `VarelaRound-Regular.ttf` | first, and most of what a HUD says — the score, the accuracy, the combo, the counters, mod acronyms, a player's name when it is Latin |
| `Commissioner-Regular.ttf` | Cyrillic, which Varela Round does not carry at all |
| `MPLUSRounded1c-Regular.ttf` | kana and 4,954 kanji, which is a great many osu! titles |
| `JetBrainsMono-Bold.ttf` | last, for a sign none of the three has, so one missing character does not take the line down to nothing |

`dossier-produce` looks for them beside the program, in `assets/fonts` beside
it, and in the checkout, so a render started from either finds them with
nothing configured. `--font <path>` puts a different face at the front of that
chain, and `$DOSSIER_FONT` does the same for a whole machine — **it wins over
the search**, which is worth knowing, because a stale one set months ago will
quietly draw every render in a face nobody chose.

Without any font the play still draws — the numbers simply do not.

`JetBrainsMono-ExtraBold.ttf` is baked into the binary rather than read, because
it letters the mod badges and a badge with no lettering on it is not a badge.
`JetBrainsMono-Regular.ttf` is the application's monospace face and is here so
that both halves ship from one folder.

Only the weights the engine actually asks for live here. The application keeps
its own copies under `app/ui/fonts/`, with the bold and semibold of Commissioner
and M PLUS that an interface needs and a renderer does not.

### What was here before

Until 2026-09-09 the HUD was set in **Torus Notched**, osu!'s own face:
© 2018 Paulo Goode, all rights reserved. It was never ours to relicense or to
put in a release, which is exactly what the release workflow had been doing.

Until 2026-09-10 it was **Huninn**, which is properly licensed and was replaced
for fit rather than for paperwork. **Proxima Soft** was tried for an afternoon
in the application and taken out again: it is a retail font, and a public
repository that ships its binaries is the wrong place for one.

The rule the three of them add up to: **if the licence does not permit
redistribution, it does not belong in a repository that publishes its own
builds** — however good it looks.

## `mods/`

The badges the renderer draws for a play's mods, and the SVG they are made
from. See the README there: they are other people's drawings and it names every
author.

## `hitsounds/`

Empty, and documented rather than filled: see the README there. A skin carries
its own samples, `--samples <dir>` names a folder, and `--kit <name>` synthesises
a set instead.
