// Shapes of the JSON the simulation hands over (serde output of crates/sim).

export type BikeClassId = "e_dirt" | "fatbike";
export type TrickKind = "no_hander" | "superman" | "can_can" | "tail_whip";
export type ManualKind = "wheelie" | "stoppie";
export type CrashReason = "head" | "bail" | "flipped" | "out_of_bounds";

export interface HudState {
  speed: number;
  total: number;
  combo_points: number;
  combo_tricks: number;
  multiplier: number;
  combo_timer: number;
  in_air: boolean;
  air_time: number;
  air_rotation: number;
  trick: TrickKind | null;
  trick_extent: number;
  trick_done: boolean;
  manual: ManualKind | null;
  manual_distance: number;
  balance: number;
  best_wheelie: number;
  run_time: number | null;
  best_run: number | null;
  checkpoint: number;
  checkpoints: number;
  crashed: boolean;
  finished: boolean;
  progress: number;
}

export type SimEvent = { rider: number; tick: number } & (
  | { type: "take_off" }
  | { type: "landed"; air_time: number }
  | { type: "trick_started"; trick: TrickKind }
  | { type: "trick_landed"; name: string; points: number }
  | { type: "manual_start"; kind: ManualKind }
  | { type: "manual_end"; kind: ManualKind; distance: number }
  | { type: "combo_banked"; points: number; tricks: number; multiplier: number }
  | { type: "combo_lost"; points: number; tricks: number }
  | { type: "crash"; reason: CrashReason; speed: number }
  | { type: "respawn"; checkpoint: number }
  | { type: "checkpoint"; index: number }
  | { type: "run_start" }
  | { type: "finish"; time: number; best: boolean }
);

export interface Zone {
  kind: "section" | "wheelie_strip";
  label: string;
  x0: number;
  x1: number;
}

export interface Track {
  name: string;
  ground: [number, number][];
  checkpoints: number[];
  finish_x: number;
  kill_y: number;
  zones: Zone[];
}

export interface Suspension {
  axis: [number, number];
  travel: number;
  droop: number;
  stiffness: number;
  damping: number;
}

/** Bike tuning (crates/sim/src/tuning.rs). Only the fields the client reads are typed strictly. */
export interface BikeTuning {
  class: BikeClassId;
  name: string;
  rear_axle: [number, number];
  front_axle: [number, number];
  rear_wheel_radius: number;
  front_wheel_radius: number;
  frame_rear: [number, number];
  frame_front: [number, number];
  frame_radius: number;
  pegs: [number, number];
  bars: [number, number];
  hips: [number, number];
  rear_suspension: Suspension;
  front_suspension: Suspension;
  [key: string]: unknown;
}

export interface WorldTuning {
  gravity: number;
  solver_iterations: number;
  respawn_delay: number;
  combo_window: number;
}
