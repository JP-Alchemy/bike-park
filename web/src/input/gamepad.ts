import type { ControlState } from "./input";

const DEADZONE = 0.2;

/** Standard-mapping gamepads: RT throttle, LT brake, left stick or d-pad to lean,
 *  A trick, Y respawn. D-pad up/down also work as throttle/brake. */
export class Gamepads {
  read(): ControlState {
    const out: ControlState = { throttle: 0, brake: 0, lean: 0, trick: false, respawn: false };
    const pads = navigator.getGamepads ? navigator.getGamepads() : [];
    for (const pad of pads) {
      if (!pad || pad.mapping !== "standard") continue;
      const b = (i: number) => pad.buttons[i]?.value ?? 0;
      const pressed = (i: number) => pad.buttons[i]?.pressed ?? false;
      const stick = pad.axes[0] ?? 0;
      const lean = Math.abs(stick) > DEADZONE ? stick : 0;
      out.throttle = Math.max(out.throttle, b(7), pressed(12) ? 1 : 0);
      out.brake = Math.max(out.brake, b(6), pressed(13) ? 1 : 0);
      out.lean += lean + (pressed(15) ? 1 : 0) - (pressed(14) ? 1 : 0);
      out.trick ||= pressed(0);
      out.respawn ||= pressed(3);
    }
    out.lean = Math.max(-1, Math.min(1, out.lean));
    return out;
  }
}
