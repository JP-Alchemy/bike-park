# Phase 1: the riding prototype

The design document's biggest risk is the riding feel ("prototype nothing but the riding
first; test with kids before building more"). This prototype exists to settle that, and to
confirm the tech proposals, before any real time or money goes into multiplayer, modes or
art.

## What's in it

- **Two bike classes** with their own feel: the **E-Dirt** (punchy, light, easy wheelies,
  quick to flip) and the **Fatbike** (heavy, grippy, stable, slow to flip).
- **Riding:** throttle, brake, lean forward/back, with suspension, weight shift and air
  control; wheelies, stoppies and flips come out of the physics.
- **Wheelie meter** with a balance needle and style bonus; best wheelie tracked.
- **Tricks:** flips (single, double, …), No-Hander, Superman, Can-Can, Tail-Whip, Big Air;
  combos with a multiplier, banked after a short pause, lost on a crash.
- **Crashes:** ragdoll rider, screen shake, dust, a short respawn at the last checkpoint.
  Bailing a trick (landing before getting back on the bike) crashes.
- **The Proving Grounds:** one park with a section for each thing to test: rollers, a
  wheelie strip with distance markers, a kicker, a tabletop, a gap, a hill climb, steps and
  a big-air jump, then a finish line with a run timer and best time.
- **Bots** riding the same park as non-colliding ghosts, so it never feels empty.
- **Keyboard, touch and gamepad** from the start; the whole game works on a Chromebook
  keyboard.
- **Live tuning panel** (press `T`) to change the feel while riding and export it as JSON.

Not in the prototype (see the design document for when each lands): online multiplayer and
the server, lobbies and invite links, the round modes (race, wheelie battle, stunt battle),
the park hub, accounts, coins and cosmetics, challenges, clip saving, slow-motion crash
replays, ads, and real art and sound.

## Running a playtest

```sh
cd web && npm install && npm run dev     # then open the printed URL
```

Or build once (`npm run build`) and serve `web/dist/` from any static host.

| Controls | Keyboard | Touch | Gamepad |
| --- | --- | --- | --- |
| Throttle | `↑` / `W` | ▲ | RT or d-pad up |
| Brake | `↓` / `S` | ▼ | LT or d-pad down |
| Lean back / forward | `←` `→` / `A` `D` | ◀ ▶ | left stick or d-pad |
| Trick (+ direction) | `Space` | ★ | A |
| Respawn | `R` | ↺ | Y |

Other keys: `1`/`2` switch bike, `B` more/fewer bots, `T` tuning panel, `M` sound, `H`
controls, `F3` debug line.

URL options, useful for setting up a test station:

| Option | Effect |
| --- | --- |
| `?bike=fatbike` | Start on the fatbike (`e_dirt` is the default) |
| `?checkpoint=7` | Start at a section: 0 start, 1 wheelie strip, 2 kicker, 3 tabletop, 4 gap, 5 hill climb, 6 steps, 7 big air |
| `?bots=0` | Number of bots (default 3) |
| `?quality=0` | Force low (0), medium (1) or high (2) graphics |
| `?debug` | Show FPS, rider count and the state hash |

### Tuning workflow

1. Press `T`, change a value; the rider respawns on the new setup at their checkpoint.
2. Ride the section that exercises it (`?checkpoint=` gets you there quickly).
3. When it feels right, "Copy bike JSON" and paste the numbers into
   `crates/sim/src/tuning.rs`.
4. Run `cargo test -p bike-sim`. `tests/riding.rs` pins down the feel (throttle alone
   doesn't loop the bike, a wheelie can be popped and held, a backflip off Big Air is
   possible, the fatbike pitches up more slowly than the e-dirt, …). If a test now fails,
   decide whether the test or the tuning is wrong.

## What "feels right" means

A checklist for internal testing before putting it in front of kids:

- [ ] **Fun within 10 seconds:** holding `↑` alone is exciting and safe (no loop-outs).
- [ ] **Wheelies:** easy to start (lean back + go), hard to hold, and the meter explains why
      you dropped it.
- [ ] **Flips:** a first backflip off Big Air is possible within a few tries on the e-dirt;
      harder, but possible, on the fatbike.
- [ ] **Landings:** landing roughly level always works; nose-first or on your head never
      does, and it's obvious why.
- [ ] **Crashes are funny, not punishing:** back on the bike in under 2 seconds.
- [ ] **The bikes feel different** within one run each, and players pick a favourite.
- [ ] **Nothing floaty:** jumps feel snappy (gravity is 14 m/s² rather than 9.8 for this).
- [ ] **Performance:** 60 fps on a mid-range laptop and 30+ on a school Chromebook.

## Playtest with kids (proposal)

The design document asks where to find ~20 kids aged 10–16. Per session:

- 2–4 kids at a time, on their own devices where possible (Chromebooks, phones), 15
  minutes, no instructions beyond "try this bike game".
- Watch, don't help. Note: time to first movement, first wheelie, first flip and first
  laugh; where they get stuck; whether they switch bikes; whether they call someone over.
- Afterwards ask three questions: *What was the best bit? What was annoying? Would you play
  it again tomorrow, and with who?*
- Record every tuning set used (Copy JSON) next to the notes.

### Proposed gate (to agree before testing)

The prototype passes when most kids, unprompted:

1. keep riding for the full 15 minutes;
2. land a wheelie of 5 m or more;
3. land at least one flip;
4. name a favourite bike and say why;
5. say they'd play again, or ask to.

If it fails, fix and retest, or stop. The design document calls this the cheapest place to
learn the idea doesn't work.

## After the gate

Next is multiplayer (see [architecture](architecture.md#next-multiplayer-proposal)): the
authoritative server, client prediction for your own bike, smoothing for others, and the
park hub between rounds.
