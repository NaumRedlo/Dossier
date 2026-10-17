# osu!mania simulation foundation

The first increment is `dossier_sim::mania::Playback`. It accepts a native mania
beatmap and a supported mania replay, and returns a deterministic snapshot for a
beatmap timestamp. It is not yet routed into video export or the desktop player.
Existing standard simulation remains separate.

## Implemented

- `.osu` hold notes retain their type, end time and correctly aligned hit samples.
  Invalid/reversed hold times are rejected, rather than silently becoming taps.
- `Timeline` maps X into 1–18 columns, preserves original object indices, sorts
  notes within each column and handles simultaneous chords.
- `InputTrack` reads column masks from replay X, retains press/release edges
  (including changes at the same timestamp), and supports stateless backward seek.
  The standard `Keys` byte is deliberately not used for mania columns.
- `InputTrack::from_replay` recognizes stable's leading `(256, -500)` skip
  markers and redundant backwards zero-key frames before the first press. It
  preserves gameplay timestamps, original error indices and replay evidence.
  `InputTrack::new` remains strict; malformed gameplay rollbacks are rejected.
- `visible_at` returns head/tail distances from the judgement line, normalized
  by an explicit approach duration. This is constant-speed geometry, not yet SV.
  Per-lane start times and prefix end maxima bound the search; a long hold cannot
  hide behind an already expired tap. Pathological overlapping holds may still
  require scanning the overlapping portion of a lane.
- `Playback::at` combines visible notes and held columns. Holding a key is input
  state, not evidence that a note was successfully judged.
- Unsupported modes, mods, malformed frames and fractional key counts return
  errors. This phase does not attempt standard-to-mania conversion or key mods.
- Lazer DT (including a configured 1.01–2.0 rate) and AC are accepted for note
  judgement. Native stable NoMod, HR, DT and HRDT use the separate ScoreV1
  policy. Other modifiers and unsupported DT settings are rejected.

All timestamps are **beatmap milliseconds**. Playback speed belongs at the clock
boundary. A renderer must not apply DT/HT to notes and replay timestamps twice.
Hash matching remains the caller's responsibility: the parsed beatmap has no MD5.
The existing standard `Timeline` explicitly rejects mania hold objects.

## Experimental lazer judgement

`playback.judge_with(playback.lazer_rules(od)?)` opts into the native-map
lazer rules described by upstream revision
`bdfabf93c77483bb471dfde8c900fa9176a9d531`. This is a selected rules hypothesis,
not automatic compatibility with every lazer replay version. Stable uses the
separate ScoreV1 policy below; it is never routed to this judge by the audit.

The event stream contains tap, hold-head, hold-tail and one hold-break event per
hold. Head and tail results use six mania grades. Body breaks are counted
separately and do not add a seventh scored note. Tail release lenience is 1.5;
regrabbing after a missed head or broken body caps successful tail results at Meh.
Holding through the tail indefinitely is a miss, not an automatic Perfect.
Column note lock switches eligibility at the next head's start. One press cannot
judge two heads. Duplicate same-column taps remain distinct: early separate
presses can hit both; at the shared start only the last is hittable, and earlier
duplicates expire normally. Overlaps involving holds are explicitly unsupported.

`Playback::clock_rate()` exposes the replay speed to the future audio/playback
clock. DT multiplies unrounded windows before flooring; neither note nor replay
timestamps are rescaled. For example, OD 5 DT Perfect is `floor(19.4*1.5)+0.5`,
or 29.5 ms in beatmap time. Supplying NoMod windows to DT `judge_with()` returns
`MismatchedClockRate`, preventing an accidental second/missing speed adjustment.

AC affects failure conditions, not note grades. `has_accuracy_challenge()` exposes
its presence, but HP, failure/restart, and partial-play completion remain outside
this simulation. Accepting an AC replay does not claim that its failure state was
reproduced. The corpus comparison still uses complete-map counts.

`counts_at(t)` queries cumulative results without future information and supports
backward seeking. `final_counts()` expires the entire map even after replay EOF;
therefore it must not be presented as a partial-play result. Deadlines currently
use the first representable instant after the successful window, rather than
reconstructing lazer's update-frame scheduler. This is one remaining fidelity
limitation. Lazer combo, accuracy and unmodified numeric score are available
through the event stream as described below. HP and failure stopping are not.

Windows preserve lazer's float difficulty and interpolation order before floor;
an algebraically simplified OD 9 formula incorrectly loses one millisecond.
Boundary tests cover this alongside missed heads, repeated key states, chords,
early release, regrab, late release and equal-time release/press pairs.
Re-grabbing is allowed before the head's scheduled start after an early hit or
miss has already judged it. Fractional note-lock boundaries remain unrounded.

Corpus audit (read-only; hashes actual map bytes):

```sh
cargo run --locked -p dossier-cli --example mania-corpus -- "/path/to/Dossier Corpus" > /tmp/mania-corpus.tsv
```

The report includes replay/map hashes, client version, mods, unsupported cases,
expected six header counts and observed counts. A counts-match status is not a
claim of matching combo, numeric score, HP, individual object results or visuals.
No expectations are silently rewritten.

`--audit-rounding` additionally searches a bounded set of hypothetical sub-ms
frame shifts through the full judge. It requires simultaneous agreement with
recorded counts, maximum combo and unmodified score, and records the offsets in
a separate report. All 41 differing lazer replays currently have such a witness;
112 match without changes. Witnesses preserve frame order and export back to the
same timestamps, but are selected using aggregate targets: they are **not**
recovered original input, corrected scores, or independent expected fixtures.
Ordinary judging and its measured report remain unchanged. See the corpus status
document for scope, limits and report fields.

Use `--trace <replay-file-md5>` after the corpus directory to emit input edges and
per-object results instead of aggregate rows. The audit also checks whether
aggregate differences are compatible with independent half-millisecond release
uncertainty: lazer's legacy export rounds frame times, while tail windows can
end at quarter milliseconds. This diagnostic does not modify judgement results
and cannot prove exactness. See the corpus status document for measurements and
the separate note-lock boundary candidate.

`tail_frame_rounding` refines that diagnostic by requiring a common displacement
for eligible tail releases from the same input frame. It reports `compatible`,
`incompatible`, or an explicit indeterminate reason for ambiguous release
provenance/resource limits. It does not merge distinct frames sharing a rounded
timestamp. Object assignment, broken holds, inter-frame ordering and client
updates are outside this model, so compatibility remains evidence of a possible
explanation, not a corrected or recovered score. Stable rows leave this field
empty. Neither diagnostic affects simulation events or observed counts.

For a controlled fractional-time experiment, `Judge::with_frame_offsets` takes
the original frames, one explicit offset per frame, and the selected lazer
rules. It uses the same lane judge as ordinary playback. Every column change in
a frame shares its offset; offsets must be finite and within ±0.5 ms, and all
frames must stay ordered, including frames with unchanged masks. Invalid masks,
backwards source input, unsupported overlaps, mismatched offset counts and
timestamps too large for quarter-ms probes are rejected. Source data is borrowed
and never edited. This low-level diagnostic does not select replay mod policies;
the caller must use the correct lazer rules as with `Judge::new`.

The corpus tool exposes this only inside lazer trace mode as
`--probe-frame <zero-based-original-index> <offset-ms>`. It writes hypothetical
counts and event differences to stderr, while stdout remains the original
trace. No automatic counter matching or replay correction is performed. A zero
offset preserves the ordinary event stream, and a nonzero probe is never counted
as a corpus accuracy improvement. See the status report for the reproducible
0.25 ms note-lock case.

### Combo

`Judge::combo_at(time_ms)` returns `Some(Combo { current, maximum })` for lazer.
`maximum` is the achieved streak up to that time, not the map's possible full
combo. Successful taps and hold heads/tails increment the streak; Miss and
HoldBreak reset it. A re-grab adds no combo, and a body break does not add a
basic miss to the six counts. This follows the pinned official ScoreProcessor,
HitResult and HoldNoteBodyJudgement rules linked in the corpus report.

Snapshots share the judgement stream's ordering and support stateless backward
seeking, without leaking a later maximum. Queries before the first event or at
NaN return the zero snapshot; positive infinity includes every event.
`final_combo()` includes automatic events after replay EOF, just like
`final_counts()`. The ordinary and fractional-frame probe paths share this
calculation. Stable maps also expose this seekable combo: every successful
grade adds one and Miss resets the streak. Empty maps return a zero snapshot.
Stable holds add one combo per 100 ms held, phased from each grab or re-grab in
map time, plus one for a successful combined Hold result. A tick at the release
instant counts and precedes results at that instant. The head adds nothing;
releasing the body, releasing before the head, or a missed or forced head resets
the streak. Held intervals stop at the note tail even when a key stays pressed.
A delayed old-tail expiry cannot stop a newer hold in the same lane.
Intervals and checkpoints avoid allocating individual ticks; storage depends on
objects and grabs, not hold duration. Unrepresentable 32-bit totals return `None`.
The tick phase and input-boundary ordering are experimental, not reconstructed
client update times. Stable numeric score remains unavailable, including on tap-only maps.

Maximum combo matches all 153 lazer replays and 43 of 51 stable replays: all 18
tap-only maps and 25 of 33 maps with holds. The other eight stable values are
1–4 higher than recorded; seven of them have matching counts. Two alternatives
were measured and rejected: excluding a tick at the release instant matches 40,
and phasing ticks from the note's start matches 23. This is independent of the
six-count comparison and does not prove per-frame ordering, numeric score or HP
fidelity. `expected_max_combo`, `observed_max_combo` and
`max_combo_error` in the corpus report keep that distinction visible.

### Lazer accuracy and score

`Judge::score_at(time_ms)` and `final_score()` return `Some(LazerScore)` for lazer,
with accuracy as a fraction and the normalised score before mod multipliers.
Stable returns `None`. Before the first event (also NaN/empty input), the snapshot
is accuracy 1 and score 0. Backward seeks reuse prefixes without future results;
the final snapshot includes automatic results after EOF, not failure stopping.

Accuracy uses weights 305, 300, 200, 100, 50, 0 for Perfect through Miss. Hold heads
and tails count separately; a body break affects combo but not accuracy or
immediate points. Combo-weighted points use 300 for Perfect, a clamped base-4
logarithm of combo after judgement, and the all-Perfect denominator for the whole
map. The final score combines 150,000 times combo progress with 850,000 times
`accuracy^(2 + 2*accuracy)` times judged-map progress, then rounds ties to even.
The source revision is the same as the judgement policy; references are linked
in the corpus report. Numerical/event-order parity is still experimental.

All 153 lazer files provide an independent unmodified-score reference: 112
match exactly, 41 differ. All count-matching replays now also match the score.
Equal-time events retain their original input-edge order across columns,
including forced results triggered by that edge. Sorting by column had changed
combo weights and caused six 1–3 point score errors. Automatic deadlines still
use analytical timing, not reconstructed client updates. Accuracy matches
header-derived values in 112 cases; no separately
recorded accuracy exists in these files. Mod multipliers, HP/failure policy,
legacy stable scoring and older lazer score policies are not inferred.

## Experimental stable ScoreV1

`playback.judge_stable_with(playback.stable_rules(od)?)` selects a separate
native ScoreV1 policy from legacy replay mods (NoMod, HR, DT or HRDT).
It emits the same seekable `Judge` event/count interface,
but starts with a constant Perfect threshold of 16 ms and OD-based thresholds
for other grades, with rounded hit error. Tap expiry excludes the last integer
millisecond of OK; hold heads retain that millisecond. Early Meh is still possible. Fractional ties currently use .NET-style
ties-to-even. Expiry is analytical, not stable's update scheduler.

Raw thresholds are multiplied by `clock_rate / difficulty_multiplier` before
flooring, following the pinned official Classic ScoreV1 windows. HR divides by
1.4; DT multiplies by 1.5. All errors, windows and expiry times remain in beatmap
milliseconds; timestamps and columns are not transformed. For example, native
OD8 HRDT Perfect is 17 ms, Great is 42 ms and Good is 78 ms. A supplied rule with
a mismatched rate or HR setting is rejected by both normal and trace playback.
`StableRules::native_no_mod(od)` is retained for explicit NoMod timelines;
`native_with_mods(od, rate, hard_rock)` supports only stable rates 1 and 1.5.

Holds emit **one `Part::Hold` result**, at release or timeout. The head does not
increment the six counts. The timing classifier uses both the head constraint
and the combined absolute head/release error budget. `hold_result_for` only
classifies timing: body breaks, missing heads, note lock and expiry belong to
the lane state machine. A body break or recovered missed head caps a normal
release at Meh; overholding has a separate rule. The event offset is the signed tail release offset,
not the sum of both errors. Hold combo ticks are described under Combo; numeric
score and HP are not implemented, and no stable hold-break events are exposed yet.

The initial doubled tail timeout has been removed. A hold still grabbed after
the late OK boundary finishes automatically using the stored head error and a
Meh-sized substituted release error. This yields Good, Ok or Meh, rather than
Miss, even if the player never releases. An ungrabbed hold still expires as Miss.
The automatic event has no physical release offset; a later release cannot
score the same hold again. The independent parity implementation and tests linked
in the corpus report support this behaviour. Sub-millisecond scheduling is still
analytical and is not claimed identical to stable's update loop.

All 50 native NoMod stable replays are measurable: 48 have matching counts,
including all 18 tap-only replays and 30 of 32 maps containing holds. The remaining
two cases include early release/re-grab candidates before a hold's scheduled
start; the current body-break cap is not claimed exact there. See the corpus
report for hashes and residual counts. No expected replay count drives a grade.
The remaining HRDT replay also matches all six counts, bringing stable coverage
to 51 measured and 49 matching. Duplicate taps, overlapping holds, converts,
other stable modifiers and ScoreV2 remain unsupported. Lazer DT/AC playback
cannot accidentally enter this stable policy; stable HR cannot enter lazer rules.

The lane scheduler uses indexed pending heads/tails and one grabbed hold per
column; it does not scan every future hold for each key edge. Subsequent notes
can be hit at an old tail's timestamp or early after its body ends without the
old recoverable hold stealing the press. Input at equal timestamps retains order,
and final results/count prefixes remain seekable backwards.

The corpus audit selects stable/lazer explicitly from the replay client version,
records the attempted `policy`, and supports `--trace` for either. It leaves the
lazer-specific tail-rounding diagnostic empty for stable replays.

`playback.trace_stable_with(rules)` returns the same `Judge` plus an optional
diagnostic transition stream. `StableRules::judge_with_trace` exposes the same
operation for a timeline/input pair. Each transition carries the original object
index, column, timestamp, accepted head offset and broken flag. Re-grabs do not
replace the accepted head offset. Transitions are sorted by time and column;
equal-time operations in one column retain their order. They are simulator
decisions, not recovered original-client judgements, and never add to scored
counts. The normal judging path does not allocate this log.

The audit's trace TSV adds `transition`, `head_offset_ms` and `broken` columns.
Rows with `kind=state` use these fields; input/result rows leave them empty.
Input, result and state sections are separate, not a merged chronological stream.
Lazer currently emits no state rows. This makes pre-head release/re-grab cases
inspectable without altering their grades to fit replay headers.

`--audit-stable` runs those transitions across the corpus and compares a separate
NoMod ScoreV1 formula model against recorded header scores. It appends ten
columns to the ordinary report without changing its original fields or events.
The model currently matches 36/50 NoMod scores; 12 of the remaining 14 already
have matching count vectors. The experimental stable merge preserves original
replay-frame order at equal timestamps and orders results within one frame by
original object index, rather than column. Forced misses retain their triggering
frame; analytical deadlines sort before input at the same instant. This resolves
four recorded score comparisons without changing grades. It is corpus evidence
for the merge policy, not proof of stable's complete update-frame scheduling.
HRDT scoring is not inferred from the NoMod formula.
This audit is not exposed through the engine's stable score queries: update
order, body-break effects and numeric fidelity need more evidence first.
The same report lists pre-head release counts and original object IDs to guide
reference-client checks. The corpus status document records the limitations.

Four of those columns bound the score over all equal-time grade permutations.
The calculation retains per-bonus extrema and preserves distinct-time order;
it relaxes original frame ordering and never feeds the recorded score into the
model. All 14 residual NoMod scores lie outside these bounds. Being inside the
interval would not prove reachability. Searches exceeding 2,000,000 transitions
or 50,000 states per layer return unavailable bounds (`indeterminate-limit`).
The [reference checklist](mania-reference-checks.md) identifies the stable
observations needed before altering remaining grade/score/combo rules.

## Rendering

`dossier_render::mania::Mania` draws a replay from `Playback` and its `Judge`:
lanes over the map's dimmed background, notes and hold bodies at a constant
scroll, a hold's body stopping at the line while it is held, pressed columns,
a flash at the receptor on each hit, the latest grade, combo, accuracy and the
six counts. Stable grades are named MAX/300/200/100/50/MISS, lazer ones by their
lazer names; stable accuracy comes from the counts, lazer accuracy from the
judge's score. It scales from a 1080p reference to any frame size.

`dossier_produce::render::render_mania` picks stable or lazer rules by the
replay's client version and encodes through the same `video::encode` as
osu!standard (any `dossier_render::Picture` can be encoded). `dossier video`
and the application's render path (and so the render farm's workers) send
osu!mania replays there. The application's own replay list still shows
osu!standard only. Not drawn yet: skins, hitsounds, scroll speed changes (SV),
HP and failure, storyboards and map videos. Replays the simulation does not
support are refused with the reason rather than drawn wrongly.

## Next increments, in order

1. **Reference fixtures.** Pin local 4K and 7K maps/replays with taps, chords,
   early/late releases, missed heads, regrabs, jacks and overlapping long notes.
   Keep stable, lazer and lazer Classic cases separate. Store expected results,
   original object ids and timestamps. Do not download raw maps from production
   osu! endpoints; use user-provided files or the existing mirror/archive path.
2. **Rules and judgement.** Extend the experimental versioned per-column judge. Consume key edges once, expire missed heads deterministically,
   and model hold head/body/tail separately. Verify boundaries, release leniency,
   regrabs and note lock against the pinned client source/fixtures before exposing
   counts. Avoid using standard's 300/100/50 judge or hold-as-slider substitutions.
3. **Score and health.** Track score alongside remaining grade differences; add mod
   multipliers, HP and stable hold score. Stable ScoreV1, ScoreV2 and lazer need distinct score
   policies. Compare final results to replay headers; the headers must not drive
   the simulated result. Do not claim exactness before corpus agreement.
4. **Scroll and mods.** Integrate timing/SV into a scroll-distance track, then add
   fixed/BPM-dependent scroll, remaining rate mods, Mirror and visibility mods.
   Key conversion and Random require their own deterministic transforms.
5. **Renderer.** Add a mania playfield scene consuming snapshots: lanes, receptors,
   tap and hold geometry, pressed-key highlights and judgement effects. Separate
   layout/skin configuration from simulation. Add render golden tests for long
   holds crossing the viewport and bounds at different resolutions.
6. **Production integration.** Route by ruleset before constructing GameState,
   select mania audio/hit samples and HUD, then enable preview/export only for the
   validated rules/mod combinations. Keep seeking independent of frame rate.

## Sources and verification

- [Official ManiaReplayFrame legacy conversion](https://github.com/ppy/osu/blob/master/osu.Game.Rulesets.Mania/Replays/ManiaReplayFrame.cs): column bits are stored in X.
- [Official .osu format](https://osu.ppy.sh/wiki/en/Client/File_formats/osu_(file_format)): hold end time precedes hitSample.
- [Official mania judgement documentation](https://github.com/ppy/osu-wiki/blob/master/wiki/Gameplay/Judgement/osu%21mania/en.md): source for client-specific timing constraints; exact replay fidelity still requires reference verification.

Run `cargo test -p dossier-beatmap -p dossier-sim --locked` and
`cargo check -p dossier-produce --locked` for this increment. Synthetic tests cover
geometry, input, parsing and judgement boundaries. Corpus measurements are recorded
in `docs/mania-corpus-status.md`; exact replay fidelity remains unverified.

Reference for this judgement increment:
- [ManiaHitWindows](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game.Rulesets.Mania/Scoring/ManiaHitWindows.cs)
- [Difficulty interpolation](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Beatmaps/IBeatmapDifficultyInfo.cs)
- [Hold tail](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game.Rulesets.Mania/Objects/Drawables/DrawableHoldNoteTail.cs)
- [Ordered hit policy](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game.Rulesets.Mania/UI/OrderedHitPolicy.cs)
- [Hold re-grab handling](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game.Rulesets.Mania/Objects/Drawables/DrawableHoldNote.cs)
- [Legacy replay timestamp rounding](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Scoring/Legacy/LegacyScoreEncoder.cs)
- [Mania rate-adjusted windows](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game.Rulesets.Mania/Mods/IManiaRateAdjustmentMod.cs)
- [DT settings](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Rulesets/Mods/ModDoubleTime.cs)
- [AC failure condition](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Rulesets/Mods/ModAccuracyChallenge.cs)
- [Stable mania judgement rules](https://github.com/ppy/osu-wiki/blob/master/wiki/Gameplay/Judgement/osu%21mania/en.md)
- [Stable prefix handling in the official legacy decoder](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Scoring/Legacy/LegacyScoreDecoder.cs)
