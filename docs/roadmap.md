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

Measured 2026-09-01, against build `5f7599a`:

```
dossier corpus --songs ~/.osu/Songs --expect tools/corpus.tsv <corpus>/*.osr

78 exact of 145 (14 lazer), total count error 278, 4 skipped
score compared on 139, worst 55.44%, within 0.5% on 113
```

Four are skipped for want of the map: two beatmaps out of 136 are gone from
ppy and from every mirror, which is not a thing this end can fix.

It was 762 that morning. Two causes account for the difference and both were
one bug rather than a class of them.

**lazer replays were judged at the map's stats and not the ones they were
played at.** Difficulty Adjust and the rate mods' own rate are settings stable
has no equivalent for, and they were parsed and dropped. Three replays are
played at OD 11 on maps written at 8 and below; on `down [noob...]` the great
window is 14ms rather than 32, and judging with the map's own turned 77
hundreds into threes with the combo correct to the object. 762 to 510, and two
of the three are now exact.

**A missed note took the whole Relax stream behind it.** The game presses on
every frame under Relax; this engine aims one press per note, and when the note
in front was out of reach that press was refused by the note lock and there was
no second one to spend when the lock let go. Ten refusals in a row from one
unreachable circle. 510 to 278.

Neither was found by reading. The instrument was: `--trace` showed every press
with what it was tested against — and printed `none` for every Relax replay
there is, because it walked the replay's recorded keys and a Relax replay has
none. Fixing that showed the cascade whole, on one screen, in the first look.

### What is left

Sixty replays sharing 278, the largest of them twenty, and no single cause among
them. That is the shape the old claim described and could not support at the
time: what remains looks like hit-window edges rather than a rule that is wrong.
Worth re-measuring the *shape* of it — how much sits within two milliseconds of
a boundary — before spending a day on any one replay.

Whether lazer replays belong in the corpus at all is settled by the above and
needs no decision: they were disagreeing because the engine was reading them
wrongly, not because it judges a game it does not imitate.

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


**The storyboard and the video are in, and three things about them are
not.** Both are behind flags — `--storyboard` and `--video` — and off by
default, like the artwork.

- *Triggers* (`T,HitSoundClap,…`) are parsed far enough to be skipped whole.
  They fire on things the storyboard cannot know by itself, and a trigger
  expanded on a guess is a sprite that appears when nothing happened. Doing
  them properly means handing the storyboard the hit sounds as they are
  played, which is a thing the renderer does not have and the audio side does.
- *`--video` is `video` only.* `frame` would need a seek per picture and
  `exhibit` a seek per clip; neither is hard and neither is written.
- *A tinted sprite allocates.* tiny-skia carries an opacity through a blit but
  not a colour, so a sprite under a `C` command is multiplied into a scratch
  copy first. White sprites — nearly all of them — cost nothing. A storyboard
  that tints hundreds at once would want a cache keyed by picture and colour.

**Cursor rotation should be a setting.** It follows the game now — a full turn
every ten seconds, off when the skin says `CursorRotate: 0` — and asked for as
something a person can turn off for themselves rather than only the skin.
Wanted 2026-08-28.

**Measuring against danser: deferred, and the reason is the cost.** danser is
the reference this engine was written against and has never been run on the
corpus — only quoted. The harness for it exists and compiles: `tools/danser-judge`
drives danser's own ruleset the way its replay controller does. It does not run
on macOS, because the hit objects load skin textures and a font in
`SetDifficulty` before anything is judged and the atlas wants a GL context.

Three routes were priced on 2026-09-01 and all were declined for now:

- **patch the two resource loaders and run natively** — the smallest change, and
  the rules stay untouched, but the attempt hit four blockers in a row and each
  one was only visible after the previous was cleared. A long tail.
- **a Linux VM locally** — danser unmodified, data stays put, over a gigabyte
  of machinery for one number.
- **a Linux CI runner** — free and quick, and it would send other people's
  replays to a third party. They were given for finding judging errors here.

What is taken instead costs nothing and is better evidence. **A lazer replay
carries a count for every judgement type**, where a stable `.osr` carries four
totals; that is the finer instrument, and the corpus's fourteen lazer replays
all sit at a count error of 2 or less. More of them is a question of asking,
and thirty-one replays a week already arrive on their own. The shapes worth
asking for are the ones the residual lives on: streams with a miss in the
middle, and dense patterns at high OD.

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
3. **Tell a mismatched worker where to get the right build.** The release
   exists; the message that turns a worker away still ends at "git pull". The
   bot knows its own stamp and the release that carries it, and saying so turns
   a dead end into a link.
4. **Write the baseline.** `--update-expect` over what is on disk, once the
   nine are looked at. Then back the replays up somewhere, because losing them
   cost months the last time.
5. Then the open fidelity question, and the two Exhibit features: one is a
   question about whether the engine is right, and the other two are about what
   it shows.
