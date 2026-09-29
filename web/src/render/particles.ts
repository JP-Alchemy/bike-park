// A small pool of dust particles (one draw call) for wheel spin, landings and crashes.

import * as THREE from "three";

const MAX = 400;

export class Dust {
  readonly points: THREE.Points;
  private readonly pos = new Float32Array(MAX * 3);
  private readonly vel = new Float32Array(MAX * 3);
  private readonly life = new Float32Array(MAX);
  private readonly maxLife = new Float32Array(MAX);
  private readonly alpha = new Float32Array(MAX);
  private readonly size = new Float32Array(MAX);
  private next = 0;

  constructor() {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(this.pos, 3));
    geo.setAttribute("alpha", new THREE.BufferAttribute(this.alpha, 1));
    geo.setAttribute("size", new THREE.BufferAttribute(this.size, 1));
    const material = new THREE.ShaderMaterial({
      transparent: true,
      depthWrite: false,
      uniforms: { color: { value: new THREE.Color(0xd9b38c) }, scale: { value: 300 } },
      vertexShader: `
        attribute float alpha;
        attribute float size;
        varying float vAlpha;
        uniform float scale;
        void main() {
          vAlpha = alpha;
          vec4 mv = modelViewMatrix * vec4(position, 1.0);
          gl_PointSize = size * scale / -mv.z;
          gl_Position = projectionMatrix * mv;
        }`,
      fragmentShader: `
        uniform vec3 color;
        varying float vAlpha;
        void main() {
          float d = length(gl_PointCoord - 0.5);
          if (d > 0.5) discard;
          gl_FragColor = vec4(color, vAlpha * smoothstep(0.5, 0.2, d));
        }`,
    });
    this.points = new THREE.Points(geo, material);
    this.points.frustumCulled = false;
  }

  emit(x: number, y: number, vx: number, vy: number, count: number, spread = 1): void {
    for (let n = 0; n < count; n++) {
      const i = this.next;
      this.next = (this.next + 1) % MAX;
      this.pos[i * 3] = x + (Math.random() - 0.5) * 0.3;
      this.pos[i * 3 + 1] = y + Math.random() * 0.1;
      this.pos[i * 3 + 2] = (Math.random() - 0.5) * 1.6;
      this.vel[i * 3] = vx + (Math.random() - 0.5) * 2 * spread;
      this.vel[i * 3 + 1] = vy + Math.random() * 1.5 * spread;
      this.vel[i * 3 + 2] = (Math.random() - 0.5) * 1.5 * spread;
      this.maxLife[i] = this.life[i] = 0.5 + Math.random() * 0.7;
      this.size[i] = 0.25 + Math.random() * 0.35;
    }
  }

  update(dt: number): void {
    for (let i = 0; i < MAX; i++) {
      if (this.life[i] <= 0) {
        this.alpha[i] = 0;
        continue;
      }
      this.life[i] -= dt;
      const k = i * 3;
      this.vel[k] *= 1 - 2.5 * dt;
      this.vel[k + 1] = this.vel[k + 1] * (1 - 2.5 * dt) + 0.6 * dt;
      this.pos[k] += this.vel[k] * dt;
      this.pos[k + 1] += this.vel[k + 1] * dt;
      this.pos[k + 2] += this.vel[k + 2] * dt;
      const t = Math.max(0, this.life[i] / this.maxLife[i]);
      this.alpha[i] = 0.55 * t;
      this.size[i] += dt * 0.8;
    }
    const geo = this.points.geometry;
    geo.attributes.position.needsUpdate = true;
    geo.attributes.alpha.needsUpdate = true;
    geo.attributes.size.needsUpdate = true;
  }
}
