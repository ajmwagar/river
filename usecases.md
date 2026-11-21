# River use cases

Status: working list. These are the tests the language has to pass, not a
roadmap. If a use case needs a different shape, that is a signal that River is
wrong, not that the use case is special.

The common shape is:

```text
source → transform → detect → event → judge → scene/machine → action
```

The variation is in the rate, the latency, the durability, the placement, and
who pays attention. Those are the axes the language has to expose.

---

## 1. SDR / RF + signal fusion → events and state machines

**Jev-like:** signals become events, events become tracks, tracks become a state
machine with confidence and provenance.

- **Sources:** RTL-SDR, HackRF IQ, AIS/ADS-B decoders, RF fingerprint embeddings,
  TDoA, recorded IQ.
- **Events:** emitter detected, protocol message, vessel/aircraft track,
  anomaly, frequency occupancy.
- **Judges:** interesting, anomalous, same-emitter, track association.
- **Machines:** candidate → confirmed → lost; retune; hand off between radios.
- **Actions:** publish to Panopticon, alert, record IQ, retune, fuse with a feed.
- **Stresses:** exclusive vs shared radios, sample-rate streams, DSP placement
  (CPU/GPU/FPGA), event time, false positives, provenance.

**Fit:** strongest. This is the reference vertical.

---

## 2. Hue / home automation

Home Assistant-style automations, but with presence, judgment, and TTL.

- **Sources:** presence, BLE (ring/watch/phone), calendar, light level, audio,
  contact sensors.
- **Events:** arrival, departure, calm moment, sleep, guest, focus session.
- **Judges:** calm/available, atmosphere, “is this worth interrupting?”
- **Actions:** lights, music, climate, TV, notifications.
- **Stresses:** events → signals, TTL/budget, consent, low latency, fallback,
  no guilt, cross-device consistency.

**Fit:** strong. This proves judges and scenes, and it is where the “gentle”
requirement is non-negotiable.

---

## 3. Instinctual / conscious agents

Ways to understand a lot of signals and carry that context while reading an
email or a text.

- **Sources:** email, SMS/chat, notifications, transcripts, calendar, documents.
- **Events:** commitment, question, deadline, thread, person, urgency.
- **Judges:** what matters now, interruptibility, reply/ignore/defer.
- **Actions:** summarize, remind, draft, surface in Canvas, ask.
- **Stresses:** memory and TTL, privacy, system-1 vs system-2 tiering, context
  selection, explainability, “read like an email” UI.

**Fit:** strong, but this is the memory/judge layer more than the DSP layer. It
is the use case that makes the small-model flywheel worth training.

---

## 4. Computer / browser / tiling WM control

“Show me things” — the agent acts on the machine you are looking at.

- **Sources:** window state, browser tabs, screen, clipboard, current app,
  agent intent.
- **Events:** “show me X,” focus change, layout request, cast request.
- **Judges:** which view/target, ambiguity, is this reversible?
- **Actions:** `canvasctl`/`canvas-fleet`, WM commands, open/arrange windows,
  open a URL, cast to TV.
- **Stresses:** constrained action grammar, reversibility, safety, latency,
  accessibility APIs, human takeover, one state two actors.

**Fit:** strong for the control part. This is the best proof of constrained
decoding for actions: the model chooses from `present`, `focus`, `arrange`,
`open`, never raw compositor commands.

---

## 5. Audio work / VST authoring — maybe

- **Sources:** audio, MIDI, DAW state, parameter automation.
- **Events:** notes, gestures, sections, parameter targets.
- **Judges:** musical intent, “warmer,” “less attack.”
- **Actions:** synth parameters, patch generation, routing, render.
- **Stresses:** real-time, determinism, sample accuracy, plugin sandboxing,
  latency.

**Fit:** partial. The audio callback is not River. River orchestrates patches,
presets, render jobs, and analysis; DSP kernels are Rust/WASM, and VST hosting
is a native adapter. Worth keeping as a target, not a first proof.

---

## 6. TTS / STT pipelines

- **Sources:** mic, Omi, text, files.
- **Events:** speech segment, speaker, wake, intent, end-of-turn.
- **Judges:** is this addressed to me? is it urgent? which voice?
- **Actions:** speak, caption, route to a device, store a transcript.
- **Stresses:** streaming latency, privacy, voice identity, device placement,
  interruption, offline.

**Fit:** strong. Same eventization as RF, different sensor.

---

## 7. Finetuning / training pipelines

River orchestrating its own flywheel.

- **Sources:** transcript/correction datasets, eval sets, model registry,
  checkpoints.
- **Events:** job state, metric, checkpoint, regression, eval gate.
- **Judges:** quality, safety, cost, “is this better than last-known-good?”
- **Actions:** launch, stop, promote, roll back, canary, publish.
- **Stresses:** long-running jobs, GPU placement, power/cost, durability,
  artifact CIDs, reproducibility, eval gates.

**Fit:** strong. Elmers handles the durable job, River declares the pipeline and
the gates, and the small model is the output.

---

## 8. Video / encoding / CV

- **Sources:** RTSP cameras, files, screen capture, drone feed.
- **Events:** object, scene, anomaly, segment, face, motion.
- **Judges:** is this notable? is this the same object? is this private?
- **Actions:** record, transcode, alert, present, retain/delete.
- **Stresses:** bandwidth, GPU, data gravity, retention, privacy, real-time.

**Fit:** strong. The camera/NVR/TPU stack is a natural River vertical.

---

## 9. Sensor fusion

- **Sources:** BLE, GPS, IMU, audio, cameras, calendar.
- **Events:** presence, activity, attention, confidence, transition.
- **Judges:** fused state, calibration, missing modality.
- **Actions:** scenes, suggestions, alerts.
- **Stresses:** uncertainty, missing data, edge power, calibration, privacy.

**Fit:** core. The judge layer is the product.

---

## 10. IoT

- **Sources:** MQTT, Zigbee/Matter, BLE, LoRa/mesh, HTTP webhooks.
- **Events:** state change, telemetry, fault, battery, connectivity.
- **Judges:** normal/anomalous, “does this need a person?”
- **Actions:** control, scene, notify, schedule.
- **Stresses:** protocol adapters, offline, idempotency, device identity,
  security, vendor quirks.

**Fit:** strong. The adapters are capabilities; River does not learn each vendor.

---

## Also on the horizon

- **Robotics / motion control.** Constrained action grammar, safety envelope,
  simulation-before-apply. Covered in `language-design.md`.
- **E-ink reader / EPUB / recipes.** A Spore, not a River vertical; useful as a
  sink for the “read more” propensity.

---

## How they differ

| Use case | Rate | Latency | Durability | Placement | Risk |
|---|---|---|---|---|---|
| RF / SDR | very high | ms | low–medium | edge + GPU | medium |
| Hue / home | low | ms–s | low | local | low |
| Email / text agents | low | s | high (memory) | cloud/local | medium |
| Computer control | medium | ms | low | local | medium |
| Audio / VST | very high | sub-ms–ms | low | local | low |
| TTS / STT | high | 100s ms | low | local/edge | medium |
| Training | low | hours | high | GPU | high (cost) |
| Video / CV | high | ms–s | medium | GPU/edge | medium |
| Sensor fusion | medium | ms–s | medium | edge | medium |
| IoT | low | ms–s | medium | local | high (security) |

That table is the argument for stream classes, placement constraints, and SLOs
in the language. A single “pipeline” abstraction cannot serve sub-millisecond
audio and a three-day training job without those axes.

## What they share

- **Eventization is the product.** Agents consume events, not raw samples.
- **Judges are small and local first.** System-1 for the common path; system-2
  for escalation.
- **Actions are host-mediated and constrained.** The model proposes; the grammar
  and the safety envelope dispose.
- **Placement is a first-class decision.** Latency, bandwidth, power, data
  gravity, durability.
- **Memory and TTL are part of every use case**, not just the assistant.
- **Privacy and consent are host law**, never the language.

## First proofs

1. **RF / AIS → Panopticon event.** The reference: source, DSP, event, judge,
   provenance, placement.
2. **Hue atmosphere.** Judge + scene + TTL + events → signals.
3. **Computer control: “show me X.”** Constrained action grammar, reversibility,
   human takeover.
4. **Training pipeline as the meta-proof.** River orchestrates the flywheel that
   produces its own small model.

If those four work, the rest are variations on the same shape. If one of them
needs a fundamentally different construct, that is the next design decision.
