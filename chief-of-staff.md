# Chief of staff

Status: working draft. The loop, not the platform. If this reads like a
dashboard, it is wrong.

## The job

One **System One** surface that reduces decisions and protects attention. It
holds the threads so you do not have to. It dispatches System Two work; it does
not do the work.

- **System One** = the chief of staff: fast, associative, local, mostly silent.
  Habits, defaults, presence, gentle nudges.
- **System Two** = Edgerunner / the Software Factory, the nightly consolidation,
  and the rare deliberate check. Slow, expensive, off the hot path.

The goal is not a smarter assistant. It is a quieter one.

## One surface

**One place for decisions.** One inbox. One morning intent. One evening
shutdown. No parallel status channels, no second dashboard, no “just checking
in.”

Everything else is a sink behind it: notifications, calendar, email, Beeper,
shells, MRs, metrics. If it needs a decision, it comes to the one surface. If it
does not, it stays silent.

## The four loops

### 1. Morning intent

**System One output:** two or three things that actually matter today, chosen
from projects and commitments. Not a backlog, not a score.

- Pull from open loops, deadlines, and the last shutdown.
- Propose a default; do not ask an open question.
- One line per item: what, why now, first step.
- System Two only if a choice is genuinely ambiguous or expensive.

### 2. Focus guard

**System One output:** attention protection.

- Presence + attention: at the bench, in a call, asleep, away.
- Batch interruptions; answer the routine ones; hold the rest.
- Break through only for a decision or a deadline.
- Learn the difference between “urgent” and “loud.”

### 3. Evening shutdown

**System One output:** closed loops and a clean stop.

- Capture what is open; schedule the next step; close what is done.
- Reconcile the calendar with reality: the calendar is the loser.
- One question: “anything I should carry tomorrow?”
- Then stop. No evening dashboard.

**System Two (background):** consolidate the day — memories to semantic,
repeated patterns to habit candidates, stale rules to decay, build gaps to
proposals.

### 4. Weekly review

**System One output:** a short list, not a report.

- Stalled projects with one suggested next action each.
- Habits that are working, habits that are not.
- One thing to stop doing.
- Five items maximum; if it is longer, the TTLs are wrong.

## System One / System Two routing

“As much System One as possible” is a routing policy, not a vibe. Make it
explicit:

```text
route(risk, reversibility, confidence, latency, novelty, cost)
  -> act | act+audit | propose | ask | defer | gate
```

|  | Reversible | Irreversible |
|---|---|---|
| **Low risk** | act, audit asynchronously | propose, then gate |
| **High risk** | act with an undo window, audit | gate + human approval |

Patterns:

- **Sequential gate** — System Two runs before the action. Use when a wrong
  action costs more than waiting: money, messages to humans, locks, deletes,
  publishing.
- **Speculative / parallel** — System One acts immediately; System Two audits in
  the background and can undo or compensate. Use when the action is reversible
  and latency matters. This is the default for daily life.
- **Escalation** — System One handles the common path; System Two is invoked on
  low confidence, a novel context, or a named trigger.
- **Deferred / batch** — the decision is not time-sensitive; the nightly or
  weekly pass handles it.
- **Second opinion** — both run independently; an arbiter compares. Expensive;
  reserve it for high-stakes, genuinely uncertain calls.

**Find the commit point.** Before it, parallel and speculative are safe. After
it, you need a gate. If you cannot name the commit point, the action is not
ready to automate.

**System Two is an offline compiler for System One.** Its best output is not a
decision; it is a new default, habit, or rule that makes the next decision
unnecessary. Cache the conclusion, then promote it only when it proves itself.

## Edgerunner contract

Edgerunner is the factory. The chief of staff is the dispatcher.

```text
dispatch:
  outcome        "ship the fix", "draft the quote", "file the taxes"
  constraints    deadline, budget, safety, approvals
  receipts       one line + the decision you owe
```

Rules:

- Never narrate the factory. One line out.
- Never re-plan what the factory already planned.
- Escalate to the human only for ambiguity, irreversible effects, or money.
- Every dispatch produces a receipt and a new memory.

## KPIs

Directional, not a scoreboard:

- **Interruptions per day** — down.
- **Decisions surfaced per day** — down; the ones left are real.
- **Focus blocks per week** — up.
- **Open loops closed per week** — up.
- **Time to close a loop** — down.
- **System-thought** — how often you had to think about the system itself. Down.
- **False-interrupt rate** — down.
- **Acceptance rate of defaults** — up; if it is low, the defaults are wrong.

If the system-thought KPI goes up, the chief of staff has become a System Two
tool wearing a System One costume.

## Non-goals

- Not a dashboard, chat UI, or agent console.
- Not always-on voice; voice is one surface, not the product.
- Not autonomy without consent.
- Not a replacement for Edgerunner, Elmers, or the Spores.
- Not a place to browse the fleet.

## Open questions

1. Which surface first: Beeper, phone, or voice in the shop?
2. Where do memories live: `soil:records`, local Markdown, or both?
3. What is the first thing to promote from System Two to System One?
4. What is the commit point for money, messages, and publishing?
5. How do we measure “gentle” without turning it into a metric?
