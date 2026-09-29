// The park: the track extruded from the simulation's ground profile, plus scenery.
// Scenery is seeded, so every player sees the same park.

import * as THREE from "three";
import type { Track } from "../sim/types";
import { box, mat } from "./parts";

export const TRACK_HALF_WIDTH = 2.2;
const BASE_Y = -30;

/** Small seeded PRNG (mulberry32) so scenery is identical for everyone. */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export class World {
  readonly group = new THREE.Group();
  private readonly clouds: THREE.Group[] = [];
  private readonly checkpointFlags: THREE.Mesh[] = [];
  private shownCheckpoint = -1;

  constructor(private readonly track: Track) {
    this.group.add(this.buildTrack());
    this.group.add(this.buildGround());
    this.buildHills();
    this.buildTrees();
    this.buildClouds();
    this.buildMarkers();
  }

  private buildTrack(): THREE.Mesh {
    const g = this.track.ground;
    const positions: number[] = [];
    const colors: number[] = [];
    const indices: number[] = [];
    const dirt = new THREE.Color(0xc0875a);
    const dirtDark = new THREE.Color(0xa06d45);
    const random = rng(7);
    const c = new THREE.Color();

    // Top surface: rows of (back, rut, centre, rut, front) across the track.
    const zs = [-TRACK_HALF_WIDTH, -0.9, 0, 0.9, TRACK_HALF_WIDTH];
    for (let i = 0; i < g.length; i++) {
      const [x, y] = g[i];
      const n = random() * 0.08;
      for (let k = 0; k < zs.length; k++) {
        positions.push(x, y, zs[k]);
        c.copy(k === 2 ? dirtDark : dirt).offsetHSL(0, 0, n - 0.04);
        colors.push(c.r, c.g, c.b);
      }
    }
    const cols = zs.length;
    for (let i = 0; i < g.length - 1; i++) {
      for (let k = 0; k < cols - 1; k++) {
        const a = i * cols + k;
        const b = (i + 1) * cols + k;
        indices.push(a, a + 1, b, b, a + 1, b + 1);
      }
    }
    // Front face: the cross-section of the ground, in strata that follow the surface.
    const bands: [depth: number, color: number][] = [
      [0, 0x6fae45],
      [0.12, 0x6fae45],
      [0.13, 0xa8764c],
      [0.9, 0x9a6a44],
      [0.91, 0x87593a],
      [2.2, 0x7d5236],
      [2.21, 0x6c4630],
      [4.5, 0x5e3d2a],
      [4.51, 0x533626],
      [30, 0x3a261b],
    ];
    const start = positions.length / 3;
    const bandColor = new THREE.Color();
    for (let i = 0; i < g.length; i++) {
      const [x, y] = g[i];
      for (const [depth, color] of bands) {
        positions.push(x, Math.max(BASE_Y, y - depth), TRACK_HALF_WIDTH);
        bandColor.set(color).offsetHSL(0, 0, (random() - 0.5) * 0.03);
        colors.push(bandColor.r, bandColor.g, bandColor.b);
      }
    }
    const rows = bands.length;
    for (let i = 0; i < g.length - 1; i++) {
      for (let k = 0; k < rows - 1; k++) {
        const a = start + i * rows + k;
        const b = start + (i + 1) * rows + k;
        indices.push(a, a + 1, b, b, a + 1, b + 1);
      }
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
    geo.setAttribute("color", new THREE.Float32BufferAttribute(colors, 3));
    geo.setIndex(indices);
    geo.computeVertexNormals();
    const mesh = new THREE.Mesh(geo, new THREE.MeshLambertMaterial({ vertexColors: true }));
    mesh.receiveShadow = true;
    return mesh;
  }

  private buildGround(): THREE.Mesh {
    const [x0, x1] = this.extent();
    const geo = new THREE.PlaneGeometry(x1 - x0 + 400, 260).rotateX(-Math.PI / 2);
    const mesh = new THREE.Mesh(geo, mat(0x7fbf4d));
    mesh.position.set((x0 + x1) / 2, -0.6, -TRACK_HALF_WIDTH - 130);
    mesh.receiveShadow = true;
    return mesh;
  }

  private extent(): [number, number] {
    const g = this.track.ground;
    return [g[0][0], g[g.length - 1][0]];
  }

  private buildHills(): void {
    const [x0, x1] = this.extent();
    const layers = [
      { z: -45, amp: 7, color: 0x6fb04a, base: -1 },
      { z: -95, amp: 16, color: 0x7fb88a, base: -2 },
      { z: -170, amp: 30, color: 0x9cc3c8, base: -4 },
    ];
    layers.forEach((layer, li) => {
      const random = rng(100 + li);
      const phases = [random() * 6, random() * 6, random() * 6];
      const pts: THREE.Vector2[] = [];
      const from = x0 - 250;
      const to = x1 + 250;
      pts.push(new THREE.Vector2(from, BASE_Y));
      for (let x = from; x <= to; x += 12) {
        const h =
          Math.sin(x * 0.011 + phases[0]) * 0.5 +
          Math.sin(x * 0.027 + phases[1]) * 0.3 +
          Math.sin(x * 0.061 + phases[2]) * 0.2;
        pts.push(new THREE.Vector2(x, layer.base + layer.amp * (0.6 + h)));
      }
      pts.push(new THREE.Vector2(to, BASE_Y));
      const mesh = new THREE.Mesh(new THREE.ShapeGeometry(new THREE.Shape(pts)), mat(layer.color));
      mesh.position.z = layer.z;
      this.group.add(mesh);
    });
  }

  private buildTrees(): void {
    const [x0, x1] = this.extent();
    const random = rng(42);
    const trunks: THREE.Matrix4[] = [];
    const crowns: THREE.Matrix4[] = [];
    const m = new THREE.Matrix4();
    for (let x = x0 - 60; x < x1 + 60; x += 3 + random() * 7) {
      const z = -TRACK_HALF_WIDTH - 5 - random() * 26;
      const s = 0.8 + random() * 0.9;
      trunks.push(m.clone().compose(new THREE.Vector3(x, -0.6, z), new THREE.Quaternion(), new THREE.Vector3(s, s, s)));
      crowns.push(m.clone().compose(new THREE.Vector3(x, -0.6 + 1.2 * s, z), new THREE.Quaternion(), new THREE.Vector3(s, s * (1 + random() * 0.5), s)));
    }
    const trunkGeo = new THREE.CylinderGeometry(0.15, 0.2, 1.4, 5).translate(0, 0.7, 0);
    const crownGeo = new THREE.ConeGeometry(1.1, 3.2, 6).translate(0, 1.6, 0);
    const trunkMesh = new THREE.InstancedMesh(trunkGeo, mat(0x6b4a2f), trunks.length);
    const crownMesh = new THREE.InstancedMesh(crownGeo, mat(0x3f8f3a), crowns.length);
    trunks.forEach((t, i) => trunkMesh.setMatrixAt(i, t));
    crowns.forEach((t, i) => crownMesh.setMatrixAt(i, t));
    crownMesh.castShadow = true;
    this.group.add(trunkMesh, crownMesh);
  }

  private buildClouds(): void {
    const [x0, x1] = this.extent();
    const random = rng(9);
    const white = new THREE.MeshLambertMaterial({ color: 0xffffff, flatShading: true });
    const geo = new THREE.IcosahedronGeometry(1, 0);
    for (let i = 0; i < 18; i++) {
      const cloud = new THREE.Group();
      for (let k = 0; k < 4; k++) {
        const puff = new THREE.Mesh(geo, white);
        puff.position.set(k * 3 - 4.5 + random() * 2, random() * 1.5, random() * 2);
        puff.scale.set(3 + random() * 2, 1.6 + random(), 2);
        cloud.add(puff);
      }
      cloud.position.set(x0 + random() * (x1 - x0 + 200) - 100, 28 + random() * 22, -140 - random() * 40);
      this.clouds.push(cloud);
      this.group.add(cloud);
    }
  }

  private buildMarkers(): void {
    const t = this.track;
    const groundY = (x: number) => groundHeight(t, x);

    // Section signs behind the track.
    for (const zone of t.zones) {
      const x = zone.x0 + 3;
      const sign = new THREE.Group();
      const board = new THREE.Mesh(new THREE.PlaneGeometry(3.6, 1.1), new THREE.MeshBasicMaterial({ map: labelTexture(zone.label, "#ffd21f", "#1a1c20") }));
      board.position.set(0, 2.3, 0.06);
      sign.add(board, box(3.8, 1.3, 0.1, mat(0x1a1c20), 0, 2.3, 0), box(0.12, 2.3, 0.12, mat(0x5a4632), -1.4, 1.1, 0), box(0.12, 2.3, 0.12, mat(0x5a4632), 1.4, 1.1, 0));
      sign.position.set(x, groundY(x), -TRACK_HALF_WIDTH - 1.2);
      this.group.add(sign);

      if (zone.kind === "wheelie_strip") {
        for (let d = 10; d < zone.x1 - zone.x0; d += 10) {
          const mx = zone.x0 + d;
          const line = new THREE.Mesh(new THREE.PlaneGeometry(0.25, TRACK_HALF_WIDTH * 2).rotateX(-Math.PI / 2), new THREE.MeshBasicMaterial({ color: 0xffffff }));
          line.position.set(mx, groundY(mx) + 0.02, 0);
          this.group.add(line);
          const label = new THREE.Mesh(new THREE.PlaneGeometry(1.3, 0.65), new THREE.MeshBasicMaterial({ map: labelTexture(`${d} m`, "#ffffff", "#1f6fff"), transparent: true }));
          label.position.set(mx, groundY(mx) + 0.6, -TRACK_HALF_WIDTH - 0.3);
          this.group.add(label);
        }
      }
    }

    // Checkpoint flags.
    t.checkpoints.forEach((x, i) => {
      const flag = new THREE.Group();
      flag.add(box(0.08, 2.4, 0.08, mat(0xe8e8e8), 0, 1.2, 0));
      const cloth = box(0.7, 0.45, 0.03, mat(i === 0 ? 0x1f6fff : 0xff7a1a), 0.38, 2.1, 0);
      this.checkpointFlags.push(cloth);
      flag.add(cloth);
      flag.position.set(x, groundY(x), -TRACK_HALF_WIDTH - 0.2);
      this.group.add(flag);
    });

    // Finish: a checkered line and a banner.
    const fx = t.finish_x;
    const checker = checkerTexture();
    const line = new THREE.Mesh(new THREE.PlaneGeometry(1.2, TRACK_HALF_WIDTH * 2).rotateX(-Math.PI / 2), new THREE.MeshBasicMaterial({ map: checker }));
    line.position.set(fx, groundY(fx) + 0.02, 0);
    const banner = new THREE.Mesh(new THREE.PlaneGeometry(5, 1.3), new THREE.MeshBasicMaterial({ map: labelTexture("FINISH", "#ffffff", "#e8413c") }));
    banner.position.set(fx, groundY(fx) + 4.2, -TRACK_HALF_WIDTH - 0.5);
    const postA = box(0.18, 4.8, 0.18, mat(0x1a1c20), fx - 2.6, groundY(fx) + 2.4, -TRACK_HALF_WIDTH - 0.5);
    const postB = box(0.18, 4.8, 0.18, mat(0x1a1c20), fx + 2.6, groundY(fx) + 2.4, -TRACK_HALF_WIDTH - 0.5);
    this.group.add(line, banner, postA, postB);
  }

  /** Colours passed checkpoints green; drifts the clouds. */
  update(dt: number, checkpoint: number): void {
    if (checkpoint !== this.shownCheckpoint) {
      this.shownCheckpoint = checkpoint;
      this.checkpointFlags.forEach((f, i) => {
        if (i > 0) f.material = mat(i <= checkpoint ? 0x2fd67a : 0xff7a1a);
      });
    }
    for (const c of this.clouds) c.position.x += dt * 0.6;
  }
}

export function groundHeight(t: Track, x: number): number {
  const g = t.ground;
  if (x <= g[0][0]) return g[0][1];
  if (x >= g[g.length - 1][0]) return g[g.length - 1][1];
  let lo = 0;
  let hi = g.length - 1;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (g[mid][0] <= x) lo = mid;
    else hi = mid;
  }
  const [ax, ay] = g[lo];
  const [bx, by] = g[hi];
  return ay + ((by - ay) * (x - ax)) / (bx - ax);
}

function labelTexture(text: string, fg: string, bg: string): THREE.CanvasTexture {
  const canvas = document.createElement("canvas");
  canvas.width = 512;
  canvas.height = 160;
  const ctx = canvas.getContext("2d")!;
  ctx.fillStyle = bg;
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = fg;
  ctx.font = "italic 900 88px system-ui, 'Segoe UI', Roboto, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(text.toUpperCase(), canvas.width / 2, canvas.height / 2 + 4, canvas.width - 40);
  const tex = new THREE.CanvasTexture(canvas);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

function checkerTexture(): THREE.CanvasTexture {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 128;
  const ctx = canvas.getContext("2d")!;
  for (let y = 0; y < 8; y++) {
    for (let x = 0; x < 4; x++) {
      ctx.fillStyle = (x + y) % 2 ? "#111" : "#fff";
      ctx.fillRect(x * 16, y * 16, 16, 16);
    }
  }
  const tex = new THREE.CanvasTexture(canvas);
  tex.magFilter = THREE.NearestFilter;
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}
