# River language design

Status: working draft. This is the shape we are converging on, not a spec.

**River is a signal-to-event language.** It describes how the fleet turns
streams (audio, RF/IQ, images, video, sensors) into events, how it judges those
events, and what it does in response. It compiles to WASM components and a
deployment plan, and it is written in a surface that a person and a constrained
model can both produce.

The pipeline is:

```text
prose / conversation
      ↓  (optional, big model)
Elixir-like surface          ← humans write and read this
      ↓  parse + typecheck + lint
S-expression IR              ← constrained-decoding target, canonical AST
      ↓  resolve + place
WASM component + plan + lock ← what the host executes
      ↓
Neo runtime                  ← placement, leases, judges, machines, events
      ↓
Elixir-like surface          ← pretty-printed view for review
```

Two surfaces, one semantics:

- **Elixir-like surface** for people and models. Pipes read like dataflow.
- **S-expression IR** for machines. The smallest grammar, the easiest thing to
  constrain-decode, the canonical artifact for the linter, node editor, and
  lock.

The surface is a view. The IR is the contract. The compiler is Rust. The target
is a WASM component with WIT imports, not Elixir or Lisp.

---

## 1. Why these choices

- **Elixir over Lisp for the surface:** `source |> fft |> detect |> event` reads
  like "and then," which is the mental model of a signal graph. `do/end`,
  keyword arguments, and pattern matching are readable to non-programmers,
  well-represented in model training data, and map directly to a node editor's
  edges.
- **Lisp over Elixir for the IR:** one reader rule, no precedence, no layout.
  Constrained decoding becomes "choose one of these valid symbols" instead of
  "write syntax." Homoiconicity makes the linter, pretty-printer, node editor,
  and rewrite rules operate on the same tree.
- **Neither is the implementation language:** both are syntaxes over the same
  typed dataflow. FFTs and math live in Rust/WASM modules; Lua is the escape
  hatch; the compiler is Rust.

Orb, Firefly, and Popcorn prove the Elixir-to-WASM direction, and they also show
where the boundary is. Orb is a kernel language (compile-time Elixir → tiny
WASM). Firefly is a full BEAM compiler. Popcorn is a full BEAM VM in the
browser. River should take the surface and the lowering discipline, not the
runtime.

---

## 2. Surface conventions

Reserved words: `pipeline`, `source`, `judge`, `scene`, `machine`, `state`,
`when_`, `unless`, `do_`, `hold`, `notify`, `ttl`, `budget`, `sees`, `tier`,
`threshold`, `fallback`, `hole`, `place`, `cost`, `safety`, `on`, `->`.

Conventions:

- `|>` is the only composition operator. No user-defined operators.
- Named arguments use `key: value`.
- Symbols are atoms (`:ais`, `:system1`).
- Blocks are `do … end`; no significant whitespace.
- Conditions are small expressions or named predicates, not a general language.
- A stage is a capability call with a closed vocabulary.
- `hole` marks a value the compiler fills: `?time`, `?comfortable`, `?due`.
- `ttl` and `budget` are lifecycle, not comments.

---

## 3. Signals → events

### AIS over RF

```elixir
pipeline :maritime do
  source :rf_hackrf0, iq(rate: 2_000_000, center: 161_975_000), lease: true
  |> fft(size: 1024)
  |> detect(mod: :ais)
  |> decode(:ais)
  |> event(type: :ais_message, confidence: true)
  |> publish(:ais, to: :panopticon)
end

judge :interesting_vessel do
  sees [:ais_message, :tracks_recent]
  tier :system1
  prompt "Is this vessel doing something worth surfacing?"
  threshold 0.8
  fallback rules("not in a known lane")
end

scene :watch_maritime do
  when_ :interesting_vessel
  do_ :panopticon, publish(:ais)
  notify when: urgency >= :high
  ttl months(6)
  budget 1/day
end
```

IR:

```lisp
(pipeline maritime
  (source rf-hackrf0 (iq :rate 2000000 :center 161975000) :lease)
  (-> (fft :size 1024)
      (detect :mod ais)
      (decode ais)
      (event :type ais-message :confidence true)
      (publish ais :to panopticon)))

(judge interesting-vessel
  :sees (ais-message tracks-recent)
  :tier system1
  :prompt "Is this vessel doing something worth surfacing?"
  :threshold 0.8
  :fallback (rules "not in a known lane"))

(scene watch-maritime
  (when interesting-vessel)
  (do panopticon (publish ais))
  (notify :when (>= urgency high))
  (ttl 6mo)
  (budget 1/day))
```

### Audio → speech events

```elixir
pipeline :speech_events do
  source :omi, audio(rate: 16_000, channels: 1)
  |> vad
  |> asr(model: :whisper_small, tier: :system1)
  |> event(type: :speech, confidence: true)
  |> publish(:speech, to: :records)
end
```

IR:

```lisp
(pipeline speech-events
  (source omi (audio :rate 16000 :channels 1))
  (-> vad
      (asr :model whisper-small :tier system1)
      (event :type speech :confidence true)
      (publish speech :to records)))
```

---

## 4. Events → signals, home automation, lights

The same language runs the other direction: events and judgments produce
signals, curves, and commands. Home Assistant-style automations are scenes;
Hue is a capability, not a vocabulary the judge has to know.

### Atmosphere

```elixir
pipeline :atmosphere do
  source :presence, presence(person: :avery)
  |> fuse(features: [:where, :activity, :attention], window: minutes(10))
  |> judge(:calm_moment,
      tier: :system1,
      prompt: "Is this a calm, available moment for a gentle atmosphere change?",
      threshold: 0.72,
      fallback: rules("attention != :busy and guests == 0"))
  |> act(:hue, atmosphere(intent: "calm romantic evening in fall"))
end

scene :fall_evening do
  when_ :calm_moment and after_sunset()
  unless_ :focus_session or :guests
  do_ :hue, scene(:warm_dim, transition: seconds(20))
  do_ :audio, play(mood: :jazz, volume: :low)
  hold minutes(5)
  ttl months(6)
  budget 1/day
end
```

IR:

```lisp
(pipeline atmosphere
  (source presence (presence :person avery))
  (-> (fuse :features (where activity attention) :window 10m)
      (judge calm-moment
        :tier system1
        :prompt "Is this a calm, available moment for a gentle atmosphere change?"
        :threshold 0.72
        :fallback (rules "attention != busy and guests == 0"))
      (act hue (atmosphere :intent "calm romantic evening in fall"))))

(scene fall-evening
  (when (and calm-moment (after-sunset)))
  (unless (or focus-session guests))
  (do hue (scene warm-dim :transition 20s))
  (do audio (play :mood jazz :volume low))
  (hold 5m)
  (ttl 6mo)
  (budget 1/day))
```

### Events → a light curve

Events do not always become one command. A light plan is a signal over time.

```elixir
pipeline :light_curve do
  source :events, bus(:atmosphere)
  |> map(fn e -> curve(e.palette, e.duration) end)
  |> hue(transition: :ease, tick: milliseconds(50))
end
```

IR:

```lisp
(pipeline light-curve
  (source events (bus atmosphere))
  (-> (map (lambda e (curve e.palette e.duration)))
      (hue :transition ease :tick 50ms)))
```

The Hue adapter owns palette names, physical limits, and transitions. The judge
says "calm fall evening"; the adapter decides what that means for these bulbs.

---

## 5. Machines and robotic control

Deterministic state machines and probabilistic judges live side by side.

```elixir
machine :pick_and_place do
  state :idle do
    on event(:object_detected, where: :table) -> :approach
  end

  state :approach do
    on judge(:graspable, tier: :system1, threshold: 0.8) -> :grasp
    on timeout(seconds(5)) -> :idle
  end

  state :grasp do
    do_ :arm, close(gripper: :soft, force: newtons(2))
    on ok() -> :lift
    on fault() -> :retreat
  end

  state :lift do
    do_ :arm, move(to: :tray, speed: meters_per_second(0.1))
    on ok() -> :release
    on fault() -> :retreat
  end

  state :release do
    do_ :arm, open(gripper: :soft)
    on ok() -> :idle
  end

  state :retreat do
    do_ :arm, move(to: :home, speed: meters_per_second(0.05))
    on ok() -> :idle
  end

  safety speed <= meters_per_second(0.2), force <= newtons(5)
end
```

IR:

```lisp
(machine pick-and-place
  (state idle
    (on (event object-detected :where table) -> approach))
  (state approach
    (on (judge graspable :tier system1 :threshold 0.8) -> grasp)
    (on (timeout 5s) -> idle))
  (state grasp
    (do arm (close :gripper soft :force 2N))
    (on ok -> lift)
    (on fault -> retreat))
  (state lift
    (do arm (move :to tray :speed 0.1m/s))
    (on ok -> release)
    (on fault -> retreat))
  (state release
    (do arm (open :gripper soft))
    (on ok -> idle))
  (state retreat
    (do arm (move :to home :speed 0.05m/s))
    (on ok -> idle))
  (safety (speed <= 0.2m/s) (force <= 5N)))
```

### Constrained control

The model does not emit torques. It emits a **typed action** from a small
grammar:

```text
action  ::= move(target, speed) | grasp(force) | release | wait(ms) | abort
target  ::= home | table | tray | waypoint(id)
speed   ::= 0..0.2 m/s
force   ::= 0..5 N
```

The compiler checks that the action is in the machine's current state, that the
target exists, and that the safety envelope holds. The runtime validates again
before the host applies it. Simulation and shadow mode run first.

This is the pattern for every risky actuator: **grammar → typecheck → safety
envelope → simulation → host apply → receipt.**

---

## 6. Types, effects, and placement

Implicit typing with compile-time checking, in a functional style. The type
system is small and structural:

```text
Stream<T, rate, clock, shape>
Event<T>
Judge<Inputs, Score, Reason>
Machine<State, Event, Action>
Effect { caps: ... }
Placement { prefer, avoid, latency, power, throughput }
```

Examples:

```elixir
source :rf_hackrf0 : IQ(rate: 2_000_000, center: 161_975_000)
fft   : Spectrum(n: 1024) <- IQ(rate: 2_000_000)
detect: Event(:ais_message, confidence: f32) <- Spectrum(n: 1024)
```

Compile-time checks:

- rate/shape/clock compatibility at every edge,
- event schema and payload types,
- exact capability/effect availability,
- exclusive resource conflicts (one HackRF, one tuner),
- placement feasibility and cost budget,
- judge threshold/cooldown/budget present,
- control actions inside the declared safety envelope,
- `ttl` and `budget` on every standing behavior.

Placement and cost are annotations, resolved by the compiler against Mycelium
facts:

```elixir
place :asr do
  prefer gpu(memory >= 8GB)
  avoid battery
  latency milliseconds(250)
  power watts(20)
  fallback :cpu
end
```

The language does not name hosts. It names constraints. The resolver picks the
node and pins it in the lock.

---

## 7. Sensor fusion as a small transformer

The judge layer is where a learned model belongs. The aggregator produces
features; the judge scores the moment.

A practical fusion model:

- **Tokenize** each channel into time-stamped tokens: RF spectrogram patches,
  log-mel audio frames, camera embeddings, BLE/GPS/heart-rate samples.
- **Window** 5–30 seconds; predict state (presence, activity, attention) or an
  event class with confidence.
- **Fuse** by late fusion or cross-attention over channel tokens; keep the
  model small enough for a phone or an edge TPU.
- **Calibrate** (Brier/ECE); a 0.7 must mean 0.7.
- **Cascade**: system-1 model on-device for the common path; system-2 model on
  the DGX only for ambiguity, novelty, or high-stakes decisions.

Training loop:

```text
synthetic episodes (RF propagation, room lighting, robot physics)
        + recorded episodes
        + human corrections (accepted / ignored / "warmer")
        ↓
   pretrain / fine-tune the fusion model
        ↓
   RLCD as judge: "is this event meaningful?" "is this plan good?"
        ↓
   distill into a smaller, faster scorer
        ↓
   shadow mode → canary → default
```

RLCD is not the generator here. It is the **scorer**: a fast local model that
answers a bounded question (keep/drop, meaningful/not, safe/unsafe) over a
feature window. It also decides pruning/renewal for memories and habits.

Why not just scale transformers?

- edge latency, power, and offline operation,
- privacy: raw audio/RF never leaves the home,
- determinism and auditability for control,
- cost per always-on stream.

Why not pure rules?

- presence, "calm," "worth interrupting," and "graspable" are not thresholds,
- sensor fusion across noisy modalities needs learned representations.

The bet is the hybrid: **deterministic state machines + small learned judges +
a big model for escalation.** If scaling wins on a given task, the judge
interface lets you swap the model without changing the language.

---

## 8. Constrained decoding

The model generates the **IR**, not the surface. That is the speedup.

- The grammar is generated from the capability registry: valid stages, event
  types, capabilities, schemas, judge tiers, hole policies.
- The IR grammar is LL(1): lists, atoms, keywords, a tiny expression form. No
  precedence, no layout.
- Identifier tokens come from a trie of known symbols. The model chooses among
  valid next symbols, not syntax.
- The compiler typechecks the result. Invalid programs are impossible to emit,
  and type errors are a separate, explainable pass.
- The surface is pretty-printed from the IR, and the round-trip is tested.
- A node editor consumes the same IR: node = stage, edge = pipe.

For control, constrain the **action grammar**, not the prose:

```text
move(target, speed) | grasp(force) | release | wait(ms) | abort
```

No free-form motor commands. The grammar plus the safety envelope is the
difference between "an agent that can move a robot" and "an agent that can
break a robot."

---

## 9. Runtime (Neo)

Neo executes the plan:

- resolves placement and acquires resource leases (shared vs exclusive),
- runs the signal graph as WASM components + host capabilities,
- evaluates judges and state machines,
- emits events to Unibus / records / Panopticon,
- hands durable steps to Elmers,
- enforces grants, approval, idempotency, and receipts,
- records provenance for every event and action.

Spores provide capabilities. Elmers provides durable consequences. Unibus
provides transport and QoS. Mycelium provides identity and placement. River
provides the program.

---

## 10. Prior art

- **Orb** (`useorb.dev`) — an Elixir macro DSL that emits WebAssembly
  instructions at compile time. Tiny modules, no Elixir runtime, no DOM, no
  WASI. Borrow: compile-time elaboration, composable tiny kernels, "Elixir is
  your compiler." Do not borrow: as the surface for graphs and judges. Orb is a
  kernel language.
- **Firefly** (`GetFirefly/firefly`) — a BEAM/Erlang → WASM compiler using
  MLIR/LLVM, with a layered IR: syntax → core → kernel → SSA → codegen. Borrow:
  the lowering discipline and pattern-match/closure-lifting stages. Do not
  borrow: the project itself (experimental, 2022-era) or full BEAM
  compatibility.
- **Popcorn** (`popcorn.swmansion.com`) — Erlang/OTP in WebAssembly, LiveView in
  the browser, offline, ~1 MB compressed. Borrow: an optional **app/UI runtime**
  for local-first LiveView and OTP state. Do not borrow: the sensor/DSP hot
  path; a full VM is the wrong weight for a kernel.

The shape this suggests: a **custom typed subset** with an Elixir-like surface,
compiled by Rust to WASM components; Orb-style compile-time kernels for small
DSP pieces; Popcorn only as an optional app runtime, never the signal path.

## 11. Open questions

1. Is the surface canonical for humans and the IR canonical for machines, with
   both stored and hash-checked? (Proposed: yes.)
2. How much type inference do we want before the language feels like work?
3. Do judges train per-person, per-home, or shared with a personal adapter?
4. Is the fusion transformer worth it versus engineered features for each
   domain? (Measure event precision and false-interrupt rate.)
5. What is the safety certification path for robotic control?
6. Does the node editor become a first-class surface, or a view of the IR?
7. Where does "Neo" end and "Pican" begin? (Runtime vs intelligence platform.)

---

## 12. First proof

Three verticals, one language:

1. **AIS / RF → Panopticon event.** Proves sources, shared/exclusive leases,
   DSP, eventization, provenance, placement.
2. **Hue atmosphere.** Proves presence fusion, a judge, a scene, TTL/budget, and
   events → signals.
3. **Robot pick-and-place in simulation.** Proves machines, constrained action
   decoding, safety envelope, and simulation-before-apply.

Measure: authoring time, compile errors, time-to-approve, event precision,
false-interrupt rate, latency, power. If those beat hand-wired Lua, River has
earned its place.
