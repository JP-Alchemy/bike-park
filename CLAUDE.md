# Bike Park

Browser bike game; phase 1 is the riding prototype. Design: `docs/game-design-document.md`.
Architecture and determinism rules: `docs/architecture.md`.

## Commands

- `cargo test --workspace` — simulation tests (unit, riding behaviour, determinism)
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all`
- `cd web && npm run build` — wasm + typecheck + bundle; `npm run smoke` — headless browser check
- `node scripts/check-determinism.mjs` — native vs wasm hash (after `npm run wasm`)

## Rules that matter

- The simulation must stay deterministic across native and wasm: use `bike_sim::math`
  for trig (never `f32::sin` etc. or glam's angle helpers), no `HashMap` iteration, no
  unseeded randomness, inputs only through the quantized `Input`.
- Feel numbers live in `crates/sim/src/tuning.rs`; `crates/sim/tests/riding.rs` pins the
  feel down. If a tuning change breaks a riding test, decide which one is wrong.
- The render layout in `crates/sim/src/world.rs` (`render_layout`) is mirrored in
  `web/src/sim/layout.ts`; change both together.
- No real brands, riders or other games' IP in names, art or text.
