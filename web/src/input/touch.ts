import type { ControlState } from "./input";

type Control = "back" | "forward" | "brake" | "throttle" | "trick" | "respawn";

const LAYOUT: { control: Control; label: string; side: "left" | "right" | "top" }[] = [
  { control: "back", label: "◀", side: "left" },
  { control: "forward", label: "▶", side: "left" },
  { control: "brake", label: "▼", side: "right" },
  { control: "throttle", label: "▲", side: "right" },
  { control: "trick", label: "★", side: "right" },
  { control: "respawn", label: "↺", side: "top" },
];

/** On-screen buttons, shown once the device is touched. Multi-touch: each button tracks
 *  its own pointers, so thumbs can hold throttle and lean at the same time. */
export class TouchControls {
  private readonly held = new Map<Control, Set<number>>();

  constructor(root: HTMLElement) {
    const groups = {
      left: el("div", "touch-group touch-left"),
      right: el("div", "touch-group touch-right"),
      top: el("div", "touch-group touch-top"),
    };
    for (const { control, label, side } of LAYOUT) {
      const button = el("button", `touch-btn touch-${control}`);
      button.textContent = label;
      button.setAttribute("aria-label", control);
      const pointers = new Set<number>();
      this.held.set(control, pointers);
      const release = (e: PointerEvent) => {
        pointers.delete(e.pointerId);
        button.classList.toggle("held", pointers.size > 0);
      };
      button.addEventListener("pointerdown", (e) => {
        e.preventDefault();
        button.setPointerCapture(e.pointerId);
        pointers.add(e.pointerId);
        button.classList.add("held");
      });
      button.addEventListener("pointerup", release);
      button.addEventListener("pointercancel", release);
      button.addEventListener("lostpointercapture", release);
      button.addEventListener("contextmenu", (e) => e.preventDefault());
      groups[side].appendChild(button);
    }
    for (const g of Object.values(groups)) root.appendChild(g);
    const show = () => document.body.classList.add("touch");
    window.addEventListener("touchstart", show, { once: true, passive: true });
    if (matchMedia("(pointer: coarse)").matches) show();
  }

  private on(c: Control): boolean {
    return (this.held.get(c)?.size ?? 0) > 0;
  }

  read(): ControlState {
    return {
      throttle: this.on("throttle") ? 1 : 0,
      brake: this.on("brake") ? 1 : 0,
      lean: (this.on("forward") ? 1 : 0) - (this.on("back") ? 1 : 0),
      trick: this.on("trick"),
      respawn: this.on("respawn"),
    };
  }
}

function el(tag: string, className: string): HTMLElement {
  const e = document.createElement(tag);
  e.className = className;
  return e;
}
