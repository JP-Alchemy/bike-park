// Procedural low-poly bikes. Everything is built in bike space (the simulation's chassis
// frame: origin between the axles, +x forward, +y up) from the bike's tuning, so a
// tuning change reshapes the model too. Generic designs only: no real brands.

import * as THREE from "three";
import type { BikeTuning } from "../sim/types";
import { ball, box, boxBetween, mat, placeTube, tube, type Livery } from "./parts";

export interface BikePose {
  x: number;
  y: number;
  angle: number;
  /** Wheel centres and spin angles in world space. */
  rear: [number, number, number];
  front: [number, number, number];
  /** Tail-whip progress 0..1 (0 when not whipping). */
  whip: number;
}

interface Wheel {
  group: THREE.Group;
  spin: THREE.Group;
}

export class BikeModel {
  readonly root = new THREE.Group();
  private readonly whip = new THREE.Group();
  private readonly parts = new THREE.Group();
  private readonly rearWheel: Wheel;
  private readonly frontWheel: Wheel;
  private readonly fork: THREE.Mesh[];
  private readonly swingarm: THREE.Mesh[];
  private readonly frontFender: THREE.Group;
  private readonly crank?: THREE.Group;
  /** Steering head (top of the fork) and swingarm pivot, bike space. */
  private readonly head: THREE.Vector2;
  private readonly pivot: THREE.Vector2;
  private readonly fat: boolean;
  private readonly owned: THREE.BufferGeometry[] = [];

  constructor(t: BikeTuning, livery: Livery) {
    this.fat = t.class === "fatbike";
    const axis = new THREE.Vector2(...t.front_suspension.axis).normalize();
    const front = new THREE.Vector2(...t.front_axle);
    this.head = front.clone().addScaledVector(axis, this.fat ? 0.62 : 0.66);
    this.pivot = new THREE.Vector2(t.pegs[0] + (this.fat ? 0 : 0.02), t.pegs[1] + (this.fat ? 0.03 : 0.16));

    this.root.add(this.whip);
    this.whip.position.set(this.head.x, this.head.y, 0);
    this.whip.add(this.parts);
    this.parts.position.set(-this.head.x, -this.head.y, 0);

    this.rearWheel = this.makeWheel(t.rear_wheel_radius, livery);
    this.frontWheel = this.makeWheel(t.front_wheel_radius, livery);
    this.parts.add(this.rearWheel.group, this.frontWheel.group);

    const frameMat = mat(this.fat ? livery.plastic : livery.frame);
    const accent = mat(this.fat ? livery.frame : livery.plastic);
    const dark = mat(0x1a1c20);
    const chrome = mat(0xb9c0c8);

    // Fork: two stanchions either side of the wheel, updated each frame.
    this.fork = [tube(0.028, chrome, 0.09), tube(0.028, chrome, -0.09)];
    this.parts.add(...this.fork);
    // Swingarm (dirt bike) or chainstays (fatbike), updated each frame.
    this.swingarm = [tube(this.fat ? 0.022 : 0.035, frameMat, 0.1), tube(this.fat ? 0.022 : 0.035, frameMat, -0.1)];
    this.parts.add(...this.swingarm);

    this.frontFender = new THREE.Group();
    const r = t.front_wheel_radius;
    this.frontFender.add(boxBetween(-0.22, r + 0.07, 0.2, r + 0.04, 0.02, this.fat ? 0.2 : 0.14, accent));
    this.parts.add(this.frontFender);

    const [hx, hy] = [this.head.x, this.head.y];
    const [px, py] = [this.pivot.x, this.pivot.y];
    const [bx, by] = t.bars;
    const add = (...o: THREE.Object3D[]) => this.parts.add(...o);

    if (this.fat) {
      // E-fatbike: bicycle-style frame, long padded seat, big battery on the down tube.
      const seat = new THREE.Vector2(t.hips[0] - 0.06, t.hips[1] - 0.14);
      const dtube = tube(0.04, frameMat);
      placeTube(dtube, hx, hy, px, py);
      const ttube = tube(0.034, frameMat);
      placeTube(ttube, hx, hy - 0.04, seat.x + 0.08, seat.y - 0.02);
      const stube = tube(0.03, frameMat);
      placeTube(stube, px, py, seat.x, seat.y - 0.04);
      const stays = tube(0.02, frameMat, 0.1);
      placeTube(stays, seat.x, seat.y - 0.06, t.rear_axle[0], t.rear_axle[1]);
      const stays2 = tube(0.02, frameMat, -0.1);
      placeTube(stays2, seat.x, seat.y - 0.06, t.rear_axle[0], t.rear_axle[1]);
      add(dtube, ttube, stube, stays, stays2);
      add(boxBetween(hx - 0.06, hy - 0.1, px + 0.06, py + 0.1, 0.13, 0.12, dark, 0));
      add(boxBetween(seat.x - 0.34, seat.y + 0.03, seat.x + 0.16, seat.y + 0.05, 0.08, 0.2, mat(0x2a2522)));
      add(boxBetween(t.rear_axle[0] - 0.18, t.rear_axle[1] + t.rear_wheel_radius + 0.1, seat.x - 0.2, seat.y - 0.02, 0.03, 0.22, dark));
      // Pedals turn with the rear wheel.
      const crank = new THREE.Group();
      crank.position.set(px, py, 0);
      crank.add(box(0.34, 0.035, 0.03, chrome, 0, 0, 0.13));
      crank.add(box(0.1, 0.03, 0.12, dark, 0.17, 0, 0.18));
      crank.add(box(0.1, 0.03, 0.12, dark, -0.17, 0, -0.18));
      crank.add(ball(0.08, chrome, 0, 0, 0.1));
      this.crank = crank;
      add(crank);
      add(box(0.08, 0.06, 0.08, mat(0xfff3b0), hx + 0.07, hy - 0.02, 0)); // headlight
    } else {
      // E-dirt: slim frame, battery pack, motocross plastics.
      const seatFront = new THREE.Vector2(t.hips[0] + 0.22, t.hips[1] - 0.12);
      const seatRear = new THREE.Vector2(t.hips[0] - 0.36, t.hips[1] - 0.12);
      const d1 = tube(0.03, frameMat, 0.06);
      placeTube(d1, hx, hy, px, py);
      const d2 = tube(0.03, frameMat, -0.06);
      placeTube(d2, hx, hy, px, py);
      const sub = tube(0.022, frameMat);
      placeTube(sub, seatRear.x, seatRear.y - 0.04, px, py + 0.02);
      const top = tube(0.028, frameMat);
      placeTube(top, hx, hy - 0.02, seatFront.x, seatFront.y - 0.05);
      add(d1, d2, sub, top);
      add(boxBetween(hx - 0.1, hy - 0.12, px + 0.12, py + 0.06, 0.24, 0.2, accent)); // battery shroud
      add(boxBetween(seatRear.x, seatRear.y, seatFront.x, seatFront.y, 0.07, 0.2, mat(0x202226))); // seat
      add(boxBetween(seatRear.x - 0.1, seatRear.y - 0.02, seatRear.x - 0.42, seatRear.y - 0.07, 0.025, 0.16, accent)); // tail
      add(boxBetween(hx + 0.02, hy + 0.12, hx + 0.1, hy - 0.08, 0.2, 0.16, accent, 0)); // number plate
      add(ball(0.07, dark, px + 0.03, py - 0.02, 0.07)); // motor
      const shock = tube(0.03, mat(0xffc21f));
      placeTube(shock, seatRear.x + 0.2, seatRear.y - 0.05, px - 0.2, py - 0.02);
      add(shock);
    }
    // Handlebar riser and bars.
    const riser = tube(0.022, chrome);
    placeTube(riser, hx, hy, bx, by);
    add(riser, box(0.04, 0.04, 0.62, dark, bx, by, 0));

    this.root.traverse((o) => {
      if (o instanceof THREE.Mesh) o.castShadow = true;
    });
  }

  private makeWheel(radius: number, livery: Livery): Wheel {
    const group = new THREE.Group();
    const spin = new THREE.Group();
    group.add(spin);
    const tyreTube = this.fat ? 0.075 : 0.05;
    const own = <G extends THREE.BufferGeometry>(g: G): G => {
      this.owned.push(g);
      return g;
    };
    const tyre = new THREE.Mesh(
      own(new THREE.TorusGeometry(radius - tyreTube, tyreTube, 6, this.fat ? 16 : 18)),
      mat(0x1d1d1f),
    );
    tyre.scale.z = this.fat ? 1.7 : 1.1;
    tyre.castShadow = true;
    spin.add(tyre);
    const rimR = radius - tyreTube * 2;
    const rim = new THREE.Mesh(own(new THREE.TorusGeometry(rimR, 0.018, 4, 18)), mat(this.fat ? livery.plastic : 0x9aa3ad));
    rim.scale.z = this.fat ? 3 : 1.4;
    spin.add(rim);
    const spokes = this.fat ? 5 : 9;
    for (let i = 0; i < spokes; i++) {
      const s = box(rimR, this.fat ? 0.03 : 0.012, 0.012, mat(0xc8cdd3));
      s.rotation.z = (i / spokes) * Math.PI;
      spin.add(s);
    }
    spin.add(new THREE.Mesh(own(new THREE.CylinderGeometry(0.05, 0.05, 0.16, 8).rotateX(Math.PI / 2)), mat(0x3a3f47)));
    if (!this.fat) {
      const disc = new THREE.Mesh(own(new THREE.CylinderGeometry(0.1, 0.1, 0.01, 12).rotateX(Math.PI / 2)), mat(0xb9c0c8));
      disc.position.z = 0.07;
      spin.add(disc);
    }
    return { group, spin };
  }

  update(p: BikePose): void {
    this.root.position.set(p.x, p.y, 0);
    this.root.rotation.z = p.angle;
    this.whip.rotation.y = p.whip * Math.PI * 2;

    const c = Math.cos(-p.angle);
    const s = Math.sin(-p.angle);
    const local = (wx: number, wy: number): [number, number] => {
      const dx = wx - p.x;
      const dy = wy - p.y;
      return [c * dx - s * dy, s * dx + c * dy];
    };
    const [rx, ry] = local(p.rear[0], p.rear[1]);
    const [fx, fy] = local(p.front[0], p.front[1]);
    this.rearWheel.group.position.set(rx, ry, 0);
    this.rearWheel.spin.rotation.z = p.rear[2] - p.angle;
    this.frontWheel.group.position.set(fx, fy, 0);
    this.frontWheel.spin.rotation.z = p.front[2] - p.angle;

    for (const f of this.fork) placeTube(f, fx, fy, this.head.x, this.head.y);
    for (const a of this.swingarm) placeTube(a, this.pivot.x, this.pivot.y, rx, ry);
    this.frontFender.position.set(fx, fy, 0);
    this.frontFender.rotation.z = Math.atan2(this.head.y - fy, this.head.x - fx) - Math.PI / 2;
    if (this.crank) this.crank.rotation.z = (p.rear[2] - p.angle) * 0.5;
  }

  dispose(): void {
    for (const g of this.owned) g.dispose();
    this.root.removeFromParent();
  }
}
