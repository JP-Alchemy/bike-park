# Architecture

How the code is organised, the rules that keep it deterministic, and what the prototype
has measured so far against the tech proposals in the [design document](game-design-document.md#tech-approach).

## The pieces

```text
crates/sim        bike-sim: the deterministic simulation (Rust, Rapier 2D)
                  ├─ physics.rs   Rapier world, collision groups, terrain collider
                  ├─ rig.rs       the bike: chassis, sprung wheels, sprung rider, motors
                  ├─ rider.rs     riding pose (IK), ragdoll after a crash
                  ├─ scoring.rs   wheelies, flips, named tricks, combos (pure logic)
                  ├─ world.rs     Sim: riders, tick, crashes, respawns, checkpoints, HUD data
                  ├─ track.rs     track format + builder, the "Proving Grounds" park
                  ├─ tuning.rs    every number that shapes the feel, per bike class
                  ├─ bot.rs       bots that fill quiet rooms
                  └─ check.rs     shared scenario for cross-platform determinism checks
crates/sim-wasm   wasm-bindgen wrapper the browser uses (thin: no game logic)
web/              browser client: Vite + TypeScript + Three.js
                  ├─ sim/         fixed-tick driver over the wasm, render layout, JSON types
                  ├─ render/      procedural low-poly bikes, riders, park, camera, dust
                  ├─ input/       keyboard, touch, gamepad → one set of controls
                  ├─ hud/         DOM heads-up display
                  ├─ audio/       synthesised sound (no audio files)
                  └─ debug/       live tuning panel
scripts/          build-wasm.mjs, check-determinism.mjs
```

The server does not exist yet. It will be a third crate that runs the same `bike-sim` for
many rooms; nothing in `bike-sim` depends on the browser.

## One simulation, two platforms

`Sim` advances in fixed ticks of 1/60 s. Given the same starting state and the same inputs
per tick, it produces bit-identical results on every platform. That is what lets the
browser predict its own bike and the server stay authoritative without the two drifting
apart.

Rules that keep it that way (break one and `scripts/check-determinism.mjs` fails):

1. **Rapier with `enhanced-determinism`.** Rapier then uses pure-Rust math internally and
   no SIMD or multithreading.
2. **Trig through `bike_sim::math`** (`sin`, `cos`, `atan2`, …), which wraps the `libm`
   crate. `f32::sin` calls the platform's libm, which may differ in the last bit between
   x86 Linux and WebAssembly. `sqrt`, `+`, `-`, `*`, `/` are exact everywhere and are fine.
   Avoid glam helpers that use trig internally (`Vec2::from_angle`, `angle_to`, …).
3. **Inputs are quantized** to bytes (`Input`, 4 bytes) before they reach the simulation.
4. **Stable iteration order.** Riders live in a `Vec` in join order; no `HashMap` iteration
   in the simulation.
5. **Seeded randomness only.** Bots use their own xorshift generator seeded from their id.

Two checks enforce this: `tests/determinism.rs` (same inputs, same state, every tick) and
`scripts/check-determinism.mjs`, which runs a 60-second scenario with four riders, crashes,
ragdolls and respawns natively and in WebAssembly and compares the final state hash.

## The bike

```text
           rider body (hips, torso, head) ── 2-axis spring: lean shift + legs
                │
  rear wheel ── chassis ── front wheel      each wheel: pin-slot joint = spring
                                            along the suspension axis, free spin,
                                            angular motor for drive and brakes
```

- **Suspension** is a Rapier pin-slot joint per wheel: the wheel slides along the
  suspension axis on a force-based spring-damper (preloaded so the bike rests at its design
  pose) and spins freely.
- **Drive and brakes** are the same joint's angular motor: "hold this wheel speed, using at
  most this much torque". Throttle targets top speed with a flat-torque, then
  constant-power curve (an electric motor); braking targets zero relative spin. Because the
  motor pushes the wheel and the frame equally and oppositely, throttle lifts the nose,
  braking drops it, and wheelies emerge from the physics.
- **Lean** shifts the rider's hips along the bike (real weight transfer) and adds a torque:
  a direct push on the ground (plus light pitch damping while balancing a wheelie), and in
  the air a spin-rate controller, so flips are controllable and releasing the lean keeps
  the spin going.
- **Crashes**: the rider's head or torso touching the ground, landing mid-trick, lying
  upside down, or falling out of the world. The rider body is swapped for a 9-body ragdoll
  that starts in the same pose at the same velocity; after `respawn_delay` the rider
  reappears at their last checkpoint. Riders stuck with the throttle held are respawned
  without a crash.

Every number lives in `tuning.rs` (`BikeTuning`, `RiderTuning`, `WorldTuning`) as plain,
serialisable data. The tuning panel in the browser edits the same structures live.

### Scoring

`scoring.rs` is a pure state machine fed one observation per tick, so it is unit-tested
without physics:

- Wheelie or stoppie: distance scored with up to 2× style for riding near the tipping point
  (the rig's centre of mass over the wheel).
- Flips: counted from the bike's rotation between take-off and landing (40° of slack).
- Named tricks: the trick button in the air; the held direction picks No-Hander (none),
  Superman (up), Can-Can (down) or Tail-Whip (lean). Landing before the rider is back on the
  bike is a bail.
- Landed tricks join a combo (multiplier = number of tricks). The combo banks after
  `combo_window` seconds on the ground; a crash loses it. Tricks only count if the rider
  rides away from the landing.

## Browser client

- **Loop:** each animation frame reads the controls, runs as many fixed ticks as real time
  allows (feeding the local rider's input each tick), then renders the state interpolated
  between the last two ticks. This keeps the physics identical at any display refresh rate.
- **Across the wasm boundary:** per-frame render state is one `Float32Array`
  (`RENDER_STRIDE` floats per rider; layout in `world.rs::render_layout` and mirrored in
  `web/src/sim/layout.ts`). HUD data, events and tuning cross as JSON, a few times per frame
  at most.
- **Rendering:** Three.js with flat-shaded Lambert materials; bikes and riders are built
  from primitives in bike space from the same tuning data, riders are posed from the
  simulation's skeleton points (IK while riding, ragdoll bodies after a crash). The park is
  extruded from the ground profile. Adaptive quality drops shadows, then resolution, if the
  frame rate stays under 40 fps.
- **Test hooks:** `window.bikePark` exposes the simulation, and
  `bikePark.autopilot = (hud) => controls` steers the bike tick by tick for automated checks.

## Measured so far

| GDD proposal / budget | Status | Evidence |
| --- | --- | --- |
| Rust + Rapier 2D, same code on client and server | Confirmed on the prototype | Native x86-64 and WebAssembly give bit-identical states over 3,600 ticks with crashes and ragdolls (`check-determinism.mjs`) |
| Three.js low-poly on a 2D plane | Working | Procedural models, 60 fps target with adaptive quality; not yet measured on a real school Chromebook |
| First download under 10 MB | Well under | Production build 2.3 MB (≈0.8 MB gzipped): wasm 1.8 MB, JS 0.6 MB |
| Playable within 5 s | On track | Game ready ~0.4 s after page load from a local server (headless Chromium); needs a Chromebook on a school network |
| One server process, many 12-player rooms | Promising | A 12-bot room costs ~0.14 ms per tick, ≈115 rooms per CPU core at 60 ticks/s, physics only (`room_bench`, 2.1 GHz Xeon) |
| Bots fill quiet rooms | Working | Bots ride the whole park on both bikes (`tests/riding.rs`) |

## Next: multiplayer (proposal)

The plan the GDD sketches, mapped onto this code:

1. `crates/server`: an authoritative Rust process hosting many rooms, each a `Sim`,
   ticking at 60 Hz and sending snapshots at ~30 Hz over WebSockets.
2. Clients send their `Input` (4 bytes) every tick with the tick number.
3. **Own bike:** client-side prediction with rollback. The client keeps running its local
   `Sim`; when a server snapshot disagrees about the local rider, it restores the rider's
   state from the snapshot and re-simulates its unacknowledged inputs. Determinism makes
   corrections rare and small. This needs snapshot/restore of a rider's bodies in
   `bike-sim` (Rapier supports serialising the world).
4. **Other riders:** rendered from buffered server snapshots with interpolation (and a
   little extrapolation), since races use non-colliding ghost riders.
5. Ragdolls are cosmetic: the server only needs "crashed at tick T, respawns at T+N".
   Clients could simulate ragdolls locally to save server time and bandwidth.
6. The same `check.rs` scenario, run by the server build, keeps client and server honest.
