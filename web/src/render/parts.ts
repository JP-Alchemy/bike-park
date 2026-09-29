// Small helpers for building low-poly models out of primitives.

import * as THREE from "three";

const materialCache = new Map<string, THREE.MeshLambertMaterial>();

/** Flat-shaded Lambert material, shared per colour: cheap on school Chromebooks. */
export function mat(color: THREE.ColorRepresentation): THREE.MeshLambertMaterial {
  const key = new THREE.Color(color).getHexString();
  let m = materialCache.get(key);
  if (!m) {
    m = new THREE.MeshLambertMaterial({ color, flatShading: true });
    materialCache.set(key, m);
  }
  return m;
}

// Unit shapes, scaled per use.
const unitCylinder = new THREE.CylinderGeometry(1, 1, 1, 7);
unitCylinder.translate(0, 0.5, 0);
const unitBox = new THREE.BoxGeometry(1, 1, 1);
const unitSphere = new THREE.IcosahedronGeometry(1, 1);

/** A tube from (0,0) upwards; place it between two points with `placeTube`. */
export function tube(radius: number, material: THREE.Material, z = 0): THREE.Mesh {
  const m = new THREE.Mesh(unitCylinder, material);
  m.scale.set(radius, 1, radius);
  m.position.z = z;
  m.castShadow = true;
  return m;
}

/** Stretches a `tube` so it runs from a to b (in its parent's x/y plane). */
export function placeTube(m: THREE.Object3D, ax: number, ay: number, bx: number, by: number): void {
  const dx = bx - ax;
  const dy = by - ay;
  m.position.x = ax;
  m.position.y = ay;
  m.scale.y = Math.max(1e-3, Math.hypot(dx, dy));
  m.rotation.z = Math.atan2(dy, dx) - Math.PI / 2;
}

export function box(
  w: number,
  h: number,
  d: number,
  material: THREE.Material,
  x = 0,
  y = 0,
  z = 0,
  rotZ = 0,
): THREE.Mesh {
  const m = new THREE.Mesh(unitBox, material);
  m.scale.set(w, h, d);
  m.position.set(x, y, z);
  m.rotation.z = rotZ;
  m.castShadow = true;
  return m;
}

export function ball(r: number, material: THREE.Material, x = 0, y = 0, z = 0): THREE.Mesh {
  const m = new THREE.Mesh(unitSphere, material);
  m.scale.setScalar(r);
  m.position.set(x, y, z);
  m.castShadow = true;
  return m;
}

/** Box spanning two points (a plate or a seat), `thickness` across, `depth` in z. */
export function boxBetween(
  ax: number,
  ay: number,
  bx: number,
  by: number,
  thickness: number,
  depth: number,
  material: THREE.Material,
  z = 0,
): THREE.Mesh {
  const len = Math.hypot(bx - ax, by - ay);
  return box(len, thickness, depth, material, (ax + bx) / 2, (ay + by) / 2, z, Math.atan2(by - ay, bx - ax));
}

/** Interpolates angles along the short way round. */
export function lerpAngle(a: number, b: number, t: number): number {
  let d = b - a;
  while (d > Math.PI) d -= Math.PI * 2;
  while (d < -Math.PI) d += Math.PI * 2;
  return a + d * t;
}

/** Colour schemes for riders; index 0 is the local player. */
export const LIVERIES = [
  { plastic: 0xff7a1a, frame: 0x23272e, jersey: 0x1f6fff, pants: 0x1b2433, helmet: 0xffd21f },
  { plastic: 0x8fe03a, frame: 0x2b2f36, jersey: 0xe8413c, pants: 0x2a2a2a, helmet: 0xffffff },
  { plastic: 0x2ad4c6, frame: 0x1d2a33, jersey: 0xffc21f, pants: 0x3a2f5c, helmet: 0xe8413c },
  { plastic: 0xc05bff, frame: 0x26222e, jersey: 0x2fd67a, pants: 0x222831, helmet: 0x1f6fff },
  { plastic: 0xff4f8b, frame: 0x2b2b2b, jersey: 0xfafafa, pants: 0x2f3f5f, helmet: 0x8fe03a },
  { plastic: 0xffd21f, frame: 0x2a2f2a, jersey: 0x7a5cff, pants: 0x1f1f1f, helmet: 0xff7a1a },
];

export type Livery = (typeof LIVERIES)[number];
