// Side-view follow camera: leads in the direction of travel, pulls back with speed,
// frames portrait phones as well as wide laptop screens, and shakes on big hits.

import * as THREE from "three";

const FOV = 36;

export class FollowCamera {
  readonly camera = new THREE.PerspectiveCamera(FOV, 1, 0.5, 700);
  private readonly focus = new THREE.Vector2();
  private lead = 0;
  private width = 18;
  private shake = 0;
  private initialized = false;

  resize(aspect: number): void {
    this.camera.aspect = aspect;
    this.camera.updateProjectionMatrix();
  }

  /** Adds screen shake (0..1). */
  kick(amount: number): void {
    this.shake = Math.min(1, this.shake + amount);
  }

  update(dt: number, x: number, y: number, vx: number, speed: number): void {
    if (!this.initialized) {
      this.focus.set(x, y);
      this.initialized = true;
    }
    const k = 1 - Math.exp(-dt * 6);
    this.focus.x += (x - this.focus.x) * k;
    this.focus.y += (y - this.focus.y) * (1 - Math.exp(-dt * 4));
    this.lead += (Math.max(-3, Math.min(6, vx * 0.35)) - this.lead) * (1 - Math.exp(-dt * 2));
    const targetWidth = 16 + Math.min(speed, 20) * 0.75;
    this.width += (targetWidth - this.width) * (1 - Math.exp(-dt * 1.5));

    // Distance that shows `width` metres across, but never more than ~18 m top to
    // bottom (portrait phones) nor less than ~11 m (ultra-wide screens).
    const tanHalf = Math.tan(THREE.MathUtils.degToRad(FOV / 2));
    const aspect = this.camera.aspect;
    const fit = this.width / (2 * tanHalf * aspect);
    const dist = Math.min(18 / (2 * tanHalf), Math.max(fit, 11 / (2 * tanHalf)));

    this.shake = Math.max(0, this.shake - dt * 2.5);
    const s = this.shake * this.shake * 0.35;
    const jx = (Math.random() - 0.5) * s;
    const jy = (Math.random() - 0.5) * s;
    const cx = this.focus.x + this.lead;
    const cy = this.focus.y + 1.0;
    this.camera.position.set(cx + jx, cy + dist * 0.12 + jy, dist);
    this.camera.lookAt(cx + jx, cy + jy, 0);
  }

  /** Jump straight to the target (e.g. after a respawn far away). */
  snap(): void {
    this.initialized = false;
  }
}
