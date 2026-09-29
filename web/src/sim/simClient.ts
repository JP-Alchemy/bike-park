// Runs the WebAssembly simulation at its fixed tick rate and keeps the last two render
// states so the renderer can interpolate between ticks at any display refresh rate.

import init, { Game, defaultTuningJson, renderStride, tickRate } from "../wasm/pkg/bike_sim_wasm.js";
import type { ControlState } from "../input/input";
import type { BikeClassId, BikeTuning, HudState, SimEvent, Track, WorldTuning } from "./types";

/** Longest frame we try to catch up on; beyond this the game just slows down. */
const MAX_FRAME = 0.25;

export class SimClient {
  readonly stride: number;
  readonly dt: number;
  readonly track: Track;
  prev: Float32Array;
  curr: Float32Array;
  localId: number;
  private readonly bots: number[] = [];
  private acc = 0;
  /** Bike tunings by rider id, for building models, and a counter bumped on each change. */
  private tunings = new Map<number, BikeTuning>();
  private versions = new Map<number, number>();

  private constructor(private readonly game: Game, classId: BikeClassId, checkpoint: number) {
    this.stride = renderStride();
    this.dt = 1 / tickRate();
    this.track = JSON.parse(game.trackJson()) as Track;
    this.localId = game.addRider(classId, checkpoint);
    this.curr = game.renderState();
    this.prev = this.curr;
  }

  /** Starts a room with the local rider at `checkpoint` (0 = the start line). */
  static async create(classId: BikeClassId, checkpoint = 0): Promise<SimClient> {
    await init();
    return new SimClient(new Game(), classId, checkpoint);
  }

  static defaultTuning(classId: BikeClassId): BikeTuning {
    return JSON.parse(defaultTuningJson(classId)) as BikeTuning;
  }

  get riderCount(): number {
    return this.curr.length / this.stride;
  }

  get botCount(): number {
    return this.bots.length;
  }

  /** Adds or removes bots until there are `n`, spread over the park's checkpoints. */
  setBotCount(n: number): void {
    const classes: BikeClassId[] = ["e_dirt", "fatbike"];
    while (this.bots.length < n) {
      // The first bot starts alongside the player; the rest spread over the park.
      const i = this.bots.length;
      const checkpoint = i % this.track.checkpoints.length;
      this.bots.push(this.game.addBot(classes[i % 2], checkpoint));
    }
    while (this.bots.length > n) {
      const id = this.bots.pop()!;
      this.game.removeRider(id);
      this.tunings.delete(id);
    }
    this.refreshState();
  }

  isBot(id: number): boolean {
    return this.bots.includes(id);
  }

  tuning(id: number): BikeTuning {
    let t = this.tunings.get(id);
    if (!t) {
      t = JSON.parse(this.game.tuningJson(id)) as BikeTuning;
      this.tunings.set(id, t);
    }
    return t;
  }

  setTuning(id: number, tuning: BikeTuning): void {
    this.game.setTuningJson(id, JSON.stringify(tuning));
    this.tunings.delete(id);
    this.versions.set(id, this.tuningVersion(id) + 1);
    this.refreshState();
  }

  tuningVersion(id: number): number {
    return this.versions.get(id) ?? 0;
  }

  worldTuning(): WorldTuning {
    return JSON.parse(this.game.worldTuningJson()) as WorldTuning;
  }

  setWorldTuning(world: WorldTuning): void {
    this.game.setWorldTuningJson(JSON.stringify(world));
    this.refreshState();
  }

  hud(): HudState {
    return JSON.parse(this.game.hudJson(this.localId)) as HudState;
  }

  stateHash(): string {
    return this.game.stateHash();
  }

  /**
   * Advances the simulation by real time `frameDt`, feeding the local rider's controls
   * every tick. Returns the interpolation factor between `prev` and `curr`.
   */
  advance(frameDt: number, controls: () => ControlState, onEvents: (events: SimEvent[]) => void): number {
    this.acc += Math.min(frameDt, MAX_FRAME);
    while (this.acc >= this.dt) {
      const c = controls();
      this.game.setInput(this.localId, c.throttle, c.brake, c.lean, c.trick, c.respawn);
      this.game.step();
      this.prev = this.curr;
      this.curr = this.game.renderState();
      const events = JSON.parse(this.game.drainEventsJson()) as SimEvent[];
      if (events.length) onEvents(events);
      this.acc -= this.dt;
    }
    return this.acc / this.dt;
  }

  /** After riders are added or removed the rider order changes; don't interpolate across it. */
  private refreshState(): void {
    this.curr = this.game.renderState();
    this.prev = this.curr;
  }
}
