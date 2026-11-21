# River training flywheel

Status: working draft. This is the loop that makes a small model good enough to
run the language, without pretending the big model is the runtime.

## The thesis

The grammar guarantees validity. The small model learns **semantics and
vocabulary** — your rooms, people, devices, projects, phrasing, and taste. The
big model is the teacher and the data collector; the small model is the runtime.

```text
conversation → proposal → human edit → accepted artifact
      ↑                                      │
      └──── capture, curate, train, eval ◄────┘
```

The authoring UX and the training capture surface are the same product. Every
correction is a label.

## What to save

One episode per interaction:

```text
episode
  transcript
  context        # presence, time, devices, registry version, schema version
  proposal       # surface + IR + holes + diagnostics
  edits          # structured diff, human corrections
  accepted       # surface + IR + lock + hash
  outcome        # ran, latency, errors, user feedback
```

Store it locally, host-owned, with retention and redaction. Transcripts are the
most sensitive data in the system; training on them without consent is how the
whole thing becomes creepy.

## Targets

| Target | Label | Use |
|---|---|---|
| Prose + context → accepted **surface** | positive | SFT, distillation |
| Accepted surface → **IR** | structural | round-trip, typecheck |
| Proposal → edited surface | preference pair | DPO / RLCD-style ranking |
| Compile/lint error | negative | repair training, judge |
| Judge verdict | accepted / ignored / corrected | judge + pruner |
| Ran + outcome | success/failure | eval, reward |

The **edit trace** is the highest-value signal. The final artifact is weak; the
delta between proposed and accepted is the lesson.

## Training stages

1. **Big model + constrained decoding.** No training. Collect real episodes.
2. **SFT** on accepted surface + IR pairs.
3. **Preference tuning** on proposed → edited pairs.
4. **Train the judge** on “is this faithful to the prose?” and “is this memory
   still wanted?”
5. **Distill** the big model into a small compiler.
6. **Canary** the small model behind the grammar; keep the big model as fallback.

Do not start at 2. The schema will move, and you will train on a grammar you are
about to change.

## Surface vs IR

Train on the **committed surface**, with IR as a structural label. The surface
is what a human reviewed and what git holds. The IR is the check that it
typechecks. Keep both, aligned by a round-trip test.

If the model emits IR and a printer renders the surface, the printer has to be
lossless: comments, holes, names, and formatting must survive. Otherwise the
committed artifact is not the thing anyone reviewed.

## Synthetic data

Synthetic is for **coverage**, never for truth:

- paraphrases and ambiguity,
- contradictions and conflicts,
- prompt injection and adversarial asks,
- edge cases and missing capabilities,
- empty/denied/offline states.

Real corrections are the only ground truth. A synthetic corpus teaches the model
your idea of River; your edits teach it your life.

## Privacy and licensing

- Local-first: prefer on-device or home-host training; no raw transcripts to a
  third party without explicit consent.
- Redact secrets, health, location, and third-party content.
- Keep provenance per episode: who, when, model, prompt, schema.
- Per-household adapters (LoRA or small fine-tune) are a legitimate design;
  a global model trained on private transcripts is not the default.
- Respect the licences of any code or text used as synthetic seeds.

## Evaluation

- **TTVA**: time to a valid, parseable artifact.
- **Acceptance rate**: proposed → accepted without edits.
- **Edit distance**: how much a human changed.
- **Compile/lint errors per artifact.**
- **Task success**: did the scene/flow do the right thing?
- **False-interrupt rate** for judges.
- **Latency and cost** per artifact.

Hold out by **time, person, and scene**, not a random split. The model must
generalize to a new week and a new context, not memorize the training day.

## Risks

- **Overfitting the current schema.** Version the grammar and retrain on
  migration; keep a general eval.
- **Catastrophic forgetting.** Keep a broad instruction set and a canary.
- **Learning the wrong lesson from edits.** A correction may be a one-off, not a
  preference. Weight repeated corrections.
- **Training too early.** The biggest waste is a model trained on a schema that
  changes next month.
- **Silent privacy loss.** Capture is easy; consent and deletion are the hard
  parts.

## The loop

```text
capture → curate → train → eval → canary → default
   ↑                                          │
   └────────────── corrections ◄──────────────┘
```

The moat is not the base model. It is the closed vocabulary, the grammar, the
corrections, and the fact that the small model knows **you**. The big model can
be swapped; the flywheel cannot.
