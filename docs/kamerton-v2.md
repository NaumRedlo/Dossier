# Kamerton v2: skill × mod, an extra slot

A plan for the next version of the slot model. The condition is unchanged: the
architecture changes only with a measurable gain on the same OWC year splits.
The server code lives in the `1984` repository (`services/duels`), the review in
`native/src/desk_screen.rs`.

## Why one label per map gets in the way

Today Kamerton is one logistic regression: map features → one of the slots.
Training pulls the labelled slot's probability towards 1 and **every other slot
towards 0**. That causes three problems:

1. A slot is a combination of a **skill** and a **mod**: NM1 "Jumps" and HR1 "Aim
   with HR" test one skill under different mods. The model learns them as
   unrelated classes and splits an already small sample between them.
2. A map that honestly fits two slots becomes a negative example for the second
   one during training. The model learns to be confused.
3. Rare slots (HR1, HD1, DT3, FM1–FM4) have almost no examples, while criterion
   M04 asks for at least 30 checked maps per slot.

## Skills

A skill is what a map tests in a player regardless of the mod. Each slot splits
into a skill and a mod; combined slots give a skill a share.

| Skill | What it tests | Slots (mod) |
|---|---|---|
| `aim` | long jumps, direction changes, arm load | NM1, DT1, HR1, HD1 (steady aim), FM1 |
| `stream` | long even runs of notes | NM2 |
| `alt` | runs with spacing between notes at a middle tempo | NM3, HD3 (½) |
| `speed` | high tap density, tapping stamina | NM5, DT2 |
| `burst` | short fast runs of 3–8 notes | DT3 |
| `tech` | rhythm changes, sliders, unusual patterns | NM4, HD3 (½), HR3, DT4 (½), FM3 |
| `reading` | screen density, overlaps, low AR | HD2, NM6 (½), FM2 (½) |
| `gimmick` | unusual patterns and anti-mod | NM6 (½), DT4 (½) |
| `precision` | small circles and exact transitions | HR2, FM2 (½) |
| `mix` | an even mix of skills, length | TB |

By OWC rules free mod allows HD, HR or HD+HR. FM1 counts as aim like NM1, HD1
and HR1, FM2 as accuracy and reading like HR2 and HD2, FM3 as technique like
NM4, HD3 and HR3. FM4 is not used.

The skills are stricter than the slot names: `stream`, `alt`, `speed` and
`burst` are kept apart because in pools they are different tasks, and one
"speed" label mixed them.

### The skill profile: a transparent estimate before training

So that the model does not start from nothing, every skill has a **signature**:
a formula over the analyser's features with signs fixed in advance, without
training. Features are normalised over the corpus (median and interquartile
range, as in `Normaliser`) and the signature is mapped to 0–1. For example:

- `aim`: larger distances in radii (p90) and peak 2 s aim load, smaller stream share;
- `stream`: stream share and runs of 16–31 and 32+;
- `alt`: streams × spacing between notes inside runs above the usual;
- `speed`: a short interval (median), 10 s tap load, share of loaded time;
- `burst`: runs of 3–5 and 6–15 with a short median pause between runs;
- `tech`: rhythm changes (p90), slider share and speed, repeats;
- `reading`: visible objects (p90), overlaps, long approach time;
- `precision`: effective CS and distances in radii with a small radius;
- `mix`: length and evenness of load (share of loaded time over 30 s, no dominant skill).

The profile serves three goals: it gives the model ready, meaningful features;
it is shown to the reviewer ("the map is 80% aim, 40% technique"); and it is
tested for truth: for each skill we measure how well the signature separates
maps labelled with that skill from the rest (AUC). A signature that does not
separate is fixed or dropped.

## A two-layer model

```
features in the mod's form ─┬─► skill layer: softmax over skills ──┐
skill profile ──────────────┘                                      ├─► slot = skill × mod
NF, NFHR, NFDT features, rules ─► mod layer: yes/no ───────────────┘
```

**The skill layer.** A softmax over skills whose input is the features **in the
slot's mod form** (DT1 with DT, HR1 with HR) and the skill profile. It learns from
all slots at once: `aim` gets the examples of NM1, DT1, HR1, HD1 and FM1. A combined
slot gives a soft target (HD3: `alt` 0.5, `tech` 0.5).

**The mod layer.** A separate yes/no for NM, HD, HR and DT: mods do not compete,
and a map may fit several. The hard rules stay: HD2 only at AR ≤ 8.5, HR2 only at
CS ≥ 6 after HR, and stars in the mod's form within 5–10★. Positive examples are
the mod the map was put under and confirmed extra slots; negative ones are only
explicit decisions by people (removed from a slot, moved). Everything else is
"unknown" with a small negative weight, otherwise the model would learn that
almost any map is "not for HR".

**The slot.** A slot's score = the skill's probability (in the slot's mod form) ×
the mod's fitness, normalised over the allowed slots. The best combination is the
main slot; a strong second one with another skill or mod is a candidate extra slot.

Both parts are logistic regressions: on ~600 maps a neural network would overfit
and lose explainability. The explanation becomes two-part: "this is an aim map
(distances, aim load peaks); with HR it stays aim and asks for precision".

## The extra slot

- An assignment keeps one main slot and gets at most one extra slot. It is stored
  as an ordered list so it can grow later without a migration.
- Only a person sets the extra slot; Kamerton suggests one when the second
  combination is close to the first.
- The pool generator takes a map into its extra slot at a surcharge, that is,
  when the main slot's corridor lacks maps. A map is never repeated in a pool.
- The catalogue shows extra slots separately: "5 +2".
- "Does not fit the slot" feedback refers to the slot the map was played in.
- Training: both layers of the accepted model learn the two allowed answers
  explicitly. In a map's target the extra slot gets `SECONDARY_SHARE` (0.25) and
  the main slot the rest. In the skill layer this is a mix of the two slots'
  skills; in the slot layer the extra slot becomes a class of its own, even a
  reserve one. That is why HR1 on NM1 maps changes the model although their skill
  is shared. The data fingerprint includes the extra slot, and adding one is a
  reason to train again. In the report, `admitted.acceptable` is the share of
  admitted maps that landed in their main or extra slot.

## Order and checks

1. **An honest estimate** (step 1 of the DT/FM4 plan): one decision path for the
   check and for real assignment; a report over all classes, by skill and by mod,
   and the second slot's share among the top two.
2. **Load windows:** fix the false peak of the last note, raise `FEATURE_VERSION`,
   recompute analyses from the stored `.osu` files.
3. **The skill profile** and the signature check (AUC per skill).
4. **The two-layer model** next to the current one, on the same splits. The report
   compares both: macro-F1 of the working slots, precision and recall per slot,
   rare slots, precision of confident decisions. It is published only as a shadow
   version and only if it is not worse.
5. **The extra slot:** storage, the reviewer's decision, the generator, the
   catalogue, the interface.

The measuring corpus is built by the same `load_corpus` (OWC 2021–2025 pools from
osu-wiki and files from allowed mirrors). OWC labels are the original slot
numbers, not checked skills: conclusions from them are preliminary, and the
decisive check is the reviewers' labelling.

## Measurement of 27 October 2026

Corpus: 566 OWC 2021–2025 maps (`load_corpus`, files from osu.direct; 40 maps were
not found), `features-v2` after `reanalyse`. Each year is held out in turn, map
sets never overlap with training; the three models decide through one path
(`kamerton.decide`, with the HD2 and HR2 rules). Labels are OWC slot numbers.

| Working duel slots, 287 maps | Current Kamerton | Skill × mod | Skills → slots |
|---|---|---|---|
| macro-F1 | 0.803 | 0.711 | 0.802 |
| Right slot among the top two | 0.83 | 0.79 | 0.83 |
| Would be admitted to duels | 123 (43%) | 33 (11%) | 148 (52%) |
| Precision of the admitted | 0.94 | 0.85 | 0.95 |
| Of them confident | 50 (17%) | 7 (2%) | 82 (29%) |
| Precision of the confident | 0.92 | 1.00 | 0.95 |
| Refusals: outside the trained range / not a working slot / below threshold | 6 / 43 / 115 | 14 / 46 / 194 | 5 / 48 / 86 |
| Catalogue, 566 maps: macro-F1 over working slots | 0.654 | 0.570 | 0.664 |
| Catalogue: macro-F1 over all 21 classes | 0.393 | 0.345 | 0.397 |

The table was recomputed after review. The earlier count took a model's answer as
confident even when the map lay outside the trained range and would never reach a
duel (stacking had 88 confident at 0.93 precision). And the "catalogue, 21
classes" row actually computed macro-F1 over working slots only. The report now
separates catalogue classification (`macro_f1`, `macro_f1_all`) from real
admission (`admitted`: share, precision, refusals by reason).

**The skill × mod product is rejected.** A skill estimated in the slot's mod form
mixes with the mod: HR2's CS is multiplied by 1.3, DT3's AR rises, and the
features tell the mod apart rather than the skill. Besides, OWC numbering does
not follow a "HR1 is aim with HR" scheme, and a product over 21 classes smears
confidence.

**Stacking is accepted: skills → slots** (`kamerton.StackedModel`). The first
layer estimates ten skills from the map without mods and in its DT form (trained
outside its own third of the sets); the second is the old slot model, getting
these estimates along with the usual features. Slot quality is the same, and 1.6
times more maps are assigned confidently with no loss of precision. HD2, HR2, NM1
and HR1 got better; NM3 and DT3 got worse, which is the next thing to work on.

**Skill signatures** (AUC against OWC labels): precision 0.98, streams 0.85,
reading 0.85, mix 0.76, technique 0.75, bursts 0.75, speed 0.74, aim 0.69; alt
0.45 and gimmick 0.52, whose formulas are wrong and will be rebuilt from the
map's features without mods.

To repeat the measurement on the server's database: `python scripts/duel_corpus.py compare`.

## Rhythm, measurement of 29 October 2026

Corpus: 602 OWC 2021–2026 maps from allowed mirrors, labels are OWC slot numbers,
each year held out in turn. The `features-v3` analyser computes for each pair of
neighbouring notes the beat fraction (1/2, 1/4, 1/3, 1/6, 1/8, a whole beat,
other), the triplet share, how often the grid changes, pauses and the number of
BPM changes. Without rhythm NM3 cannot be told apart since 2023: pickers put more
and more 1/3 alternating into it.

| Working slots | v2 | All rhythm features | Triplets and grid changes |
|---|---|---|---|
| macro-F1 | 0.820 | 0.807 | 0.817 |
| Would be admitted to duels, precision | 174, 0.943 | 171, 0.953 | 173, 0.954 |
| Confident, precision | 98, 0.949 | 102, 0.971 | 103, 0.971 |
| NM3 | 0.676 | 0.667 | 0.706 |
| NM4 | 0.757 | 0.833 | 0.806 |
| HD2 | 0.812 | 0.727 | 0.824 |

All nine features at once add noise: on 600 maps HD2 and NM2 dropped. Only the
triplet share and the grid change rate went into the model; the other shares stay
in the analysis for later measurements without recomputing.
