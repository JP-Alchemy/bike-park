# Bike Park

An online multiplayer bike game for the browser (working title): drop into a live park with
up to 12 riders, pull wheelies and tricks, crash spectacularly, and play short rotating
rounds together. See the [game design document](docs/game-design-document.md).

**Status: phase 1, the riding prototype.** The design document's biggest risk is the riding
feel, so this first version is nothing but riding: two bikes, one test park, wheelies,
flips, tricks, combos, crashes and bots, playable on keyboard, touch and gamepad. Plan and
playtest guide: [docs/prototype.md](docs/prototype.md).

## How it's built

One Rust physics core runs in both the browser (as WebAssembly) and, later, the game
server, so riding behaves identically on both sides. The core is deterministic: native and
WebAssembly builds produce bit-identical physics, which CI checks on every change.

```text
crates/sim       deterministic 2D riding simulation (Rust + Rapier 2D)
crates/sim-wasm  WebAssembly bindings for the browser
web/             client: Vite + TypeScript + Three.js
scripts/         wasm build and the native-vs-wasm determinism check
docs/            design document, architecture, prototype plan
```

More in [docs/architecture.md](docs/architecture.md).

## Getting started

You need Rust (stable) with the WebAssembly target, the matching `wasm-bindgen` CLI, and
Node.js 22:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # must match crates/sim-wasm/Cargo.toml

cd web
npm install
npm run dev          # builds the wasm, then serves the game with hot reload
```

Open the printed URL and ride: `↑` go, `↓` brake, `←` `→` lean, `Space` trick, `R`
respawn. `T` opens the live tuning panel.

## Checks

```sh
cargo test --workspace                            # simulation: unit, riding and determinism tests
cargo clippy --workspace --all-targets -- -D warnings
node scripts/check-determinism.mjs                # native vs WebAssembly, bit for bit (after npm run wasm)

cd web
npm run build                                     # wasm + typecheck + production build in web/dist
npm run smoke                                     # boots the build in headless Chromium
```

Handy while tuning the feel:

```sh
cargo run -p bike-sim --release --example probe -- wheelie e_dirt 1   # scripted inputs, printed state
cargo run -p bike-sim --release --example ride -- fatbike 90          # a bot rides the park
cargo run -p bike-sim --release --example room_bench                  # cost of a 12-rider room
```
