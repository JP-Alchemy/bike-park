// Live tuning panel for the riding prototype: every slider edits the simulation's tuning
// data and respawns the rider on the new setup. "Copy JSON" exports a set to paste back
// into crates/sim/src/tuning.rs once it feels right.

import GUI from "lil-gui";
import { SimClient } from "../sim/simClient";
import type { BikeTuning, WorldTuning } from "../sim/types";

type Range = [key: string, min: number, max: number, step: number, label?: string];

const BIKE_FOLDERS: [string, Range[]][] = [
  [
    "Motor",
    [
      ["max_torque", 50, 700, 5, "torque (N·m)"],
      ["max_power", 500, 15000, 100, "power (W)"],
      ["top_speed", 5, 30, 0.5, "top speed (m/s)"],
      ["coast_torque", 0, 50, 1, "coast drag"],
      ["air_drag", 0, 2, 0.05],
    ],
  ],
  [
    "Lean & air",
    [
      ["ground_lean_torque", 0, 500, 5, "ground lean"],
      ["wheelie_assist", 0, 400, 5],
      ["rider_lean_shift", 0, 0.35, 0.01, "hip shift (m)"],
      ["air_spin_rate", 1, 12, 0.1, "spin rate (rad/s)"],
      ["air_spin_gain", 50, 1500, 10, "spin gain"],
      ["air_spin_max_torque", 100, 2000, 10, "spin torque"],
      ["air_spin_damping", 0, 50, 1, "spin damping"],
    ],
  ],
  [
    "Brakes & tyres",
    [
      ["rear_brake_torque", 0, 1000, 10, "rear brake"],
      ["front_brake_torque", 0, 1000, 10, "front brake"],
      ["tyre_friction", 0.2, 3, 0.05],
      ["tyre_restitution", 0, 0.5, 0.01],
    ],
  ],
  [
    "Mass",
    [
      ["chassis_mass", 10, 120, 1, "frame mass (kg)"],
      ["chassis_inertia", 1, 30, 0.5, "frame inertia"],
      ["wheel_mass", 1, 20, 0.5, "wheel mass (kg)"],
    ],
  ],
];

const SUSPENSION: Range[] = [
  ["stiffness", 1000, 40000, 100],
  ["damping", 50, 4000, 10],
  ["travel", 0.02, 0.35, 0.01],
];

const WORLD: Range[] = [
  ["gravity", 5, 30, 0.5, "gravity (m/s²)"],
  ["respawn_delay", 0.3, 4, 0.1, "respawn delay (s)"],
  ["combo_window", 0.5, 5, 0.1, "combo window (s)"],
  ["solver_iterations", 1, 12, 1, "solver substeps"],
];

export class TuningPanel {
  private gui?: GUI;

  constructor(private readonly sim: SimClient) {}

  get open(): boolean {
    return this.gui !== undefined;
  }

  toggle(): void {
    if (this.gui) {
      this.gui.destroy();
      this.gui = undefined;
    } else {
      this.build();
    }
  }

  /** Rebuilds the panel for the current bike (after switching class). */
  refresh(): void {
    if (!this.gui) return;
    this.gui.destroy();
    this.build();
  }

  private build(): void {
    const id = this.sim.localId;
    const tuning = structuredClone(this.sim.tuning(id)) as BikeTuning;
    const world = this.sim.worldTuning();
    const gui = new GUI({ title: `Tuning: ${tuning.name}` });
    gui.domElement.style.pointerEvents = "auto";
    this.gui = gui;

    const applyBike = () => this.sim.setTuning(id, tuning);
    const applyWorld = () => this.sim.setWorldTuning(world);
    const numbers = tuning as unknown as Record<string, number>;

    for (const [name, ranges] of BIKE_FOLDERS) {
      const folder = gui.addFolder(name);
      for (const [key, min, max, step, label] of ranges) {
        folder.add(numbers, key, min, max, step).name(label ?? key).onFinishChange(applyBike);
      }
      if (name !== "Motor") folder.close();
    }
    for (const [name, susp] of [
      ["Rear suspension", tuning.rear_suspension],
      ["Front suspension", tuning.front_suspension],
    ] as const) {
      const folder = gui.addFolder(name);
      for (const [key, min, max, step] of SUSPENSION) {
        folder.add(susp as unknown as Record<string, number>, key, min, max, step).onFinishChange(applyBike);
      }
      folder.close();
    }
    const worldFolder = gui.addFolder("World");
    for (const [key, min, max, step, label] of WORLD) {
      worldFolder.add(world as unknown as Record<string, number>, key, min, max, step).name(label ?? key).onFinishChange(applyWorld);
    }
    worldFolder.close();

    const actions = {
      reset: () => {
        this.sim.setTuning(id, SimClient.defaultTuning(tuning.class));
        this.refresh();
      },
      copyBike: () => copy(JSON.stringify(tuning, null, 2)),
      copyWorld: () => copy(JSON.stringify(world satisfies WorldTuning, null, 2)),
    };
    gui.add(actions, "reset").name("Reset bike to defaults");
    gui.add(actions, "copyBike").name("Copy bike JSON");
    gui.add(actions, "copyWorld").name("Copy world JSON");
  }
}

function copy(text: string): void {
  navigator.clipboard?.writeText(text).catch(() => undefined);
  console.log(text);
}
