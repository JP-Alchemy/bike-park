// Draws the simulation: interpolates between the last two ticks, poses every rider,
// follows the local rider with the camera.

import * as THREE from "three";
import { L, TRICK_IDS } from "../sim/layout";
import type { SimClient } from "../sim/simClient";
import { BikeModel } from "./bikeModel";
import { FollowCamera } from "./camera";
import { LIVERIES, lerpAngle } from "./parts";
import { Dust } from "./particles";
import { RiderModel } from "./riderModel";
import { World } from "./world";

const BOT_NAMES = ["Rocket", "Nova", "Dash", "Blaze", "Pixel", "Skid", "Comet", "Jet", "Bolt", "Turbo", "Zip", "Flash"];
const ANGLES = new Set<number>([L.CHASSIS + 2, L.REAR + 2, L.FRONT + 2]);

interface RiderView {
  bike: BikeModel;
  rider: RiderModel;
  tag?: HTMLElement;
  tuningVersion: number;
  dustDebt: number;
}

export type Quality = 0 | 1 | 2;

export class GameRenderer {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  readonly cam = new FollowCamera();
  readonly world: World;
  readonly dust = new Dust();
  private readonly sun: THREE.DirectionalLight;
  private readonly views = new Map<number, RiderView>();
  /** Interpolated render state of the current frame. */
  private readonly frame: Float32Array;
  private quality: Quality = 2;
  private readonly tmp = new THREE.Vector3();

  constructor(
    container: HTMLElement,
    private readonly sim: SimClient,
    private readonly tagLayer: HTMLElement,
  ) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "high-performance" });
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFShadowMap;
    container.appendChild(this.renderer.domElement);

    this.scene.fog = new THREE.Fog(0xbfe3f5, 60, 260);
    this.scene.add(new THREE.HemisphereLight(0xdff1ff, 0x7a6a4a, 1.6));
    this.sun = new THREE.DirectionalLight(0xfff4e0, 2.4);
    this.sun.castShadow = true;
    this.sun.shadow.mapSize.set(1024, 1024);
    const sc = this.sun.shadow.camera;
    sc.left = -18;
    sc.right = 18;
    sc.top = 14;
    sc.bottom = -10;
    sc.near = 1;
    sc.far = 80;
    this.sun.shadow.bias = -0.0008;
    this.scene.add(this.sun, this.sun.target);

    this.world = new World(sim.track);
    this.scene.add(this.world.group, this.dust.points);
    this.frame = new Float32Array(sim.stride * 16);
    this.resize();
    window.addEventListener("resize", () => this.resize());
  }

  resize(): void {
    const w = window.innerWidth;
    const h = window.innerHeight;
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, this.quality === 2 ? 2 : this.quality === 1 ? 1 : 0.75));
    this.renderer.setSize(w, h);
    this.cam.resize(w / h);
  }

  setQuality(q: Quality): void {
    if (q === this.quality) return;
    this.quality = q;
    this.renderer.shadowMap.enabled = q === 2;
    this.sun.castShadow = q === 2;
    this.scene.traverse((o) => {
      if (o instanceof THREE.Mesh) o.material.needsUpdate = true;
    });
    this.resize();
  }

  getQuality(): Quality {
    return this.quality;
  }

  /** Index of rider `id` in a render state, or -1. */
  private indexOf(state: Float32Array, id: number): number {
    const s = this.sim.stride;
    for (let i = 0; i < state.length / s; i++) if (state[i * s + L.ID] === id) return i;
    return -1;
  }

  /** Current interpolated values for rider slot `i`. */
  private rider(i: number): Float32Array {
    const s = this.sim.stride;
    return this.frame.subarray(i * s, (i + 1) * s);
  }

  /** World position of rider `id` (hip when thrown off). */
  riderPosition(id: number): [number, number] | null {
    const i = this.indexOf(this.sim.curr, id);
    if (i < 0) return null;
    const r = this.rider(i);
    return r[L.STATUS] === 1 ? [r[L.SKELETON + 4], r[L.SKELETON + 5]] : [r[L.CHASSIS], r[L.CHASSIS + 1]];
  }

  burst(id: number, count: number, spread: number): void {
    const p = this.riderPosition(id);
    if (p) this.dust.emit(p[0], p[1] - 0.3, 0, 0.5, count, spread);
  }

  render(dt: number, alpha: number, checkpoint: number): void {
    const { stride, curr, prev } = this.sim;
    const count = curr.length / stride;
    if (this.frame.length < curr.length) throw new Error("too many riders for the render buffer");

    // Interpolate between ticks (snapping across respawns and rider list changes).
    for (let i = 0; i < count; i++) {
      const id = curr[i * stride + L.ID];
      const j = prev.length === curr.length && prev[i * stride + L.ID] === id ? i : this.indexOf(prev, id);
      const o = i * stride;
      const po = j >= 0 ? j * stride : o;
      const src = j >= 0 ? prev : curr;
      const teleport = Math.abs(curr[o + L.CHASSIS] - src[po + L.CHASSIS]) > 4;
      const t = teleport ? 1 : alpha;
      for (let k = 0; k < stride; k++) {
        const a = src[po + k];
        const b = curr[o + k];
        this.frame[o + k] = ANGLES.has(k) ? lerpAngle(a, b, t) : a + (b - a) * t;
      }
      this.frame[o + L.ID] = id;
      this.frame[o + L.STATUS] = curr[o + L.STATUS];
      this.frame[o + L.TRICK] = curr[o + L.TRICK];
      this.frame[o + L.FLAGS] = curr[o + L.FLAGS];
    }

    const seen = new Set<number>();
    for (let i = 0; i < count; i++) {
      const r = this.rider(i);
      const id = r[L.ID];
      seen.add(id);
      const view = this.view(id);
      const trick = TRICK_IDS[r[L.TRICK]] ?? "";
      view.bike.update({
        x: r[L.CHASSIS],
        y: r[L.CHASSIS + 1],
        angle: r[L.CHASSIS + 2],
        rear: [r[L.REAR], r[L.REAR + 1], r[L.REAR + 2]],
        front: [r[L.FRONT], r[L.FRONT + 1], r[L.FRONT + 2]],
        whip: trick === "tail_whip" ? r[L.TRICK_EXTENT] : 0,
      });
      view.rider.update((k) => [r[L.SKELETON + 2 * k], r[L.SKELETON + 2 * k + 1]]);
      this.emitDust(view, r, dt);
      if (view.tag) this.placeTag(view.tag, r);
    }
    for (const [id, view] of this.views) {
      if (!seen.has(id)) {
        view.bike.dispose();
        view.rider.dispose();
        view.tag?.remove();
        this.views.delete(id);
      }
    }

    // Camera follows the local rider.
    const li = this.indexOf(curr, this.sim.localId);
    if (li >= 0) {
      const r = this.rider(li);
      const crashed = r[L.STATUS] === 1;
      const x = crashed ? r[L.SKELETON + 4] : r[L.CHASSIS];
      const y = crashed ? r[L.SKELETON + 5] : r[L.CHASSIS + 1];
      const pi = this.indexOf(prev, this.sim.localId);
      const vx = pi >= 0 ? (curr[li * stride + L.CHASSIS] - prev[pi * stride + L.CHASSIS]) / this.sim.dt : 0;
      if (Math.abs(vx) > 200) this.cam.snap();
      this.cam.update(dt, x, y, Math.abs(vx) > 200 ? 0 : vx, r[L.SPEED]);
      this.sun.position.set(x - 12, y + 22, 16);
      this.sun.target.position.set(x, y, 0);
    }
    this.dust.update(dt);
    this.world.update(dt, checkpoint);
    this.renderer.render(this.scene, this.cam.camera);
  }

  private view(id: number): RiderView {
    const version = this.sim.tuningVersion(id);
    let v = this.views.get(id);
    if (v && v.tuningVersion !== version) {
      v.bike.dispose();
      this.scene.remove(v.bike.root);
      v.bike = new BikeModel(this.sim.tuning(id), LIVERIES[id % LIVERIES.length]);
      v.tuningVersion = version;
      this.scene.add(v.bike.root);
    }
    if (!v) {
      const livery = LIVERIES[id % LIVERIES.length];
      const bike = new BikeModel(this.sim.tuning(id), livery);
      const rider = new RiderModel(livery);
      this.scene.add(bike.root, rider.root);
      let tag: HTMLElement | undefined;
      if (id !== this.sim.localId) {
        tag = document.createElement("div");
        tag.className = "name-tag";
        tag.textContent = BOT_NAMES[id % BOT_NAMES.length];
        this.tagLayer.appendChild(tag);
      }
      v = { bike, rider, tag, tuningVersion: version, dustDebt: 0 };
      this.views.set(id, v);
    }
    return v;
  }

  private emitDust(view: RiderView, r: Float32Array, dt: number): void {
    const flags = r[L.FLAGS];
    const rearDown = (flags & 1) !== 0;
    const throttle = r[L.THROTTLE];
    const speed = r[L.SPEED];
    if (!rearDown || throttle < 0.2 || r[L.STATUS] === 1) return;
    view.dustDebt += dt * (8 + 25 * throttle * Math.max(0, 1 - speed / 16));
    const n = Math.floor(view.dustDebt);
    if (n > 0) {
      view.dustDebt -= n;
      const dir = Math.cos(r[L.CHASSIS + 2]);
      this.dust.emit(r[L.REAR] - 0.25 * dir, r[L.REAR + 1] - 0.3, -2 - speed * 0.15, 0.8, n, 0.6);
    }
  }

  private placeTag(tag: HTMLElement, r: Float32Array): void {
    const hx = r[L.SKELETON];
    const hy = r[L.SKELETON + 1];
    this.tmp.set(hx, hy + 0.55, 0).project(this.cam.camera);
    const visible = this.tmp.z < 1 && Math.abs(this.tmp.x) < 1.1 && Math.abs(this.tmp.y) < 1.1;
    tag.style.display = visible ? "block" : "none";
    if (visible) {
      const x = ((this.tmp.x + 1) / 2) * window.innerWidth;
      const y = ((1 - this.tmp.y) / 2) * window.innerHeight;
      tag.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) translate(-50%, -100%)`;
    }
  }
}
