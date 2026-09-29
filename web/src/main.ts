// Bike Park: riding prototype. Boots the WebAssembly simulation, then runs the loop:
// read input → advance the fixed-tick simulation → draw the interpolated state.

import "./style.css";
import { Sound } from "./audio/sound";
import { TuningPanel } from "./debug/tuningPanel";
import { Hud } from "./hud/hud";
import { Input, type ControlState } from "./input/input";
import { L } from "./sim/layout";
import { SimClient } from "./sim/simClient";
import type { BikeClassId, HudState, SimEvent } from "./sim/types";
import { GameRenderer, type Quality } from "./render/renderer";

const BIKES: { id: BikeClassId; name: string }[] = [
  { id: "e_dirt", name: "E-Dirt" },
  { id: "fatbike", name: "Fatbike" },
];
const BOT_STEPS = [0, 3, 7, 11];

const params = new URLSearchParams(location.search);

function load(key: string): string | null {
  try {
    return localStorage.getItem(`bikepark.${key}`);
  } catch {
    return null;
  }
}

function save(key: string, value: string): void {
  try {
    localStorage.setItem(`bikepark.${key}`, value);
  } catch {
    // Storage unavailable: the choice just isn't remembered.
  }
}

async function main(): Promise<void> {
  const loading = document.getElementById("loading")!;
  const startBike = (params.get("bike") ?? load("bike") ?? "e_dirt") as BikeClassId;
  let bikeIndex = Math.max(0, BIKES.findIndex((b) => b.id === startBike));

  // ?checkpoint=N starts at a section of the park (handy in playtests).
  const sim = await SimClient.create(BIKES[bikeIndex].id, Number(params.get("checkpoint") ?? 0));
  sim.setBotCount(Number(params.get("bots") ?? 3));

  const input = new Input(document.getElementById("touch")!);
  const renderer = new GameRenderer(document.getElementById("game")!, sim, document.getElementById("tags")!);
  const hud = new Hud(document.getElementById("hud")!);
  const sound = new Sound();
  const tuning = new TuningPanel(sim);
  const track = sim.track;
  const span = track.finish_x - track.checkpoints[0];
  hud.setCheckpoints(track.checkpoints.slice(1).map((x) => (x - track.checkpoints[0]) / span));
  hud.setBike(BIKES[bikeIndex].name);
  hud.setBots(sim.botCount);
  hud.setMuted(sound.muted);
  const forced = params.get("quality");
  if (forced !== null) renderer.setQuality(Number(forced) as Quality);
  loading.remove();

  // --- Menu actions (keyboard and HUD buttons) ---
  const switchBike = (index?: number) => {
    bikeIndex = index ?? (bikeIndex + 1) % BIKES.length;
    const bike = BIKES[bikeIndex];
    sim.setTuning(sim.localId, SimClient.defaultTuning(bike.id));
    hud.setBike(bike.name);
    save("bike", bike.id);
    tuning.refresh();
  };
  const cycleBots = () => {
    const i = BOT_STEPS.findIndex((n) => n > sim.botCount);
    sim.setBotCount(BOT_STEPS[i < 0 ? 0 : i]);
    hud.setBots(sim.botCount);
  };
  const toggleMute = () => hud.setMuted(sound.toggleMute());
  const blur = (fn: () => void) => (e: Event) => {
    fn();
    (e.currentTarget as HTMLElement).blur();
  };
  hud.buttons.bike.addEventListener("click", blur(() => switchBike()));
  hud.buttons.bots.addEventListener("click", blur(cycleBots));
  hud.buttons.tuning.addEventListener("click", blur(() => tuning.toggle()));
  hud.buttons.sound.addEventListener("click", blur(toggleMute));
  hud.buttons.help.addEventListener("click", blur(() => hud.toggleHelp()));
  const kb = input.keyboard;
  kb.on("Digit1", () => switchBike(0));
  kb.on("Digit2", () => switchBike(1));
  kb.on("KeyB", cycleBots);
  kb.on("KeyT", () => tuning.toggle());
  kb.on("Backquote", () => tuning.toggle());
  kb.on("KeyM", toggleMute);
  kb.on("KeyH", () => hud.toggleHelp());
  let debug = params.has("debug");
  kb.on("F3", () => (debug = !debug));

  // Audio may only start after a gesture.
  const unlock = () => sound.unlock();
  window.addEventListener("keydown", unlock);
  window.addEventListener("pointerdown", unlock);

  const onEvents = (events: SimEvent[]) => {
    for (const e of events) {
      const local = e.rider === sim.localId;
      if (e.type === "landed") {
        renderer.burst(e.rider, Math.round(6 + e.air_time * 14), 1 + e.air_time);
        if (local) {
          sound.land(e.air_time);
          if (e.air_time > 0.7) renderer.cam.kick(Math.min(0.5, e.air_time * 0.25));
        }
      }
      if (e.type === "crash") {
        renderer.burst(e.rider, 26, 2);
        if (local) {
          sound.crash();
          renderer.cam.kick(0.9);
        }
      }
      if (!local) continue;
      hud.onEvent(e);
      if (e.type === "trick_landed") sound.trick();
      if (e.type === "combo_banked" && e.tricks > 1) sound.combo(e.multiplier);
      if (e.type === "checkpoint") sound.checkpoint();
      if (e.type === "finish") sound.finish();
      if (e.type === "respawn") renderer.cam.snap();
    }
  };

  // Automated checks can steer the bike tick by tick: bikePark.autopilot = (hud) => controls.
  const hooks: { autopilot?: (h: HudState) => ControlState } = {};
  const controls = () => (hooks.autopilot ? hooks.autopilot(sim.hud()) : input.read());

  // --- Main loop ---
  const debugEl = document.createElement("div");
  debugEl.id = "debug";
  document.body.appendChild(debugEl);
  let last = performance.now();
  let slowFrames = 0;
  let frames = 0;
  let fpsTime = 0;
  let fps = 0;
  const frame = (now: number) => {
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;

    const alpha = sim.advance(dt, controls, onEvents);
    const h = sim.hud();
    hud.update(h, dt, input.used);
    renderer.render(dt, alpha, h.checkpoint);

    const local = sim.curr;
    const li = findLocal(sim);
    if (li >= 0) {
      const o = li * sim.stride;
      sound.update(local[o + L.SPEED], local[o + L.THROTTLE], (local[o + L.FLAGS] & 3) !== 0, local[o + L.CLASS] === 1, h.crashed);
    }

    // Adaptive quality: drop shadows, then resolution, on slow machines.
    frames++;
    fpsTime += dt;
    if (fpsTime >= 1) {
      fps = frames / fpsTime;
      frames = 0;
      fpsTime = 0;
      if (forced === null && fps < 40) slowFrames++;
      else slowFrames = 0;
      if (slowFrames >= 3 && renderer.getQuality() > 0) {
        renderer.setQuality((renderer.getQuality() - 1) as Quality);
        slowFrames = 0;
      }
    }
    debugEl.textContent = debug
      ? `${fps.toFixed(0)} fps · ${sim.riderCount} riders · quality ${renderer.getQuality()} · state ${sim.stateHash()}`
      : "";
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);

  // Handy for playtests and automated checks.
  (window as unknown as { bikePark: unknown }).bikePark = Object.assign(hooks, { sim, renderer, hud });
}

function findLocal(sim: SimClient): number {
  for (let i = 0; i < sim.riderCount; i++) if (sim.curr[i * sim.stride + L.ID] === sim.localId) return i;
  return -1;
}

main().catch((err: unknown) => {
  console.error(err);
  const loading = document.getElementById("loading");
  if (loading) {
    loading.classList.add("error");
    loading.textContent = `Could not start the game: ${err instanceof Error ? err.message : String(err)}`;
  }
});
