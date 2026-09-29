#!/usr/bin/env node
// Builds the simulation to WebAssembly and generates the JS bindings into
// web/src/wasm/pkg. Needs the wasm32 target and a matching wasm-bindgen CLI:
//   rustup target add wasm32-unknown-unknown
//   cargo install wasm-bindgen-cli --version <version pinned in crates/sim-wasm/Cargo.toml>
import { execFileSync } from "node:child_process";
import { readFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "web", "src", "wasm", "pkg");
const dev = process.argv.includes("--dev");

function run(cmd, args) {
  execFileSync(cmd, args, { cwd: root, stdio: "inherit" });
}

const cargoToml = readFileSync(join(root, "crates", "sim-wasm", "Cargo.toml"), "utf8");
const pinned = cargoToml.match(/wasm-bindgen = "=([\d.]+)"/)?.[1];
let cli = "";
try {
  cli = execFileSync("wasm-bindgen", ["--version"], { encoding: "utf8" }).trim().split(" ")[1];
} catch {
  console.error(`wasm-bindgen CLI not found. Install it with:\n  cargo install wasm-bindgen-cli --version ${pinned}`);
  process.exit(1);
}
if (pinned && cli !== pinned) {
  console.error(`wasm-bindgen CLI is ${cli} but the crate pins ${pinned}. Run:\n  cargo install wasm-bindgen-cli --version ${pinned}`);
  process.exit(1);
}

const profile = dev ? "dev" : "release";
run("cargo", ["build", "-p", "bike-sim-wasm", "--target", "wasm32-unknown-unknown", "--profile", profile]);
const wasm = join(root, "target", "wasm32-unknown-unknown", dev ? "debug" : "release", "bike_sim_wasm.wasm");
if (!existsSync(wasm)) {
  console.error(`missing ${wasm}`);
  process.exit(1);
}
run("wasm-bindgen", ["--target", "web", "--out-dir", outDir, wasm]);
console.log(`wasm bindings written to ${outDir}`);
