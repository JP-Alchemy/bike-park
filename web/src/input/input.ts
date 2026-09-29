// Keyboard, touch and gamepad merged into one set of bike controls. The whole game is
// playable with only a Chromebook keyboard; touch and gamepad work from day one.

import { Gamepads } from "./gamepad";
import { Keyboard } from "./keyboard";
import { TouchControls } from "./touch";

export interface ControlState {
  /** 0..1 */
  throttle: number;
  /** 0..1 */
  brake: number;
  /** -1 (lean back) .. 1 (lean forward) */
  lean: number;
  trick: boolean;
  respawn: boolean;
}

export const NO_CONTROLS: ControlState = { throttle: 0, brake: 0, lean: 0, trick: false, respawn: false };

export class Input {
  readonly keyboard: Keyboard;
  readonly touch: TouchControls;
  readonly gamepads = new Gamepads();
  /** Set once the player touches any control; used to fade the help. */
  used = false;

  constructor(touchRoot: HTMLElement) {
    this.keyboard = new Keyboard();
    this.touch = new TouchControls(touchRoot);
  }

  read(): ControlState {
    const k = this.keyboard.read();
    const t = this.touch.read();
    const g = this.gamepads.read();
    const c: ControlState = {
      throttle: Math.max(k.throttle, t.throttle, g.throttle),
      brake: Math.max(k.brake, t.brake, g.brake),
      lean: Math.max(-1, Math.min(1, k.lean + t.lean + g.lean)),
      trick: k.trick || t.trick || g.trick,
      respawn: k.respawn || t.respawn || g.respawn,
    };
    if (c.throttle || c.brake || c.lean || c.trick) this.used = true;
    return c;
  }
}
