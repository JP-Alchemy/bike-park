import type { ControlState } from "./input";

// Arrows or WASD to ride, Space (or Shift/J) for tricks, R to respawn.
const THROTTLE = ["ArrowUp", "KeyW"];
const BRAKE = ["ArrowDown", "KeyS"];
const LEAN_BACK = ["ArrowLeft", "KeyA"];
const LEAN_FORWARD = ["ArrowRight", "KeyD"];
const TRICK = ["Space", "ShiftLeft", "ShiftRight", "KeyJ"];
const RESPAWN = ["KeyR", "Backspace"];
const GAME_KEYS = new Set([...THROTTLE, ...BRAKE, ...LEAN_BACK, ...LEAN_FORWARD, ...TRICK, ...RESPAWN]);

export class Keyboard {
  private readonly down = new Set<string>();
  private readonly actions = new Map<string, () => void>();

  constructor() {
    window.addEventListener("keydown", (e) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
      if (GAME_KEYS.has(e.code) || this.actions.has(e.code)) e.preventDefault();
      if (!e.repeat) this.actions.get(e.code)?.();
      this.down.add(e.code);
    });
    window.addEventListener("keyup", (e) => this.down.delete(e.code));
    // Releasing keys while the tab is in the background must not leave the throttle stuck.
    window.addEventListener("blur", () => this.down.clear());
  }

  /** Registers a one-shot action for a key press (menus, toggles). */
  on(code: string, action: () => void): void {
    this.actions.set(code, action);
  }

  private any(codes: string[]): boolean {
    return codes.some((c) => this.down.has(c));
  }

  read(): ControlState {
    return {
      throttle: this.any(THROTTLE) ? 1 : 0,
      brake: this.any(BRAKE) ? 1 : 0,
      lean: (this.any(LEAN_FORWARD) ? 1 : 0) - (this.any(LEAN_BACK) ? 1 : 0),
      trick: this.any(TRICK),
      respawn: this.any(RESPAWN),
    };
  }
}
