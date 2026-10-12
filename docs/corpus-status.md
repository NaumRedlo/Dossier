# Render corpus status — 2026-10-11

## Reproducing the check

```sh
cargo build --locked --release -p dossier-cli
python3 tools/check-corpus.py "/path/to/Dossier Corpus"
```

The runner recursively discovers replays, compares each unique file against
`tools/corpus.tsv`, and propagates the checker's exit status. It never updates
expectations. By default it uses two measurement workers. Maps may live elsewhere:
pass `--songs /path/to/Songs`.

## Measured state

- 254 replay files, 191 unique contents, 63 duplicate copies.
- 211 historical expected replays; 20 currently absent (probably lost, according
  to the owner). Their expectations remain intact for possible recovery.
- All 191 available replays measured; no skipped measurements; 12 lazer replays.
- 181 match both hit counts and maximum combo; 10 still differ.
- Total hit-count error: 20. No per-replay regression against the saved baseline.
- Score compared for 187 replays: 186 within 0.5%; largest difference 0.57%.
- The full strict check exits 1 because 20 expected files are absent. This is a
  coverage failure, not a newly introduced judgment regression.

“Exact” here refers only to hit counts and maximum combo. It does not establish
pixel fidelity, sprite timing, audio synchronization, exact score, or acceptable
resource use on Aspire maps. The saved baseline includes known discrepancies;
passing regression comparison does not mean the engine is fully accurate.

## Restored maps

Three map files were historically recovered from the official osu! map endpoint and verified
against the exact hashes expected by the replays before copying them into the
local corpus. No existing files or expectation rows were replaced.
This historical retrieval method was prohibited by osu! support on 2026-10-12
and has been removed. Future recovery uses local files or verified mirror archives.

| Map ID | Expected MD5 |
| --- | --- |
| 5412809 | `4afc943f746e82424df81dd398aac2c3` |
| 5205868 | `30b8af8303da25aad3e9016bd28fc2f6` |
| 5547437 | `348bcaf8a31a6ddd71a63e800df7621a` |

## Check reliability

Unreadable replays, missing maps, and parse errors now report their paths and
reasons. Failed measurements fail checked runs and prevent baseline updates,
including updates with pruning. A run with zero measurements always fails.
Numeric strict ceilings also enforce the presence of expected replays.
Malformed hashes, duplicate replay rows, and non-finite score expectations are
rejected instead of weakening comparisons.

## Next work

1. Investigate the 10 remaining available judgment/combo mismatches individually. Preserve
   existing expectations while distinguishing engine bugs from missing replay
   input or client-specific behavior.
2. Add visual reference cases for sprite frames, looping, fades, transforms,
   seek/restart, and stable/lazer differences. Judgment totals cannot test these.
3. Add reproducible Aspire stress cases and record wall time and peak memory
   before changing resource limits or geometry handling.
4. Build PP and difficulty-graph validation on the established timing and
   difficulty pipeline after these foundations are measured.

## Lazer slider corrections — 2026-10-11

The first follow-up improved exact hit-count/combo matches from 178 to 180 of
191, without per-replay regressions against the unchanged baseline. Hit-count
error stays at 22; the improvement is in combo reconstruction. Maximum score
error dropped from 1.01% to 0.76%.

- After losing tracking, a slider must be reacquired inside its normal ball
  radius. The expanded follow area granted on the head press must not be
  granted again on every later attempt to resume tracking.
- Lazer tracking can resume on interpolated cursor positions between recorded
  samples; requiring an exact replay sample delayed recovery. Stable keeps its
  existing frame restriction.
- A tail recovered during lenience is credited at recovery time, preserving its
  order relative to intervening hits and combo breaks. Its drawing time remains
  at the slider end.

The radius and tail rules were cross-checked against the official
[SliderInputManager](https://github.com/ppy/osu/blob/master/osu.Game.Rulesets.Osu/Objects/Drawables/SliderInputManager.cs).
Synthetic regression cases cover returning inside/outside the normal radius,
returning between replay samples, and a miss before a recovered tail.
The corpus remains the check for the recorded client versions; current upstream
source alone is not proof of fidelity for every historical client.

Observed improvements:

| Replay MD5 | Map | Combo error before → after |
| --- | --- | --- |
| `a1e76dae90dc56dd1fc517b40d9b6bdc` | Astral Quantization (gusrua123) | +2 → 0 |
| `f6f6d6ec9ee1852fadee36f871b1740e` | Spider Dance (wickedslayer) | +1 → 0 |
| `8403a04f34dd1374e8516c3bca0902c9` | Nhelv (ncrohawk) | +5 → +1 |

The original 13-case audit is six stable and seven lazer recordings. Following
these fixes, six stable and five lazer recordings still disagree. None has been
relabelled as exact or removed from the expectation file to make the check pass.

## Follow-up investigation — overlapping lazer notes

The accepted result remains **180/191**, count error **22**. The following
experiments were rejected and are **not present in the simulator**:

1. Removing stable's stacked-note guard for non-classic lazer corrected the
   hit-count vectors for Astral Quantization and Blue Zenith, but reduced
   Astral's maximum combo from 1291 to 1290 and worsened Blue Zenith's score
   residual from the saved +0.14% to approximately -0.41%.
2. Scheduling forced misses just after the displacing hit restored Astral's
   combo, but introduced other regressions, including a previously matching
   Maidcore replay's maximum combo. This is not a valid general ordering rule.

For Astral Quantization (`4b32f50ffe753c27c677fcfb97c0aa65`), the current trace
identifies zero-based objects 843 and 844, starting at 138792 and 138873 ms.
The press at 138821 ms is inside the later slider head but outside the previous
stacked circle. It is ignored by the stack guard; the subsequent press at
138898 ms hits the circle late for 50. The replay instead records one more 100
and one fewer 50. This localizes the affected sequence, but aggregate header
counts do not establish a complete per-object ground truth.

The official [StartTimeOrderedHitPolicy](https://github.com/ppy/osu/blob/master/osu.Game.Rulesets.Osu/UI/StartTimeOrderedHitPolicy.cs)
and [OsuPlayfield](https://github.com/ppy/osu/blob/master/osu.Game.Rulesets.Osu/UI/OsuPlayfield.cs)
are the reference for the next step: reproduce the ordering of hit callbacks,
forced misses, and nested slider results together. Do not fix this by shifting
all forced misses or by changing the expectation file.

For Nhelv (`8403a04f34dd1374e8516c3bca0902c9`), the current type totals contain
738 slider-tail hits versus 737 in the replay, while large ticks and ordinary
hit counts agree. Its +1 maximum combo is therefore not evidence of a missing
combo break; investigate tail eligibility within the longest chain.
For Kyouran Hey Kids (`a95800c5f645ba1ad5636d0f0a107ac6`), tail and large-tick
totals agree, but maximum combo is 389 versus 390. Check event ordering at the
longest chain's boundaries (objects 169 and 388) before assuming extra misses.

The CLI diagnostic now reports a combo deficit as a deficit, not as a number
of extra breaks. Candidate boundaries are explicitly conditional. Neither the
maximum-combo difference nor aggregate 300 counts can prove how many breaks
occurred or uniquely identify the responsible objects.

## Short slider-head lifetime — accepted follow-up

The accepted result is now **181/191**, with total hit-count error **20**, no
skipped measurements, and no per-replay regressions against the unchanged
expectation file. Score coverage remains 185/187 within 0.5%, worst 0.76%.

Lazer slider heads retain their hit window after a short slider path has ended.
The old sweep expired every slider at its path end, redirecting valid late
head presses to the next object. The sweep now uses the head window for lazer;
stable retains its existing end-based slider lifetime. The distinction follows
[DrawableSliderHead](https://github.com/ppy/osu/blob/master/osu.Game.Rulesets.Osu/Objects/Drawables/DrawableSliderHead.cs)
and its inherited [DrawableHitCircle result check](https://github.com/ppy/osu/blob/master/osu.Game.Rulesets.Osu/Objects/Drawables/DrawableHitCircle.cs).

In Ivaxa's The Limit Does Not Exist replay
(`234bf6bee02b570870c62f1353f5e478`), zero-based slider #843 starts at 107974 ms.
The 108033 ms press now reaches its head (+59 ms, 100), and the 108068 ms press
reaches #844 (-21 ms, 300). Previously the first press incorrectly reached #844
and #843 was missed. All hit counts and maximum combo now agree with the replay.

A regression test uses a 50 ms slider and a press 80 ms after its start. It
checks the lazer 100, unchanged stable miss, and a miss outside the head window.
All 272 CLI/simulation tests passed, and the full available corpus was rerun.
Nhelv's remaining +1 combo is not fixed by this change.

## Spinner bonus display

The renderer independently counted every half turn after the stable bonus
threshold, displaying 1000 for each one. The judge awards bonuses on alternating
half turns instead. This made the visible bonus grow too quickly and restarted
its animation on non-bonus turns. The renderer now consumes only successful
`SpinnerBonus` events for the current object, using their timestamps for the
animation. It no longer allocates and filters two temporary turn lists per frame.
This also avoids applying a half-turn threshold to lazer's full-turn tick events.

Two regression tests cover the threshold, intervening non-bonus turns, seeking
backwards, full-turn event streams, and missed bonuses. All 556 CLI, simulation,
and renderer tests passed (8 existing ignored tests). This is a display fix;
the total-score calculation and the 181/191 hit-count/combo corpus result are
unchanged. It does not establish that a user-reported excessive final score is
resolved without the affected replay.

## Lazer spinner completion thresholds

Lazer's completion requirement is rounded to full rotations, not stable's half
rotations. The simulator previously used the stable threshold for both clients,
requiring an extra half rotation whenever the rounded half-turn count was odd.
The requirement and maximum bonus count now use lazer's full-turn rounding and
0.0001 floating-point allowance, matching
[Spinner.ApplyDefaultsToSelf](https://github.com/ppy/osu/blob/master/osu.Game.Rulesets.Osu/Objects/Spinner.cs).
Stable's rounding is unchanged. The renderer uses the same requirement for its
completion meter; its Clear timestamp now indexes lazer's full-turn events as
full turns instead of half turns.

Regression cases cover a one-second OD5 spinner completed with 2.1 rotations,
a short spinner requiring zero rotations, and duration rounding on either side
of the tolerance. All 558 CLI/simulation/renderer tests passed (8 ignored).
The full corpus remains 181/191, count error 20, with zero regressions against
the unchanged baseline. These cases fix a verified rule discrepancy but do not
resolve any of the ten remaining recorded hit-count/combo mismatches.

## Nested-judgement ordering audit — 2026-10-12

The accepted simulator remains at 181/191. Three additional experiments were
run against all 191 available replays and reverted:

- Deferring lazer nested score timestamps until head resolution restored
  Kyouran Hey Kids!!'s maximum combo from 389 to 390, but changed Blue Zenith's
  combo from exact to -1 and increased Kyouran's score residual from +0.08% to
  +0.11%. Overall hit/combo matches reached 182/191, but this was not accepted.
- Combining the deferral with removal of the stable stacked-note guard for
  lazer also introduced Astral Quantization's -1 combo discrepancy.
- Waiting to judge nested parts until head resolution, allowing tracking at
  the clamped endpoint, reduced total count error to 16 but left Kyouran at
  -2 combo / -1.00% score and Blue Zenith at -1 combo / -0.52% score. This was
  also rejected. The synthetic case alone passing did not validate the model.

The debugger now prints score events in their actual scoring order, including
part, result, combo after the event, and a separate drawing timestamp when it
differs. For Kyouran's zero-based slider #169, the current accepted engine awards
the tail at 56862.634 ms (combo 308), shows the slider verdict at 56898.634 ms,
then misses the head at 56933 ms (combo 0). This is a concrete ordering suspect,
not proof that moving that one event is a complete fix. Baseline expectations
were not changed.

## ScoreV2 spinner normalization — 2026-10-12

Fixed an independent total-score error: stable ScoreV2 included spinner point
and bonus events in the maximum combo-score denominator, while correctly
excluding them from the earned combo-score numerator. Extra rotations therefore
depressed the normalized base score. Bonus events are now excluded from both
sides and remain in the separate bonus sum. This follows the documented
[ScoreV2 million-point base plus spinner bonuses](https://github.com/ppy/osu-wiki/blob/master/wiki/Gameplay/Score/en.md).

A regression replay with a single four-second OD5 spinner and 20 rotations
earned just 462638 before the fix despite a 300 judgement. It now exceeds the
million-point base; a second replay with 25 rotations must increase the score
rather than decrease it. The test was observed failing before the fix and
passing afterwards. All 275 CLI/simulation tests passed. The full available
corpus still reports 181/191 hit-count/combo matches, count error 20 and zero
regressions against the unchanged baseline (strict check still fails on the 20
absent replays). This is a verified scoring correction, not a resolution of the
ten remaining judgement discrepancies. Uika's Bug replay has no spinners, so
its ScoreV2 residual requires a separate investigation.

## Stable ScoreV2 slider verdict timing — 2026-10-12

Stable ScoreV2's slider grade combines head accuracy with the collected pieces.
It was nevertheless emitted at the head press, prematurely showing the result
of future tracking and using the head-time combo for score weighting. It now
uses the same end/update ordering as stable's whole-slider verdict. Lazer's
head-only judgement remains at the head press.

The Bug replay (`bdb980f3056fd5afa2a9c5b2d19c5688`, NFHDV2) score residual
improved from -0.7566% to -0.4751%. Its single 300/100 discrepancy remains.
Full-corpus hit-count/combo matching stays 181/191 with zero baseline regressions;
186/187 scores are now within 0.5%, and the worst score residual is 0.57%.

A regression test compares two plays identical until after the head, one
releasing early and one holding to the end. Neither is assigned a slider grade
before the paths diverge; the head still adds combo immediately and the final
grades differ correctly. The existing 275 CLI/simulation tests and the added
regression test passed.

## Missing historical replays

These rows remain in `tools/corpus.tsv`; names alone are not identity. Recovery
must match the complete replay hash. Do not replace them with newly played scores
under the same identity.

| Replay MD5 | Name |
| --- | --- |
| `06e369924da11c087c18bb1e5a2657ea` | solo-replay-osu_5500905_7248916183.osr |
| `1dd7ae76bbd7a3ab18e1b3736eda9adf` | _GN_3_playing_DJ_TOTTO_Crystalia_Hysteria_Meal's_Ultra_2026_08_11.osr |
| `2ff82d9566383f3c8532dcab2187a10f` | Sakiko Togawa - yax03 - down [noob...] (2026-08-10) Osu.osr |
| `40cb01b3d29e05c4ce4c366249cc0291` | Guest_playing_ATL_Magic_people_Ksardas_Hard_2026_08_08_18_47.osr |
| `46d91c02f3508d4063d8dcdfbc7a868a` | Guest_playing_IOSYS_Usatei_2011_Saten_Usamimi_2026_08_09_19_17.osr |
| `48ea4ed297dc622f97f22e774fdb0429` | syna_psis_Ian_LeFeuvre_Johnny_Test_Theme_TV_Size_FukA's_Genius_Sisters.osr |
| `54dc2bef5343985b2087947ea99eba53` | unxora_That_Poppy_Altar_I'm_Poppy_2026_08_09_Osu_1.osr |
| `5826192bba6c8813a267eae01aafffc5` | syna_psis_EDOGA_SULLIVAN_WONDERFUL_WONDER_TV_Size_&_Sped_Up_Ver.osr |
| `67a63f72ad49cb4b7f11558b6d98a4a5` | pahanishe_Sayuri_Heikousen_Meg's_Taboo_2026_08_11_Osu.osr |
| `758280049ceb6b65c1c2038d536724e1` | pahanishe_Artemas_i_like_the_way_you_kiss_me_DETHVEDA_Frenchcore.osr |
| `907787e5fb1d35976d54a046962630a6` | pahanishe_Pythia_Heartless_Cut_Ver_Maaadbot's_Extreme_2026_08_14.osr |
| `a9087e7c713408801baf95d418e75f11` | solo-replay-osu_5676228_7303401786.osr |
| `af0ec058a879e047d04e895287481b8a` | solo-replay-osu_5282312_7213356902.osr |
| `b7a5aca223b2baa1cdad1290ff5badd1` | Guest_playing_The_Koxx_A_FOOL_MOON_NIGHT_Astar_Naos_Eclipse_2026.osr |
| `c452b053f698efc4999a949d21db9480` | goprob_Olivia_Rodrigo_all_american_bitch_daph_and_nekszy's_cocacola.osr |
| `c98ed299878e691772de5cee8e058589` | solo-replay-osu_5727651_7275254096.osr |
| `d714837bedfbaf03892c487ebe656795` | pahanishe_Yamajet_feat_haruamanya_Sweets_Holic_Cut_Ver_Lemaire's.osr |
| `e2058f9b05da50e6fada8984b775c1df` | _GN_3_playing_HARU_NEMURI_Trust_Nothing_but_Love_Ryuusei_Aika_Log.osr |
| `ea29dddfcc4fd98023fea9efc83ad301` | replay-59835.osr |
| `fe60718475671d7091b88242da866234` | Uika_Misumi_playing_Down_Halfslashed_Down_Extra_2026_08_10_00_31.osr |
