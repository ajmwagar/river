# River syntax A/B/C — same intentions, three altitudes

Source: the intents in `example.river`. The question is not "which syntax is
prettiest" but **where the semantics live**: in prose the compiler interprets, in
light declarations, or in an explicit clause IR.

All three must express the same things regardless of syntax:

- **artifact type**: one-off · cycle · commitment · propensity · track · build
- **trigger**: clock · reality (presence) · opportunity (down time) · deadline
- **holes**: `?temperature`, `?time`, `?due_date` + fill policy
  (default · learned · ask · defer)
- **lifecycle**: `ttl`, strength, review
- **effect**: retention, approval, idempotency (host-owned)
- **gaps**: "this needs a capability that does not exist"

The syntax only decides *where* those are written down.

---

## A. Prose + generated gutter

Prose is canonical. The compiler infers artifact type and holes; the gutter is a
generated, collapsible annotation. This is closest to the current file.

```river
When I get home in the evening

I'd like the lights to turn on and a jazzy playlist to come on.

The TV should display a wind-down display: tomorrow's tasks, a nice quote, and
tomorrow's weather forecast.

Set the temperature to whatever feels right for the season.

---

I need to be better at remembering to send follow-ups. Make sure I don't forget
to send an email by 4 pm the day it's due.

---

I need to remember to file my Quarterly Business Taxes.
# This would need to be proactive and cause some level of development.
# Build the tax automation and spores. Not just pure River.

---

Let's try and keep track of calories/spending on takeout month over month.
Trying to lose weight.

---

I'd like to read more books.
```

Generated gutter:

```text
evening-arrival   habit · on presence->home · evening · ?temp learned · ttl 6mo
follow-ups        commitment · due=commitment.date · ?nudge default 16:00
taxes             build gap · no tax capability · deadline quarterly · escalate
takeout-spend     track · monthly · sources bank+receipts · goal hypothesis
reading           propensity · down_time>=20m · budget 1/day · ttl 6mo
```

**Strengths:** zero ceremony, natural to write and to draft with an LLM, closest
to "it's all just prose."
**Weaknesses:** highest ambiguity; artifact type and holes are invisible until
compiled; hard to hand-edit a precise value; the gutter can drift.

---

## B. Scene declarations (light structure)

A few anchors for the compiler, still readable as a document. The compiler may
insert these anchors itself, so the human never has to.

```river
scene evening-arrival
  when I get home in the evening
  want
    lights on, warm
    a jazzy playlist, low
    TV: a wind-down board — tomorrow's tasks, a quote, tomorrow's weather
    temperature ?comfortable
  ttl 6mo
  ?comfortable: learned seasonal

commitment follow-up-email
  due ?commitment.date
  at ?nudge on ?commitment.date
  want a gentle reminder to send the email
  ?nudge: default 16:00

build tax-filing
  due quarter.end
  want collect the quarter's transactions, draft the filing, ask before submitting
  gap no tax capability yet
  escalate 30d before due

track takeout-spend
  every month, from bank + receipts
  show the trend, not a score
  goal ?target: lose weight (hypothesis, review monthly)

propensity reading
  when down_time >= 20m and attention is available
  want suggest a book, or just prepare it
  budget 1/day
  ttl 6mo
```

**Strengths:** artifact type, holes, and lifecycle are visible; still reads like
prose; diffable and lintable; maps cleanly to one-off/cycle/propensity.
**Weaknesses:** a small keyword vocabulary to learn; the taxonomy is authored
unless the compiler inserts it; more lines than A.

---

## C. Clause IR (compile target)

Precise, generated, and inspected — not hand-authored. This is what the plan
locks against capabilities.

```river
habit evening-arrival
  on presence(avery) -> home
  when time == evening
  do lights.on(room=entry, tone=warm)
  do audio.play(mood=jazz, volume=low)
  do screen.show(wind-down(tasks=tomorrow, quote=inspirational, weather=tomorrow))
  do climate.set(?comfortable)
  fallback if screen.busy -> notify(phone)
  ttl 6mo
  ?comfortable: learned seasonal

commitment follow-up-email
  due ?commitment.due
  at ?nudge on ?commitment.due
  do notify("Send the follow-up")
  ?nudge: default 16:00

build tax-filing
  due quarter.end
  requires capability tax.file (missing)
  proposal collect(transactions) -> draft(filing) -> approve -> submit
  escalate 30d before due

track takeout-spend
  source bank.transactions + email.receipts
  window month
  do review(show=trend)
  goal ?target: lose weight (hypothesis, monthly)

propensity reading
  when down_time >= 20m and attention.available
  do suggest(book) | prepare(book)
  budget 1/day
  ttl 6mo
```

**Strengths:** unambiguous; maps to WIT/actions/records/unibus; easy to lint,
diff, simulate, and constrain-decode; holes and effects are first-class.
**Weaknesses:** reads like code; not where a human should sculpt; requires a
grammar and a resolver.

---

## Comparison

| | A prose | B scenes | C clauses |
|---|---|---|---|
| Human readability | best | good | fair |
| LLM draftability | best | good | needs grammar |
| Ambiguity | high | medium | low |
| Precision / diffs | low | medium | high |
| Lint / simulate | weak | medium | strong |
| Ceremony | none | light | heavy |
| Holes + TTL visible | gutter only | yes | yes |
| Maps to WIT/actions | indirect | indirect | direct |
| "I shouldn't have to matter" | yes | mostly | no |

---

## Recommendation: they are altitudes, not rivals

- **A is the canonical source.** Humans and LLMs write prose. No ceremony.
- **B is precision mode.** The compiler inserts anchors (`scene`, `when`, `want`,
  `ttl`, `?hole`) when prose is ambiguous; the human can turn them on for a
  block and ignore them elsewhere.
- **C is the compiled IR.** Never hand-authored. Inspected in the right pane,
  diffed, simulated, and constrained-decoded from A/B.

The editor is two-pane: A/B on the left, C on the right, holes highlighted, and
a line-level diff for every conversational edit. That gives prose authoring,
precise review, and a guaranteed compile target.

## Test plan

1. Take 10 intents from `example.river`.
2. Compile the same prose into each syntax with the same model and grammar.
3. Measure: valid first pass, number of holes, contradictions, human edits,
   diff size, time-to-approve.
4. Repeat with a small distilled model.

Prediction: **B wins first-pass validity, A wins drafting speed, C wins review.**
The hybrid is the answer; the experiment tells us where to put the anchors.
