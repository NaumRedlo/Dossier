# Where the engine goes next

Written 2026-08-11 and rewritten 2026-08-27, from the state the engine is
actually in rather than from where it was meant to be. Two sibling documents
already say what was decided and why — [`stable-fidelity.md`](stable-fidelity.md)
for judgement, [`exhibit.md`](exhibit.md) for selection — and this one only says
what is left.

The August rewrite is not a tidy-up. Two of the three items the old order named
are done — the engine has a repository, this one — and the third, the corpus,
turned out to be back. Measuring against it changed what the rest of this
document had to say, which is the whole argument for having an instrument.

## Where it stands

Eight crates, about 40,000 lines of Rust, 715 tests. The Python that drives it
is in `client/` — 4,000 lines and 143 tests — and is the same code the bot and a
render worker both run.

| crate | lines | what it is |
|---|---:|---|
| `dossier-replay` | 1,342 | `.osr` parsing |
| `dossier-beatmap` | 1,609 | `.osu` parsing, slider geometry |
| `dossier-sim` | 6,060 | judgement, scoring, health — the part with a right answer |
| `dossier-assay` | 4,024 | what a play was worth, told back in numbers |
| `dossier-render` | 12,275 | frames, skin, elements |
| `dossier-exhibit` | 2,002 | which seconds of a play are worth watching |
| `dossier-audio` | 2,504 | hit sounds |
| `dossier-cli` | 10,252 | the commands, video, reels, the skin exporter |

Rendering has a look of its own, a slow-motion pass with a camera, and a skin
osu! can wear. Judgement is where the news is, and it is below.

## Where judgement actually stands

The corpus is back, and larger than it was. 149 replays on disk against a
manifest of 134 — but they are not the same 134: only 33 of the manifest's rows
are present, and 112 of the replays are ones it never listed. Whatever was lost
was replaced by more than was lost.

Measured 2026-08-27, against build `21380ed`:

```
dossier corpus --songs ~/.osu/Songs <corpus>/*.osr

73 exact of 145 (14 lazer), total count error 762, 4 skipped
score compared on 139, worst 55.44%, within 0.5% on 113
```

Four are skipped for want of the map. Nothing regressed against the 33 rows the
old manifest still covers — `0 worse` — so the engine did not get worse while
the instrument was away. It is simply being asked a much harder question now.

**The old claim was true of thirteen replays.** "Every replay in the corpus is
either exact or has a named reason, and the remainder lives on the hit-window
boundaries" was measured over a tenth of the corpus, and it does not survive the
other nine tenths: half of the replays are exact, and the remainder is not all
boundary noise. That sentence is gone from this document rather than softened.
It is the exact kind of statement an instrument exists to stop anybody making.

### The nine that are structural

Most of the misses are small — 113 of 139 scores are within half a percent, and
those are the boundary cases the old claim described. Nine are not, and a
divergence this size is a rule that is wrong rather than a rounding that is
close:

| combo | score | count error | client | replay |
|---:|---:|---:|---|---|
| +273 | +55.44% | 8 | stable | `avesemki … Power Stance [MAXIMUM LIMIT]` |
| −182 | −7.00% | 2 | lazer | `Guest … xi - Blue Zenith [Asphyxia's Hard]` |
| +30 | +15.62% | 4 | stable | `Saki_chan … Grayed Out [Antifront]` |
| −24 | ±0 | 42 | stable | `Sakiko_Togawa … Cellar of Ghosts [shoye…]` |
| −20 | −5.71% | 6 | stable | `Deeo_XD … Non-breath oblige [silverboxer…]` |
| +1 | −5.42% | 52 | stable | `Sakiko_Togawa … Yomi yori … [Y…]` |
| −1 | −19.89% | 2 | stable | `Uika_Misumi … Tsukiyo ni [Dai Sa…]` |
| ±0 | −7.79% | 76 | stable | `goprob … all-american bitch [daph…]` |
| ±0 | +8.05% | 2 | stable | `_legusshhka … Chi… [Imperial Circus]` |

Two shapes, and they want different work. A combo hundreds out with a small
count error means the *break* is in the wrong place, not the hits — one object
judged differently early, and every combo after it is wrong. A count error of
76 with the combo exact means the opposite: many objects graded one step off,
and the chain never broke. The first is one bug per replay and findable with
`--trace`; the second is a rule.

**Lazer is 14 of the exact 73 and 11 of the misses.** The engine judges by
stable's rules, and `stable-fidelity.md` documents where the two rulesets
genuinely differ. Whether lazer replays belong in the corpus at all is a
question to settle before spending a day on `Blue Zenith` — a −182 combo on a
lazer replay may be the engine being right about a game it is not imitating.

### Keeping it

The corpus lives outside this repository, and losing it once already cost
months of measurement. `tools/fetch-maps.py` fetches the beatmaps a replay
names, which is what the four skipped ones need; the replays themselves have no
such tool and are the thing to back up.

Running `dossier corpus --expect tools/corpus.tsv --update-expect` would record
what is on disk now as the new baseline. That is worth doing *after* the nine
above are looked at rather than before: a baseline written today would enshrine
`+55.44%` as expected, and the point of the file is to notice exactly that.

## Correctness

**The note lock on the 37% replay.** The single open question in
`stable-fidelity.md`: the lock suppresses roughly as many clicks as that player
genuinely missed, and it is unknown whether it is right or coincidentally
right. The measurement is stated there — object by object, do the refusals land
on the notes osu! scored as misses.

Still needs that replay, and it is still missing. It is identifiable by its
header rather than by its filename: **609/600/177/843 with the combo at 422**,
2229 objects, on `6e7f6f08671ad9a9d2fa079665d8d443`. Nothing in the corpus
matches; the most missed anywhere in the 149 is 397.

A *different* mashed run on the same map is present —
`Uika_Misumi … Chambarising`, 440/851/541/397 at 36.51%, combo 161 — and the
engine judges it to within a count error of 4. That is encouraging and is not
the measurement: the whole question is whether the refusals land on the notes
the game scored as misses, and only the replay the document walked object by
object can answer it against work already done. It is worth trying the
measurement on this one anyway, because a lock that is coincidentally right on
one play and coincidentally right on another is a lock that is probably
right.

**Hit-window boundaries.** What is left across the whole corpus is hits within
two milliseconds of a window edge. Not obviously fixable; worth re-measuring
once the corpus is whole, because the shape of the remainder is the clue to
whether one rule is off by a rounding or many are off by nothing.

## Features

**A skin's own screen.** *Mostly the bot's work, listed here because the part
that is hard is this repository's.* The grid shows a thumbnail — a hit circle
and a cursor, and nothing else, because at that size every extra piece made the
thumbnails look more alike rather than less. Everything else a person wants to
know about a skin belongs behind a tap on it:

- who sent it, and when;
- how much it weighs;
- its author, when the skin says who — `skin.ini` has `Author`, and most fill
  it in;
- four pictures of it with the whole interface, which is where the score face,
  the judgements, the slider and the spinner get to be seen. Those are the
  parts a thumbnail cannot carry and the parts somebody choosing between two
  similar skins is actually comparing.

The first three are already in `client/dossier/skins.py`: `stamp_of` records
the filename, the count and the owner at import, and the folder's own size is a
`stat`. The grid and the screen itself belong to the bot's mini app. The
pictures are the work — and they are the case where rendering real frames
rather than compositing elements is the right answer, since at full size a
frame shows exactly what a video will look like.


**Exhibit's remaining list.** Slow motion at the first mistake is built — the
picture, the hit sounds, the music and the camera all follow one schedule, and a
reel finds the moment itself — but it is **switched off for reels** as of the
alpha. It does not yet read as a deliberate effect, and a reel is the thing
somebody shows other people, so an effect that looks like a bug is worse there
than anywhere else in the renderer. `reel::SLOW_INTO_A_MISTAKE` is the one line
that gives it back; `--slow-at` still drives the same schedule by hand, which is
how the shape of the dip gets worked out. Two items remain from the original
list:

- *Text over the clips naming the reason.* The JSON has said why since the
  beginning; burning it into the frame was deliberately deferred until the
  selection was trusted. It now is.
- *Picking between several replays of the same map.* A different feature that
  happens to share the scorers.

**The background is off by default.** `--background` works and costs nothing per
frame; the bot does not pass it. Turning it on changes how every render looks,
which is a decision rather than a flag.

**The skin is partial by design.** The scorebar, the spinner's remaining layers
and the hit-result variants fall back to the game's own. The spinner is the
honest gap: the wiki gives neither sizes nor stacking order for the new style's
layers, and a guessed spinner looks worse than the default it replaces.

## Quality

**`classic` is not as classic as it claims.** It advertises itself as imitating
osu! and draws a flat slider body, where the game draws a gradient from a dark
rim to a lighter core — the very gradient the `1984` skin now has. A fidelity
gap in the skin that is *about* fidelity.

**Stringly-typed errors in the CLI.** Twenty `Result<_, String>` in
`dossier-cli`, while `dossier-replay` and `dossier-beatmap` have had `thiserror`
types from the start. Hygiene, not a bug.

**Edition 2021 on a 1.97 toolchain.** 2024 is available and costs almost
nothing. Now that CI builds on three platforms, an edition bump is a change
that gets checked rather than one taken on trust.

**`--strict` means two things.** Alone it is `judge`'s "fail on any mismatch";
with a number it is `corpus`'s ceiling. Documented, and still a flag whose
meaning depends on whether the next argument parses as an integer.

## Shape

**The engine has a repository — this one.** Settled 2026-08-27. It was a Rust
workspace inside a Python bot's repository, joined at one seam: the bot runs
`dossier` as a subprocess and reads the event stream. `--events` was built to
make that seam deliberate, which is what made the move a matter of an afternoon
rather than of untangling.

The Python that drives the engine came too, as `client/` — the same code the
bot and a render worker both run — so what the bot depends on is one package
from one tag rather than a folder it happens to contain. Both halves are tested
here and CI runs the engine on Linux, Windows and macOS, which is where the
differences that bite actually live.

**What is not settled is how the two stay in step.** The build stamp already
refuses a mismatch — a worker on a different build is turned away rather than
handed a render that would come back different — but refusing is all it does.
The routine around it is unwritten:

- a worker told "the builds do not agree" is told nothing about what to do,
  and the answer is `git pull && cargo build --release` in a checkout they may
  not have;
- there is no release. Somebody who does not have Rust, Python and git cannot
  run a worker at all, and installing those three is where most people stopped;
- moving the tag is a manual edit in two repositories, and nothing checks that
  the tag the bot pins and the engine the server built are the same thing.

A release — `dossier.exe` and the client as one download, built by CI from a
tag — answers the second directly and gives the first something to say. That is
the next piece of work on this side of the project, and the reason `client/` was
made importable with an entry point rather than left as a script.

## Order

The old list had three things at the top and two of them are done: the engine
has a home, and `main` is this repository's only branch. The third — the
replays — turned out to be back, which is what produced the numbers above.

1. **The nine structural divergences.** They are the corpus telling us
   something specific, which is the only kind of finding worth acting on
   immediately. Take the two shapes separately: the combo-hundreds-out ones are
   one bug each and `--trace` will find them; the high-count-error ones are a
   rule.
2. **Decide about lazer.** Eleven of the misses are lazer replays judged by
   stable's rules. Either they belong in the corpus and the difference is
   documented per replay, or they do not and the totals stop being muddied by
   them. Cheap, and it changes what every number above means.
3. **A release.** `dossier.exe` and the client, built by CI from a tag. This is
   what makes a worker something a person can run rather than something a
   person can install, and it is the difference between five friends rendering
   and one.
4. **Write the baseline.** `--update-expect` over what is on disk, once the
   nine are looked at. Then back the replays up somewhere, because losing them
   cost months the last time.
5. Then the open fidelity question, and the two Exhibit features: one is a
   question about whether the engine is right, and the other two are about what
   it shows.
