# Mania corpus status — 2026-10-16

This audits six judgement counts, maximum combo for lazer and stable maps,
and lazer unmodified score;
it is not a claim of complete gameplay or rendering fidelity.
The expected counts are read from replay headers. Observations are produced by
explicit experimental policies: native lazer (NoMod, DT, AC) and native stable
ScoreV1 taps and holds (NoMod, HR, DT, HRDT). Successful totals
do not prove per-object, HP or visual equivalence; combo and score are measured separately below.

## Corpus and map recovery

- 204 mania replays, 203 distinct map hashes.
- All 203 exact `.osu` revisions recovered from mirror archives and verified by
  MD5 against the replay headers. Saved locally under
  `Documents/Dossier Corpus/Mania Beatmaps`. No production raw-map endpoint used.
- 153 lazer replays and 51 stable replays. Maps/replays are not bundled in Git.
- Per-replay audit: [`tools/mania-corpus.tsv`](../tools/mania-corpus.tsv).
  Hashes identify the inputs; filenames are labels, not identity.

## Current results

| Outcome | Replays |
| --- | ---: |
| All six counts match | 161 |
| Counts differ | 43 |
| Unsupported stable modifiers in this corpus | 0 |
| Unsupported lazer mods in this corpus | 0 |
| Unsupported overlapping notes in this corpus | 0 |
| Missing or wrong map revision | 0 |

204 measured replays contain 267,642 expected scored judgements. The sum of
absolute differences across the six count buckets is 114, with a maximum of 6
on one replay. This is **not** a count of wrong objects: moving one result from
Perfect to Great adds two to this metric; totals can also conceal compensating
errors on different objects. No per-object accuracy percentage is claimed.

| Sum of absolute bucket differences | Replays |
| --- | ---: |
| 0 | 161 |
| 2 | 31 |
| 4 | 10 |
| 6 | 2 |

| Policy | Measured | Matching counts | Different counts | Bucket error sum |
| --- | ---: | ---: | ---: | ---: |
| Lazer native | 153 | 112 | 41 | 110 |
| Stable ScoreV1 | 51 | 49 | 2 | 4 |

## Lazer combo increment

The judge now exposes seekable current and maximum-achieved combo for its lazer
policy. All **153 of 153 measured lazer replay maxima match the replay headers**,
including the 41 with residual judgement-count differences. No replay header
value participates in calculating combo. The report appends
`expected_max_combo`, `observed_max_combo`, and `max_combo_error`; existing count
statuses and timing diagnostics are unchanged.

Successful taps, hold heads and hold tails add one; Miss and HoldBreak reset the
current streak, retaining its achieved maximum. HoldBreak does not add a basic
miss. These rules follow the pinned official
[ScoreProcessor](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Rulesets/Scoring/ScoreProcessor.cs),
[HitResult extensions](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Rulesets/Scoring/HitResult.cs),
and [HoldNoteBodyJudgement](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game.Rulesets.Mania/Judgements/HoldNoteBodyJudgement.cs).

Prefix snapshots prevent future maxima from appearing during backward seeks.
Tests cover chord heads/tails, re-grabs, body breaks without counted misses,
automatic misses after replay EOF, and empty/invalid-time queries. The final
snapshot includes post-EOF automatic judgements; use a timestamp query for a
partial play. Equal-time input results retain replay input order across columns;
automatic judgements still use analytical deadlines, so matching final maxima
does not establish frame-by-frame parity.

Stable tap-only combo is now available: all **18 of 18** recorded maxima match.
Every successful tap grade increments combo, while Miss resets it. Empty maps
return zero; queries support backward seeking and automatic misses after EOF.
The 33 stable maps with holds use an analytical tick clock: one combo per
100 ms held from each grab, plus the combined Hold result. **25 of 33** match;
eight are higher than recorded (`5dc9d270` +4 with differing counts, `63e1605f`
+2, and `b51f3ac5`, `4a891973`, `cc980bbd`, `d3b6e5d3`, `b45dd299`, `738c4fd7`
+1 each). No value is lower. Excluding a tick at the release instant matches 40
of 51 stable maps, and phasing ticks from the note start matches 23; both were
rejected. Client update scheduling is one possible explanation of the remaining
1–4 tick differences, not a verified cause. No header-derived correction is used.

The [official migration guide](https://osu.ppy.sh/wiki/en/Help_centre/Upgrading_to_lazer#hold-note-ticks-are-removed)
documents stable's 100 ms combo ticks. The grab-relative phase, map-time clock and
boundary ordering above remain experimental; that guide does not specify them.
The model clips each held interval at the note tail, restarts its phase on
re-grab, and resets combo on a body drop or missed head. An expired old tail must
not close a newer hold in the same lane. Ticks do not create extra judgements.

Intervals and reset/result checkpoints support backward seeking without storing
one event per tick. Memory grows with input transitions and objects, not hold
duration. A synthetic billion-tick hold verifies this path; totals beyond the
32-bit Combo range return unavailable instead of wrapping. The implementation's
range guard does not infer how stable handles legacy header overflow.
The stored expected value is the legacy header's 16-bit maximum, as written by
the [official legacy encoder](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Scoring/Legacy/LegacyScoreEncoder.cs).
This corpus does not approach that format limit; general overflow handling is
not inferred from these fixtures.

## Lazer accuracy and numeric score increment

All 153 lazer replay extensions contain `total_score_without_mods`, an independent
recorded score. None contains an explicit accuracy value. Reference accuracy is
therefore **derived from the six header counts**, not an independently recorded
percentage. The simulation computes both values from its own event stream.

| Lazer metric | Match | Differ |
| --- | ---: | ---: |
| Maximum combo | 153 | 0 |
| Accuracy derived from header counts | 112 | 41 |
| Recorded score without mod multipliers | 112 | 41 |

The largest absolute score difference is 1,805 points. All 112 replays with
matching judgement counts now also match the recorded score exactly. Preserving
input order at equal timestamps resolved these six former 1–3 point differences:

| Replay MD5 | Recorded score | Before | After |
| --- | ---: | ---: | ---: |
| `a3c8c8832deddb44a0cfe85d95176de2` | 909202 | 909201 | 909202 |
| `f2180ba9bc19e2ac3c94b5bdc8524095` | 858006 | 858005 | 858006 |
| `c85bf6433828bae7fe747ad5438199fa` | 870091 | 870090 | 870091 |
| `33f7a7bf96c39604e6749c4e8d60475a` | 789702 | 789705 | 789702 |
| `9cab8dc6793dc8625729d624c6325692` | 771319 | 771320 | 771319 |
| `70e050ed10fec9d59d8cdc0b19bd5e90` | 818056 | 818059 | 818056 |

The previous lane merge sorted equal-time events by column, losing the order of
distinct replay frames exported to the same millisecond. Each input edge now
carries its original sequence through lane judging. Forced misses and body
breaks stay with their triggering edge, including results without a hit offset.
Automatic deadlines remain separate and sort before input at exactly the same
instant. The [official framed replay handler](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Rulesets/Replays/FramedReplayInputHandler.cs)
retains equal-time frame order and advances frames individually. No rounding
bias or replay-specific correction was added. Stable judging is unchanged.

Three new regressions cover cross-column grade ordering, a press followed by an
equal-time hold break, and forced misses before the next equal-time frame.
Zero-offset probes share ordinary input ordering; separating two frames without
crossing grade boundaries preserves their score. All corpus count, combo and
rounding-diagnostic fields are unchanged. Nine observed scores change: six
become exact; three already differing grade vectors remain differing.

The policy follows pinned official
[ManiaScoreProcessor](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game.Rulesets.Mania/Scoring/ManiaScoreProcessor.cs)
and [ScoreProcessor](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Rulesets/Scoring/ScoreProcessor.cs):
accuracy weights are 305/300/200/100/50/0; the combo portion uses 300 for Perfect,
a clamped base-4 logarithm of achieved combo, and an all-Perfect map denominator.
The 150,000-point combo portion and 850,000-point accuracy/progress portion are
combined before ties-to-even rounding. HoldBreak adds neither weighted accuracy
nor immediate points. Failure stopping, HP and mod multipliers remain outside
the model; stable score/accuracy queries remain unavailable.

The report appends `accuracy_from_header_counts` and `observed_accuracy` as
fractions, plus `expected_score_without_mods`, `observed_score_without_mods` and
absolute `score_error`. A missing score stays blank. The replay parser now
preserves a legitimate recorded zero while rejecting negative/invalid values.
All prior count, combo and rounding-diagnostic fields are unchanged.

## Stable tap increment

The [official judgement documentation](https://github.com/ppy/osu-wiki/blob/master/wiki/Gameplay/Judgement/osu%21mania/en.md)
distinguishes stable ScoreV1 from lazer: constant Perfect window, integer error
rounding/truncated thresholds, expiry after OK (no late Meh), and one combined
judgement for a hold. The tap and combined hold policies are implemented separately from lazer.
Converts, ScoreV2, other stable modifiers and duplicate same-column taps are rejected
explicitly rather than assigned lazer grades.

Stable input has two special cases confirmed by corpus traces:

- Leading `(256, -500)` frames are skip markers, not mania key masks. Only
  matching markers in the first two original positions are removed, as in the
  [official legacy decoder](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Scoring/Legacy/LegacyScoreDecoder.cs).
- Three replays also contain backwards idle timestamps before their first key
  press. Only redundant zero-key pre-roll frames are dropped. Gameplay changes,
  later rollbacks and invalid masks are still rejected; original replay bytes,
  timestamps and diagnostic frame indices are retained.

All 18 native NoMod stable tap-only replays match all six counts (17,528 notes).
The last difference, `bilbordy - ischezayu [Insane]`, replay file hash
`a3c32ee877be3a1ea916bb375d266076`, is resolved by excluding the final integer
millisecond of the tap OK window. At OD8.5, object 662 (51,891 ms) expires before
the press at 51,992 ms; this press can now hit object 665 (52,041 ms) at -49 ms.
Previously these became two Ok results; now they correctly contribute Miss and
Good. The full observed vector matches `[996, 589, 169, 28, 6, 35]`.
Hold heads keep their distinct final-millisecond boundary. Fractional scheduling
and per-object reference verification are still needed for exactness claims.

The report's `policy` column states which experimental rules were attempted.
Tail-rounding diagnostics remain blank for stable; the lazer explanation must
not be applied to stable mismatches. All 153 previous lazer observations
remain unchanged. Tests cover stable timing/expiry asymmetry, prefix markers,
idle pre-roll, real ninth-column input, retained malformed-frame indices, and
rejection of unsupported overlap/mod cases.

## Stable HR/DT increment

The last unsupported replay now runs with native stable ScoreV1 HRDT windows.
`satori de pon! [4K NM]`, replay hash `e1f355d28e678c44a427cd3d60e8c258`,
map hash `8310a14b4a0bee68e8d2738f4084a3ad`, matches all six recorded counts:
`[471, 289, 53, 6, 2, 3]` (824 objects, including 25 holds).
All 203 previously measured rows are unchanged. This corpus contains no other
stable HR/DT examples; isolated HR and DT are covered by synthetic boundary
tests, not additional recorded plays.

The window policy follows the native Classic ScoreV1 branch of the pinned
[official ManiaHitWindows](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game.Rulesets.Mania/Scoring/ManiaHitWindows.cs).
Scale the raw threshold by `clock_rate / difficulty_multiplier` before flooring;
HR uses 1.4, DT uses rate 1.5. At OD8, HRDT integer thresholds are
`[17, 42, 78, 110, 136, 175]` beatmap milliseconds. Hit errors retain the existing
stable rounding policy. Rounding before rate scaling changes boundary grades.
Geometry, original input timestamps and columns are unchanged; HR does not
mirror mania columns. Hold budgets, body lenience and analytical expiry use
the resulting map-time windows once.

`Playback::stable_rules(od)` selects this policy from legacy modifier bits.
The normal and trace entry points reject a different rate or HR setting.
Lazer HR, stable custom rates, other modifier bits and ScoreV2 remain unsupported.
No replay counters or map hashes are consulted by the judge. Stable numeric
score remains unimplemented, and the two NoMod hold residuals below
remain unresolved.

## Stable hold increment

The next 32 NoMod stable replays contain 40,433 expected judgements (taps and
holds). All are now measured; 30 match all six counts, two differ with bucket
errors 2 and 2.
The policy label is now `stable-scorev1-experimental` (formerly tap-only).

The hold state machine records the head without counting it, combines its error
with the release for one `Hold` event, handles broken/missed-head recovery and
preserves the next note's early press after an old hold body has ended. Synthetic
regressions cover combined/head-only constraints, release/chord counting, seeking,
body breaks, late releases, EOF, overlaps, and adjacent note selection.

The first implementation's doubled release horizon was incorrect. Stable
ScoreV1 has a distinct **overhold** outcome: if the key is still grabbed at the
late OK boundary, the result uses the head error with a substituted Meh-sized
release error. It finishes automatically as Good/Ok/Meh; an ungrabbed note misses.
A later physical release cannot change that result. This also applies after a
re-grab; a normal release on a broken hold remains capped at Meh.

This corrects `Chou Night of Knights`, replay
`3edcbdfd1f35a82c0e746b763e213860`: holds 835 and 838 have head offsets -25 and
-33 ms, and releases +104 ms after their tails. They now finish automatically as
Ok rather than Good, matching all buckets `[906, 505, 118, 13, 2, 7]`.
The other 201 measured replay arrays are unchanged, including all 153 lazer
replays. Overheld notes now carry automatic timestamps and no physical release
offset, even where their final grades are unchanged.

Evidence is independent of our replay totals:

- [First-hand stable overhold report and demonstration](https://osu.ppy.sh/community/forums/topics/466617?start=5349997) describes 200/100 outcomes instead of a missed release.
- [Interlude stable parity tests](https://github.com/YAVSRG/YAVSRG/blob/f83a3a309e0d95d8a014d7a8a933ab289bc60e00/prelude/tests/Gameplay/Scoring/RulesetOsuParityTests.fs) document tap +102/+103 at OD8 and hold/overhold boundaries. This is an independent implementation, **not official stable source**; its ignored edge-case tests are not treated as passing evidence.
- [Its hold classifier](https://github.com/YAVSRG/YAVSRG/blob/f83a3a309e0d95d8a014d7a8a933ab289bc60e00/prelude/src/Gameplay/Scoring/OsuHolds.fs) and [window definitions](https://github.com/YAVSRG/YAVSRG/blob/f83a3a309e0d95d8a014d7a8a933ab289bc60e00/prelude/src/Gameplay/Rulesets/osu%21mania.fs) provide independent overhold boundaries consistent with the substituted-error calculation.

Synthetic regressions cover never-released versus never-pressed notes, OD-based
overhold boundaries, re-grabbed overholds, late-release invariance, tap/hold-head
expiry differences, and the two-note ischezayu sequence. No replay expected count
is used in the simulator. The analytical event timestamps still need reference
client verification at sub-millisecond/update-frame boundaries.

Remaining stable hold-map differences, ordered as Perfect, Great, Good, Ok, Meh, Miss:

| Replay MD5 | Expected | Observed |
| --- | --- | --- |
| `5dc9d27044b9e5b8a7cc5324ae3bfa15` | 787,442,17,1,0,0 | 787,442,16,1,1,0 |
| `c4e406a1241552a855edcbe6a10168aa` | 1313,484,47,6,1,4 | 1313,484,46,6,2,4 |

In DEUX EX MAXHINA, object 1244 starts at 114,917 ms: press 114,899,
release 114,913, re-grab 114,926, final release 115,662. The early release is
before the scheduled head. Dotabata Animation object 1731 also has a release and
re-grab around the head (168,075 / 168,089 / 168,238; start 168,231). These are
specific candidates for a different pre-head release policy, not proof of the
original object grades. Neither candidate is promoted to a different grade
just to reconcile aggregate counters.

Optional stable state tracing now distinguishes accepted heads, head expiry,
forced misses, pre-head/body releases, re-grabs, tail releases and overholds.
It retains the originally accepted head offset and the broken flag. The two
candidates produce `HeadPressed → ReleasedBeforeHead → Regrabbed → ReleasedAtTail`:
DEUX retains -18 ms; Dotabata retains -156 ms (already a Miss-sized head).
Ignoring the early break alone would produce Great in DEUX and still Meh in
Dotabata, not the missing Good in either replay. Replacing the stored head with
the re-grab is also unproven. The trace describes our state machine, not recovered
client events, and therefore does not justify changing these grades.

Trace and ordinary judging share one implementation; ordinary judging allocates
no transition log. Regressions compare their result streams, retain equal-time
release/repress order, distinguish forced misses from expiry, and reject
unsupported modifier policies in both entry points.

### Stable score and recovery audit

`--audit-stable` adds a separate diagnostic report,
[`tools/mania-stable-audit.tsv`](../tools/mania-stable-audit.tsv). Its first 21
columns are identical to the ordinary report. It appends a score-model label,
recorded header score, model score, absolute score error, number of pre-head
releases and their original zero-based object indices. A repeated release of
the same hold is counted separately. Lazer rows are `not-applicable`; unmeasured
inputs remain `not-measured`, with no fabricated zero score.

All 51 stable traces were inspected. Only DEUX object 1244 and Dotabata object
1731 contain `ReleasedBeforeHead`. The other 49 contain none. The first release
is 4 ms before the head in DEUX and 142 ms before it in Dotabata: shifting these
edges by half a millisecond cannot change that classification. This association
narrows the investigation but supplies no independent per-object judgement.

The diagnostic applies the documented
[NoMod ScoreV1 formula](https://osu.ppy.sh/wiki/en/Gameplay/Score/ScoreV1/osu!mania)
to our ordered combined tap/hold results. It is **not** a production score
implementation or an independent client oracle. Bonus starts at 100, updates
before each result and stays within 0–100. Base and bonus contributions are
accumulated as doubles, with one final ties-to-even rounding. No header value
participates in the calculation. Scheduling, body-break side effects and exact
client arithmetic remain unverified. Mod scoring is explicitly excluded, so
the HRDT row retains its recorded score but has no model score or score error.
Malformed, duplicate or incomplete event streams are not scored. Negative
recorded values remain visible but produce no comparison error.

| Stable NoMod score model | Replays |
| --- | ---: |
| Recorded score matches | 36 |
| Recorded score differs | 14 |
| Counts match but score model differs | 12 |

The largest absolute model difference is 8,500 points. These differences may
come from the score model or event order/grades; they are not automatically 14
new gameplay bugs. Engine `final_score()` and `score_at()` still return `None`
for stable. The ordinary judgement baseline remains 161/204 matching counts.

### Equal-time score bounds and the remaining evidence gap

The stable audit now appends four more columns: `stable_equal_time_order`,
`stable_order_min_score`, `stable_order_max_score`, and
`stable_order_score_relation` (31 columns in total). It computes the minimum
and maximum NoMod model score over **every permutation of grades sharing an
exact timestamp**, preserving order between distinct timestamps. Original
frame/column restrictions are relaxed, so these are optimistic bounds, not a
reconstruction of client scheduling. `within-bounds` does not establish that a
particular integer score is reachable.

Dynamic programming retains the lowest and highest accumulated scores for each
remaining grade multiset and integer bonus value. Future contributions depend
only on that bonus. The reference score is compared only after the bounds are
complete. A limit of 2,000,000 transitions per replay and 50,000 states per layer
returns `indeterminate-limit` with blank bounds; it never converts an incomplete
search into exclusion. Invalid streams and unsupported mods remain separate.
Tests independently enumerate 576 two-group permutations, including duplicate
grades, verify unreachable values inside the interval, and cover chronology,
empty/invalid inputs and exhausted budgets.

All 50 NoMod rows complete within those limits. All **14 remaining recorded
scores lie outside** their equal-time bounds: 13 above and one below. Thus
same-timestamp reordering alone cannot explain these differences under the
current grades and formula. The original 27 report fields are unchanged.
Examples are `39aecf6b` (recorded 934345, bounds 929346–929346) and `63ac67c9`
(recorded 909228, bounds 908123–908341). This does not exclude delayed client
updates that reorder events with different analytical timestamps, different
per-object grades hidden by matching totals, or inaccuracies in the formula.

For hold combo, inspecting the eight residual traces finds no exact 100 ms
interval boundary at all in four of them (`5dc9d270`, `d3b6e5d3`, `b45dd299`,
`738c4fd7`). Merely changing inclusion at such a boundary cannot fix these four.
An early [first-hand report](https://osu.ppy.sh/community/forums/topics/110785)
also describes combo varying with FPS, but it is not a timing specification for
the 2025 corpus builds. No fixed frame-rate adjustment is inferred from it.

A deterministic 512-input-stream regression checks unique object results,
monotone counts/achieved maxima, current combo bounds, EOF completion and
agreement between traced and ordinary playback across four mixed tap/hold lanes.
These checks verify internal consistency, not stable parity. Verification now
passes 420 beatmap/simulation tests, 25 corpus-audit tests and workspace checking.

At this point full precision is **not achieved**. The next independent evidence
is live replay output from stable, including per-object observations for the two
pre-head-release candidates. Five prioritized cases, exact hashes and observation
times are prepared in [mania-reference-checks.md](mania-reference-checks.md).
No guessed grades, tick subtraction or header-dependent result overrides are
applied. The 41 lazer timing witnesses remain counterfactual, not exact results.

### Stable simultaneous-result ordering

The previous merge sorted equal timestamps by column, losing both object order
inside a frame and the distinction between separate frames at the same time.
Key transitions now retain their original decoded frame index (including gaps
from idle/filtered frames). Stable results merge by timestamp, then frame index,
then original object index. Forced misses carry the triggering frame even though
they have no hit offset; automatic deadlines use no frame and precede input at
an exactly equal instant. Lazer's existing per-edge ordering is unchanged.

This experimental policy resolves four NoMod score comparisons:

| Replay MD5 | Recorded score | Before | After |
| --- | ---: | ---: | ---: |
| `4a891973dc4fc7ad42f5a265c2a97c95` | 973713 | 973718 | 973713 |
| `8f1bee70a0a5f497afb2748bdbe499a3` | 977070 | 977067 | 977070 |
| `a3c32ee877be3a1ea916bb375d266076` | 766292 | 766087 | 766292 |
| `2e33224f0cf3cbf4fc2caf2112bb2958` | 971246 | 971249 | 971246 |

All 32 earlier exact comparisons remain exact. Two other model scores change:
`63ac67c97e3c97eeb4bdd61e8e1b5b5a` improves from 908123 to 908341 against
909228; `3edcbdfd1f35a82c0e746b763e213860` moves from 877817 to 877801 against
886301, increasing its error by 16. No per-object grade or timestamp changes.
This is evidence for a better merge policy, not proof of original update-frame
scheduling. Numeric stable score remains diagnostic-only.

Tests distinguish one chord frame from two equal-time frames, preserve a forced
miss's triggering frame, and check tap/tail ordering independently of column.
Tap combo tests cover all successful grades, misses, EOF, backward seek, empty
maps and HR/DT combinations. All corpus grade vectors remain unchanged; lazer
combo, score and timing witnesses remain unchanged. The only ordinary report
changes are the 18 newly measured stable combo values and their zero errors.

A temporary counterfactual illustrates why both references are useful: changing
only DEUX object 1244 from Meh to Good makes this model round to the recorded
981,268. Doing the same to Dotabata object 1731 gives 955,437 instead of 957,475.
Neither substitution is applied to engine events or stored reports. The first
is stronger evidence for an object candidate, not proof of a general rule.

Six tests cover all grades, bonus recovery/reset, event-order sensitivity,
combined holds, invalid streams, modifier exclusion and reference independence.
The diagnostic introduction passed 23 audit tests and the workspace check; its
default and rounding reports were byte-identical to the preceding increment.
The later combo/ordering increment changes only the fields documented above.
Malformed-replay rows retain the selected report schema and mutually exclusive
audit flags fail before output.

## Expanded lazer coverage

All 153 lazer replays can be measured. The five replays added in this increment all match their
six expected count buckets, and results for the previous 148 are unchanged:

- `HAVA NAGILA HARDTEKK [mini yahu]`: four duplicate tap pairs are judged as
  separate notes, preserving all original object indices. This follows the
  ordered hit policy; it does not merge or silently discard duplicate notes.
- `TRILLIUM HARDTEKK [Trillium 0.o]`: configured DT 1.25× is read from replay
  settings. Rate compensation is applied to windows
  before flooring; beatmap and replay timestamps remain in beatmap time.
- `Utakata no Sekai de [EXPERT]`, `Cat Loving [Hard]`, `Halo [hyper~]`: AC does
  not change note grades. Its failure/restart policy is not simulated.

Tests cover duplicate taps before/at their common start, continued rejection of
overlapping holds, DT window boundaries/default/custom rates, unchanged input and
geometry time, mismatched rule-rate rejection, AC grade equivalence, and rejection
of unsupported/malformed modifier configurations. The single corpus DT replay
uses 1.25×; the default 1.5× rate currently has synthetic coverage only.

## Hold fix and timing investigation

Fixed re-grabbing a hold **before its scheduled head time**, after an early press
has already judged the head. Previously only already-started holds could be
re-grabbed. In `DJ GYROTTA ZAO - ZETRAKORE XAOS DA JUMPING GARGOYLZ
[JUMPSTYLE HYPER]`, object indices 849 and 879 now receive Meh tails instead of
false misses. Expected buckets are `[867, 369, 52, 4, 4, 4]`, observed
`[864, 371, 53, 4, 4, 4]` (Perfect, Great, Good, Ok, Meh, Miss). Its bucket error
fell from 10 to 6; overall error fell from 114 to 110. No other corpus counts changed.

The [official legacy score encoder](https://github.com/ppy/osu/blob/bdfabf93c77483bb471dfde8c900fa9176a9d531/osu.Game/Scoring/Legacy/LegacyScoreEncoder.cs#L172)
rounds input timestamps to integral milliseconds. Tail windows are multiplied by
1.5, leaving quarter-millisecond boundaries that this rounding can cross. For
example, an OD 8 Perfect tail window ends at 24.75 ms; an original 24.6 ms release
exports as 25 ms and becomes Great on replaying the rounded input.

- **40 of 41 differing replays** have aggregate differences compatible with
  independent ±0.5 ms tail-release uncertainty. The report adds candidate counts
  and `tail_rounding_compatible`. This is an optimistic diagnostic, not a fix or
  proof of original per-object results: shared frame timing, ordering, and broken
  holds are not reconstructed. Grades and measured totals are never adjusted.
- The remaining replay, `4K VIRUS - Ty menya ne ishchi`, has only taps. At
  94,404 ms, column 1, the judge hits object 1101 (Perfect) and forces object 1100
  at 94,312 ms to Miss. An original press just before 94,404 ms would hit 1100
  as Ok and leave 1101 to miss, matching the observed aggregate difference
  (one extra Perfect, one missing Ok). This is a specific note-lock rounding
  hypothesis, not recovered evidence of the original timestamp.

Regression tests cover both early re-grabs (hit/missed heads) and fractional
note-lock boundaries. Diagnostic tests cover upgrade/downgrade capacities,
conservation of counts, and exclusion of capped tails/taps.

### Shared input-frame rounding

The new `tail_frame_rounding` column strengthens the independent-tail check.
All eligible releases from one original exported input frame must use the same
displacement within ±0.5 ms. Frame identity comes from changes in the replay's
column mask, not timestamp equality: distinct frames can round to the same time.
If one column releases twice at that time, attribution is ambiguous and the
diagnostic returns `indeterminate-release`, rather than inventing provenance.

The diagnostic enumerates the joint grade changes between timing boundaries,
then checks whether one choice per frame can reach the expected bucket vector.
Search is bounded to 50,000 states per step and 2,000,000 option expansions;
exceeding either budget returns `indeterminate-limit`, not incompatibility.
Synthetic tests demonstrate two false positives in the old independent model:
opposite-sign tails that would require opposite frame shifts, and identical
chord tails that would have to receive different grades. A small exhaustive
enumeration independently checks the pruned search.

**All 40 lazer hold-map differences remain compatible under this stronger
constraint.** The 112 matching lazer replays are compatible with zero shift;
the tap-only note-lock case remains incompatible with this tail-only model.
There are no indeterminate results in this corpus. Stable rows leave the column
empty. All 204 rows retain their previous statuses, observed grades, errors and
independent-tail diagnostics.

Compatibility still does not recover original judgements. This calculation
freezes object assignment and hold state, excludes capped tails, and does not
enforce ordering between distinct frames or reconstruct client updates. Its
inclusive half-millisecond endpoints are an optimistic uncertainty envelope.
The replay header is used only to query diagnostic reachability, never to change
the simulator or its observed results.

### Full-judge fractional-frame probe

The tap-only note-lock hypothesis is now reproducible through the same lane
judge, rather than only a local window calculation. For replay
`2287552da2bda0569a1f722a2a2099fb`, original frame 6219 is at 94,404 ms with
column mask 9. Giving only this frame a hypothetical -0.25 ms displacement,
while preserving frame order, changes exactly two events:

| Object | Original input | Hypothetical input |
| --- | --- | --- |
| 1100, start 94,312 ms | forced Miss at 94,404 | Ok at 94,403.75, offset +91.75 |
| 1101, start 94,404 ms | Perfect at 94,404 | automatic Miss at 94,531.5+ |

The hypothetical vector is `[630,552,229,49,13,22]`, matching the header; the
original remains `[631,552,229,48,13,22]`. No other event changes. This is a
possible explanation inside export uncertainty, **not evidence that the original
frame actually occurred at 94,403.75 ms**. The original report keeps this replay
as differing, and `tail_frame_rounding=incompatible` remains correct because
that diagnostic deliberately does not model note-lock changes.

`--trace <hash> --probe-frame <index> <offset-ms>` runs this explicit experiment.
The index is zero-based in the decoded original replay frames. Probe metadata,
counts and changed events go to stderr; stdout retains the original unmodified
trace. The flag is lazer-only, rejects non-finite/out-of-range offsets and frame
indices, and fails if a shift would reorder frames, including unchanged masks.
It never searches for offsets based on expected counts or writes a replay.

Zero-offset probes preserve all judgement events in all 153 measured lazer
replays. Targeted tests cover fractional note-lock selection, common chord
timing, held-note state, equal-time release/repress order and invalid input.

### Full-judge rounding witnesses

`--audit-rounding` performs a separate bounded counterfactual search and appends
six diagnostic columns. The saved output is `tools/mania-timing-audit.tsv`;
`tools/mania-corpus.tsv` remains the ordinary measured report.

| Lazer outcome | Replays |
| --- | ---: |
| Counts, maximum combo and unmodified score match without changes | 112 |
| All three match under an admissible timing hypothesis | 41 |
| No witness found / budget or provenance indeterminate | 0 |

The search evaluates interior timing representatives around successful tail
grade boundaries and exact-start presses that force an earlier head to miss.
All column changes in a frame share one shift. Unconstrained neighbouring frames
at the same exported timestamp move only when necessary to preserve frame order;
conflicting fixed choices are rejected. Every candidate runs through the full
judge, including note lock, hold breaks, automatic misses, combo and numeric
scoring. The matcher requires all six counts, recorded maximum combo and recorded
`total_score_without_mods` to agree simultaneously.

All **41** differing lazer replays have a witness: 40 involve tail timing and
one involves note lock. Across the corpus this took 205 assignments, including
three rejected for reversed frame order (202 full judgements). The largest
individual search took 72 assignments. Witnesses change one to four original
frames by ±0.375 ms for tails or -0.25 ms for note lock. An independent check of
the original replay bytes verifies every saved index, chronological order and
that each adjusted absolute timestamp rounds back to its stored millisecond.
No map, replay, input mask, simulator rule or measured total is changed.

**This establishes compatibility, not recovery of the original timestamps.**
The search uses recorded aggregate results to select a witness. It cannot prove
unique timing or per-object equivalence, and its output must never replace the
ordinary simulation or be used as an accuracy regression oracle. The baseline
therefore remains 112 exact lazer scores and 41 differences. There is currently
no lazer mismatch unexplained by this timing model; new unrounded input or
independent per-object results are needed to distinguish the possible histories.

The search is deliberately incomplete: it keeps one interior representative per
local grade combination, excludes exact half-millisecond endpoints and does not
enumerate all client-update timings or capped-tail states. `no-witness-found`
does not mean incompatibility. A hard limit of 16,384 assignments per replay
returns `indeterminate-limit`; ambiguous repeated input edges return
`indeterminate-provenance`. Stable policies are `not-applicable`.

The appended fields are `timing_probe`, `probe_candidates`, `probe_trials`,
`probe_rejected`, `probe_shifted_frames` and `probe_offsets`. The last is a list
of zero-based original frame indices and hypothetical offsets in milliseconds.
All 21 original columns are identical to the ordinary report. Six new tests
cover replaying witnesses, score/combo rejection despite count compatibility,
indivisible chord releases, equal-time frame ordering, note lock, ambiguous
provenance and budget exhaustion.

## Follow-up order

1. Obtain reference per-object results or unrounded replay timing before calling
   the remaining boundary cases exact. The existing exported files cannot supply
   lost sub-millisecond input times; do not tune windows to match header totals.
2. Preserve full-precision input when a supported source becomes available;
   keep the exported-input baseline and counterfactual reports separate.
3. Verify stable pre-head release/re-grab and combined tail timeout against
   per-object references; then add other stable modifiers, ScoreV2 and older-lazer
   policies. Keep overlapping-hold behaviour gated until targeted fixtures exist.
4. Implement HP, mod multipliers and stable hold score before rendering, and
   explain the eight stable hold combo residuals;
   retain independent score comparisons while investigating the 41 remaining
   lazer judgement differences.

## Reproduce

Verification: the hold-combo increment passes 419 beatmap/simulation tests,
all 23 corpus-audit tests, and the workspace check. Five added regressions cover
hold boundaries, breaks/re-grabs, delayed expiry of a previous hold, backward
queries against an explicit two-lane tick stream, and huge duration/range limits.
Ordinary and traced judging return the same combo snapshots.
All three reports retain 204 rows and change only the 33 previously unavailable
stable hold combo observations and errors. The original 21 fields agree across
all three reports. No lazer result, stable grade vector, diagnostic stable score,
or timing witness changes. The earlier score increment also passed
35 replay-parser tests; parser code is unchanged in this increment.
The preceding diagnostic increment traced all 50 NoMod stable replays: result counts match ordinary
judging, each object has one result, all trace rows have 12 fields and finite
timestamps, and state timestamps are ordered. An unknown replay hash fails
without emitting a report. These checks validate diagnostics, not client parity.
The new HRDT trace also has 824 unique object results matching its aggregate
vector, valid 12-column rows and ordered state transitions. Trace diagnostics
now label legacy modifier bits separately from the optional lazer mod list.

```sh
cargo run --locked -p dossier-cli --example mania-corpus -- "/path/to/Dossier Corpus" > /tmp/mania-corpus.tsv
cargo run --locked -p dossier-cli --example mania-corpus -- "/path/to/Dossier Corpus" --audit-rounding > /tmp/mania-timing-audit.tsv
cargo run --locked -p dossier-cli --example mania-corpus -- "/path/to/Dossier Corpus" --audit-stable > /tmp/mania-stable-audit.tsv
cargo run --locked -p dossier-cli --example mania-corpus -- "/path/to/Dossier Corpus" --trace 9a8112fc6456122491aea751177f990d > /tmp/mania-trace.tsv
cargo run --locked -p dossier-cli --example mania-corpus -- "/path/to/Dossier Corpus" --trace 2287552da2bda0569a1f722a2a2099fb --probe-frame 6219 -0.25 > /tmp/mania-original-trace.tsv
cargo test --locked -p dossier-cli --example mania-corpus
cargo test --locked -p dossier-beatmap -p dossier-sim
cargo test --locked -p dossier-replay
cargo check --workspace --locked
```

The audit writes only stdout. It does not alter maps, replays, or expected values.
Trace mode writes input edges, judgement events and (for stable) state transitions
to stdout, with metadata on stderr; object indices are zero-based and displayed
columns are one-based. The 12-column TSV groups inputs, results and states into
separate sections; sort by time if a merged view is needed. Within the state
section, equal-time transitions in one column retain their processing order.
Keep this measured report separate from the standard corpus manifest and do not
use `counts-match-experimental` as a promise of full mania support.
