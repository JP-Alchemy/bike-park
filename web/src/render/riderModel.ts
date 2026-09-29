// The rider, posed from the simulation's skeleton points (IK while riding, ragdoll after
// a crash). Limbs are stretched primitives; left limbs sit nearer the camera.

import * as THREE from "three";
import { SK } from "../sim/layout";
import { ball, box, mat, placeTube, tube, type Livery } from "./parts";

const HIP_Z = 0.11;
const SHOULDER_Z = 0.17;

type P = [number, number];

export class RiderModel {
  readonly root = new THREE.Group();
  private readonly torso: THREE.Mesh;
  private readonly helmet = new THREE.Group();
  private readonly limbs: { mesh: THREE.Object3D; a: number; b: number; z: number }[] = [];
  private readonly joints: { mesh: THREE.Object3D; p: number; z: number }[] = [];
  private readonly boots: { mesh: THREE.Object3D; knee: number; foot: number; z: number }[] = [];

  constructor(livery: Livery) {
    const jersey = mat(livery.jersey);
    const pants = mat(livery.pants);
    const skin = mat(0x2a2a2e); // gloves
    const bootMat = mat(0x202124);

    this.torso = box(1, 1, 0.34, jersey);
    this.root.add(this.torso);

    const limb = (a: number, b: number, z: number, r: number, m: THREE.Material) => {
      const mesh = tube(r, m, z);
      this.root.add(mesh);
      this.limbs.push({ mesh, a, b, z });
    };
    const joint = (p: number, z: number, r: number, m: THREE.Material) => {
      const mesh = ball(r, m, 0, 0, z);
      this.root.add(mesh);
      this.joints.push({ mesh, p, z });
    };
    // Far side first so near-side limbs draw over them.
    for (const side of [-1, 1]) {
      const knee = side > 0 ? SK.KNEE_L : SK.KNEE_R;
      const foot = side > 0 ? SK.FOOT_L : SK.FOOT_R;
      const elbow = side > 0 ? SK.ELBOW_L : SK.ELBOW_R;
      const hand = side > 0 ? SK.HAND_L : SK.HAND_R;
      limb(SK.HIP, knee, side * HIP_Z, 0.075, pants);
      limb(knee, foot, side * HIP_Z, 0.06, pants);
      joint(knee, side * HIP_Z, 0.07, pants);
      limb(SK.NECK, elbow, side * SHOULDER_Z, 0.05, jersey);
      limb(elbow, hand, side * SHOULDER_Z, 0.045, jersey);
      joint(elbow, side * SHOULDER_Z, 0.05, jersey);
      joint(hand, side * SHOULDER_Z, 0.045, skin);
      const boot = box(0.2, 0.09, 0.1, bootMat);
      boot.position.z = side * HIP_Z;
      this.root.add(boot);
      this.boots.push({ mesh: boot, knee, foot, z: side * HIP_Z });
    }
    joint(SK.HIP, 0, 0.13, pants);

    // Helmet: shell, visor and chin bar facing forward (+x in the torso's frame).
    const shell = ball(0.15, mat(livery.helmet));
    shell.scale.set(0.16, 0.15, 0.14);
    const visor = box(0.1, 0.05, 0.22, mat(0x15181c), 0.1, 0.03, 0);
    const chin = box(0.08, 0.06, 0.2, mat(livery.helmet), 0.12, -0.07, 0);
    const peak = box(0.14, 0.02, 0.2, mat(livery.helmet), 0.08, 0.12, 0, -0.25);
    this.helmet.add(shell, visor, chin, peak);
    this.root.add(this.helmet);
  }

  /** `pts` is the render state; `base` the offset of the skeleton's first point. */
  update(pt: (i: number) => P): void {
    const hip = pt(SK.HIP);
    const neck = pt(SK.NECK);
    const head = pt(SK.HEAD);
    const torsoAngle = Math.atan2(neck[1] - hip[1], neck[0] - hip[0]);
    const len = Math.hypot(neck[0] - hip[0], neck[1] - hip[1]);
    this.torso.position.set((hip[0] + neck[0]) / 2, (hip[1] + neck[1]) / 2, 0);
    this.torso.rotation.z = torsoAngle;
    this.torso.scale.set(len + 0.12, 0.26, 0.34);

    for (const l of this.limbs) {
      const a = pt(l.a);
      const b = pt(l.b);
      placeTube(l.mesh, a[0], a[1], b[0], b[1]);
    }
    for (const j of this.joints) {
      const p = pt(j.p);
      j.mesh.position.set(p[0], p[1], j.z);
    }
    for (const b of this.boots) {
      const knee = pt(b.knee);
      const foot = pt(b.foot);
      // Boot points forward, square to the shin.
      const shin = Math.atan2(foot[1] - knee[1], foot[0] - knee[0]);
      b.mesh.rotation.z = shin + Math.PI / 2;
      b.mesh.position.set(foot[0] + Math.cos(shin + Math.PI / 2) * 0.05, foot[1] + Math.sin(shin + Math.PI / 2) * 0.05, b.z);
    }
    this.helmet.position.set(head[0], head[1], 0);
    this.helmet.rotation.z = torsoAngle - Math.PI / 2;
  }

  dispose(): void {
    this.root.removeFromParent();
  }
}
