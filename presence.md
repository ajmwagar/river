# Presence

Status: working draft. This is the **worldview pipeline**, not the chief-of-staff.
It is the substrate the chief-of-staff, JARVIS, Hue, notifications, and the
training flywheel all consume.

## Why presence first

Presence answers the questions every other behavior depends on:

- who is here,
- where they are (site → building → room),
- what they are wearing and what is charging,
- what they are doing and whether they are interruptible,
- which output should carry a message.

Build this before the agent loop. A chief-of-staff without presence is guessing.

## What already exists in JARVIS

This draft is a layer above the existing presence code, not a second presence
implementation.

- `jarvis-core::presence_v2` — `PresenceMessageKind` (`presence.v2.observation`,
  `.tracklet`, `.source_health`, `.projection`, `.calibration_command`),
  `PresenceQos` (`ReplaceableStream`, `LatestValue`, `ReliableIdempotent`),
  `PresenceObservationV2`, `SensorTracklet`, `PresenceProjection`,
  `PresenceSourceHealth`, `PresenceModality` (camera, face, voice, WiFi CSI,
  device, manual), and `ObservationKind` (detection, occupancy, identity
  evidence, negative evidence).
- `jarvis-core` — `PresenceSnapshot` (zone, occupied, identities, confidence,
  recency, privacy), `PresenceIdentity` / `PresenceIdentitySource` (face, voice,
  device, manual, fused), `PresencePrivacyContext` / `DisclosureTact`, and
  `PresenceTransitionFact` / `PresenceTransitionKind` (zone occupied/empty,
  person arrived/left, room private/shared, unknown person).
- `presence_place` — `PlaceId`, `PlaceKind` (site/building/floor/room/zone),
  `CoordinateFrame`, `SensorRegistration`, `PlaceRegistry` with revisions.
- `jarvis-presence` — `IdentityFusion` (zone-local, face+voice, 5s max age,
  0.65 identity threshold), `PublishedPresence` v1 (atomic, 5s expiry; stale
  clears identities and reserves disclosure), `PresenceSourceAdapter` /
  `PresenceSourceBatch`, `IdentityLatch`, Frigate/RuView adapters.
- `docs/rlcd-system-one-api.md` — bounded System One choices and confidence;
  Rust owns execution.
- Canvas `presence_route` — `(producer, zone) → display`, placement only.
- Existing contract: stale/expired is unknown, anonymous occupancy is not
  identity, presence selects placement not authority.

### What is new here

| New | Why |
|---|---|
| A person-centric world state across zones | JARVIS presence is zone-local and place-centric |
| Wearable / charging state | The snapshot has device identity, not worn/charging |
| A delivery router | Presence publishes state; Sound/Subconscious chooses output |
| Snapshot + replay for world state | v1 publication is a 5s snapshot, not a replayable log |
| Artifact TTL / decay | Presence event TTL is 30s; habits and monitors live longer |

## Three layers

| Layer | Question | Shape | Home |
|---|---|---|---|
| **State** | what is happening now? | latest value, confidence, `since`, evidence | in-memory + snapshot |
| **Event** | what happened? | immutable, typed, namespaced | Unibus |
| **Record** | what do we keep? | durable, queryable, schema-tagged | records / VFS |

State is a **fold over observations and events**, with periodic snapshots.
Reconstruction is snapshot + replay. That is event sourcing, not a monad; a typed
reducer is enough.

```text
state = reduce(snapshot, events_since(snapshot))
```

Derivatives and integrals are derived state: rate of change, dwell time,
confidence decay, “in room for 20m.” They can be published as state or events.

## State schema

JARVIS already has two state shapes: `PresenceSnapshot` is zone-local
(`occupied`, `observed_identities`, `confidence`, `recency_ms`, `privacy`), and
`PresenceProjection` is place-centric (`tracks`, `identity` beliefs, validity).
The person-centric state below is the **new world layer** that folds those plus
device observations.

```text
presence/person/avery
  place:        home
  room:         studio
  confidence:   0.86
  since:        2026-11-20T18:10:00Z
  evidence:     [phone.gps, ring.ble, camera.studio]
  wearables:
    ring:       worn        last_motion: 30s
    watch:      charging    deprioritized until motion
  attention:    focused
  updated_at:   2026-11-20T18:10:05Z
```

Rules:

- **Charging is negative evidence.** A device on the charger is not worn.
  Deprioritize its signal until recent motion or a reconnect proves it is back.
  Do not ignore it forever; do not treat silence as absence.
- **Worn** means recent motion/HR/activity, not merely connected.
- **Stale is unknown**, not empty. An expired sample clears identity and
  reserves disclosure.
- **Identity fusion** only labels a person when face and voice agree on the same
  person in the same zone. Conflicting evidence stays separate.
- **Presence selects placement and delivery, never authority.** Speaker
  recognition and grants still decide what may be done.

## Events

JARVIS already has the lifecycle vocabulary. `PresenceTransitionKind` is
`zone_became_occupied`, `zone_became_empty`, `person_arrived`, `person_left`,
`room_became_private`, `room_became_shared`, `unknown_person_present`. Use it;
do not invent a parallel `presence.person.arrived.v1` family.

Transport already exists:

- `presence.v2.observation`, `.tracklet`, `.source_health` — replaceable stream.
- `presence.v2.projection` — latest value per place.
- `presence.v2.calibration_command` — reliable idempotent.

Add only what is missing:

- `presence.v2.transition` for the derived `PresenceTransitionFact` (or publish
  the fact under the existing transition contract).
- `presence.v2.wearable` — or extend `PresenceSnapshot` / `PresenceProjection`
  with device state — for `worn | charging | offline`.

Producers publish evidence; the tracker owns tracks, identity beliefs, and
projections; handlers subscribe to projections and transitions. A pipeline that
both produces and handles is still an anti-pattern: multiple handlers may need
the event, and handlers need queueing, retries, health, and fault tolerance.

Deliberately: an allowlist of kinds, not “listen to everything.” Abstractions
must not hide the raw signal from a handler that needs it.

## Delivery is System One — but not presence's job

Presence publishes state and transitions. The delivery router belongs to
Sound/Subconscious (the existing push-gate); it consumes `PresenceSnapshot` +
`PresenceProjection` and uses `rlcd-system-one-api` for bounded choices.

```text
delivery.route(urgency, person, context)
  -> phone | tts(zone, duck) | canvas(display) | hold
```

Policy examples:

- Ring/watch worn, music playing, not recording → **TTS, duck the music**.
- Phone in hand → **notification**.
- At the desk → **Canvas**.
- Asleep or in a call → **hold**.
- Others present → **reserved** (group-context tact).
- Charging device → not evidence of presence, not an output target.

Use the existing `rlcd-system-one-api` boundary: bounded questions, typed
choices, confidence. Rust owns the gate, idempotency, ordering, and delivery.
A generative model is used only when the request cannot be represented by the
bounded questions.

## State, reconstruction, derivatives

- Live state lives in the presence daemon's memory; snapshots go to records.
- Events are the durable log; a snapshot bounds replay.
- On restart: load the last snapshot, replay events since, then resume.
- If the snapshot is lost: replay from the last durable record; if that is lost,
  fall back to unknown rather than inventing state.
- Derivatives are computed over the state window and published as derived state
  or events; they are not a second source of truth.
- State is “what is happening”; an event is “something happened”; a record is
  “what we keep.”

## Memory decay

Every artifact gets a soft TTL; review is batched.

| Artifact | TTL |
|---|---|
| Live state | seconds–minutes, refreshed |
| Room presence events | hours–days |
| Sports/schedule monitor | ~2 weeks |
| Habits / preferences | months, reviewed |
| Legal / tax | explicit legal hold, cold storage |
| Raw signals | sampled/summarized, short |

The UW game monitor is the first test: create it with a two-week TTL and verify
it decays. Routine data expires; legal data goes to cold storage; raw signals are
not kept forever.

A System One disk-pressure event (`host.disk.pressure.v1`) should trigger
pruning before the disk is full, using host and fleet metrics.

## Constrained decoding is modal

1. **Reason in English** — explore, explain, ask.
2. **Emit constrained** — switch on the grammar when writing River, Rust, or Lua.

Measure **tokens per verifiable artifact** and **compile-error rate**, not raw
TPS. A syntax error that wastes a `cargo check` is the cost constrained decoding
removes; semantic and type errors remain.

## First slice

1. Extend `PresenceSnapshot` / `PresenceProjection` with **wearable/charging**
   state, or add `presence.v2.wearable` without forking `presence_v2`.
2. Publish **`PresenceTransitionFact`** for lifecycle changes; add
   `presence.v2.transition` if the existing contract has no wire kind.
3. Add the **delivery router** in Sound/Subconscious over `rlcd-system-one-api`.
4. Add **snapshot + replay** for the person-centric world state.
5. Ship the **UW game monitor** with a two-week TTL as the decay test; it is a
   River/records artifact, not a presence event.
6. Add the **daily review**: seen vs flagged vs mattered; labels for the gate.

Acceptance criteria:

- stale presence never reads as empty or occupied,
- a charging device is not used as evidence or as an output,
- a message goes to the right output without asking,
- the game monitor disappears after two weeks unless renewed,
- the system can explain why it routed a message to a given output,
- cost per useful event is measured, not estimated.

## Open questions

1. Does the person-centric reducer live in `jarvis-core`, `jarvis-presence`, or
   a new `world` daemon?
2. Is wearable state a field on `PresenceSnapshot` / `PresenceProjection`, or a
   new `presence.v2.wearable` kind?
3. Who owns delivery policy: Sound or Subconscious?
4. What is the snapshot cadence and where do snapshots live: the v1 publication,
   records, or VFS?
5. What is the first wearable signal source: ring BLE, watch, or phone charging?
6. How does this reconcile with `presence_v2` without forking it?
