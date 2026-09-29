#!/usr/bin/env node
// Runs the shared determinism scenario natively (the server's platform) and in
// WebAssembly (the browser's), and fails if the final physics state differs by a bit.
// Build the wasm first: node scripts/build-wasm.mjs
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const ticks = Number(process.argv[2] ?? 3600);

const native = execFileSync(
  "cargo",
  ["run", "-q", "-p", "bike-sim", "--release", "--example", "determinism", "--", String(ticks)],
  { cwd: root, encoding: "utf8" },
).trim();

const pkg = join(root, "web", "src", "wasm", "pkg");
const bindings = await import(pathToFileURL(join(pkg, "bike_sim_wasm.js")).href);
bindings.initSync({ module: readFileSync(join(pkg, "bike_sim_wasm_bg.wasm")) });
const wasm = bindings.determinismScenario(ticks);

console.log(`native: ${native}\nwasm:   ${wasm}\nticks:  ${ticks}`);
if (native !== wasm) {
  console.error("MISMATCH: the browser and the server would disagree about the physics.");
  process.exit(1);
}
console.log("OK: native and WebAssembly simulations are bit-identical.");
