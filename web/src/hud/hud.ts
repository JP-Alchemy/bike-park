// The heads-up display: plain DOM over the canvas, updated once per frame.

import type { CrashReason, HudState, SimEvent, TrickKind } from "../sim/types";

const TRICK_NAMES: Record<TrickKind, string> = {
  no_hander: "No-Hander",
  superman: "Superman",
  can_can: "Can-Can",
  tail_whip: "Tail-Whip",
};

const CRASH_LINES: Record<CrashReason, string[]> = {
  head: ["OUCH!", "WIPEOUT!", "SLAM!", "YARD SALE!"],
  bail: ["BAILED!", "TOO LATE!"],
  flipped: ["FLIPPED!"],
  out_of_bounds: ["WHOOPS!"],
};

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls: string, parent?: HTMLElement, text?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  e.className = cls;
  if (text) e.textContent = text;
  parent?.appendChild(e);
  return e;
}

export function formatTime(t: number): string {
  const m = Math.floor(t / 60);
  const s = t - m * 60;
  return `${m}:${s.toFixed(2).padStart(5, "0")}`;
}

export class Hud {
  private readonly score: HTMLElement;
  private readonly combo: HTMLElement;
  private readonly comboPoints: HTMLElement;
  private readonly comboMult: HTMLElement;
  private readonly comboBar: HTMLElement;
  private readonly comboList: HTMLElement;
  private readonly timer: HTMLElement;
  private readonly best: HTMLElement;
  private readonly progressFill: HTMLElement;
  private readonly progressTicks: HTMLElement;
  private readonly speed: HTMLElement;
  private readonly popups: HTMLElement;
  private readonly air: HTMLElement;
  private readonly meter: HTMLElement;
  private readonly needle: HTMLElement;
  private readonly meterDist: HTMLElement;
  private readonly meterBest: HTMLElement;
  private readonly help: HTMLElement;
  readonly buttons: {
    bike: HTMLButtonElement;
    bots: HTMLButtonElement;
    tuning: HTMLButtonElement;
    sound: HTMLButtonElement;
    help: HTMLButtonElement;
  };
  private shownScore = 0;
  private helpTimer = 0;
  checkpoint = 0;

  constructor(root: HTMLElement) {
    const left = el("div", "hud-left", root);
    const scoreBox = el("div", "score-box", left);
    el("span", "label", scoreBox, "SCORE");
    this.score = el("span", "score", scoreBox, "0");
    this.combo = el("div", "combo hidden", left);
    const comboHead = el("div", "combo-head", this.combo);
    this.comboPoints = el("span", "combo-points", comboHead, "0");
    this.comboMult = el("span", "combo-mult", comboHead, "x1");
    const bar = el("div", "combo-bar", this.combo);
    this.comboBar = el("i", "", bar);
    this.comboList = el("ul", "combo-list", this.combo);

    const center = el("div", "hud-center", root);
    const times = el("div", "times", center);
    this.timer = el("span", "timer", times, "0:00.00");
    this.best = el("span", "best", times, "");
    const progress = el("div", "progress", center);
    this.progressFill = el("div", "progress-fill", progress);
    this.progressTicks = el("div", "progress-ticks", progress);

    const right = el("div", "hud-right", root);
    const speedBox = el("div", "speed-box", right);
    this.speed = el("b", "speed", speedBox, "0");
    el("span", "unit", speedBox, "km/h");
    const buttons = el("div", "hud-buttons", right);
    const button = (label: string, title: string) => {
      const b = el("button", "hud-btn", buttons, label);
      b.title = title;
      return b;
    };
    this.buttons = {
      bike: button("E-Dirt", "Switch bike (1 / 2)"),
      bots: button("Bots 3", "More or fewer bots (B)"),
      tuning: button("Tune", "Tuning panel (T)"),
      sound: button("🔊", "Sound on/off (M)"),
      help: button("?", "Controls (H)"),
    };

    this.popups = el("div", "popups", root);
    this.air = el("div", "air hidden", root);
    this.meter = el("div", "meter hidden", root);
    const track = el("div", "meter-track", this.meter);
    el("div", "meter-sweet", track);
    this.needle = el("div", "meter-needle", track);
    this.meterDist = el("div", "meter-dist", this.meter, "0.0 m");
    this.meterBest = el("div", "meter-best", this.meter, "");

    this.help = el("div", "help", root);
    this.help.innerHTML = `
      <div class="help-title">Ride!</div>
      <div class="help-keys">
        <span><kbd>↑</kbd> go</span><span><kbd>↓</kbd> brake</span>
        <span><kbd>←</kbd><kbd>→</kbd> lean</span><span><kbd>Space</kbd> trick</span>
        <span><kbd>R</kbd> respawn</span>
      </div>
      <div class="help-tips">Lean back + go = wheelie · lean in the air to flip · Space + ↑ ↓ ← for other tricks</div>
      <div class="help-touch">◀ ▶ lean · ▲ go · ▼ brake · ★ trick</div>`;
  }

  /** Marks checkpoints on the progress bar (fractions 0..1). */
  setCheckpoints(fractions: number[]): void {
    this.progressTicks.innerHTML = "";
    for (const f of fractions) {
      const tick = el("i", "", this.progressTicks);
      tick.style.left = `${(f * 100).toFixed(2)}%`;
    }
  }

  setBike(name: string): void {
    this.buttons.bike.textContent = name;
  }

  setBots(n: number): void {
    this.buttons.bots.textContent = `Bots ${n}`;
  }

  setMuted(muted: boolean): void {
    this.buttons.sound.textContent = muted ? "🔇" : "🔊";
  }

  toggleHelp(show?: boolean): void {
    this.help.classList.toggle("hidden", show === undefined ? !this.help.classList.contains("hidden") : !show);
    this.helpTimer = 0;
  }

  update(h: HudState, dt: number, inputUsed: boolean): void {
    // Score counts up rather than jumping.
    this.shownScore += (h.total - this.shownScore) * Math.min(1, dt * 8);
    if (Math.abs(h.total - this.shownScore) < 1) this.shownScore = h.total;
    this.score.textContent = Math.round(this.shownScore).toLocaleString("en");

    const comboOpen = h.combo_tricks > 0;
    this.combo.classList.toggle("hidden", !comboOpen);
    if (comboOpen) {
      this.comboPoints.textContent = h.combo_points.toLocaleString("en");
      this.comboMult.textContent = `x${h.multiplier}`;
      this.comboBar.style.transform = `scaleX(${h.combo_timer.toFixed(3)})`;
    } else if (this.comboList.childElementCount) {
      this.comboList.innerHTML = "";
    }

    this.timer.textContent = h.run_time === null ? "0:00.00" : formatTime(h.run_time);
    this.timer.classList.toggle("done", h.finished);
    this.best.textContent = h.best_run === null ? "" : `BEST ${formatTime(h.best_run)}`;
    this.progressFill.style.transform = `scaleX(${h.progress.toFixed(4)})`;
    this.speed.textContent = String(Math.round(h.speed * 3.6));
    this.checkpoint = h.checkpoint;

    // In the air: rotation counter and the trick being held.
    const deg = Math.round((Math.abs(h.air_rotation) * 180) / Math.PI);
    const showAir = h.in_air && !h.crashed && (deg >= 90 || h.trick !== null);
    this.air.classList.toggle("hidden", !showAir);
    if (showAir) {
      const trick = h.trick ? `${TRICK_NAMES[h.trick]}${h.trick_done ? " ✓" : ""}` : "";
      this.air.textContent = [deg >= 90 ? `↺ ${deg}°` : "", trick].filter(Boolean).join("  ·  ");
      this.air.classList.toggle("done", h.trick_done);
    }

    // Wheelie meter: needle in the middle means right on the balance point.
    const showMeter = h.manual !== null && !h.crashed;
    this.meter.classList.toggle("hidden", !showMeter);
    if (showMeter) {
      // Left: falling back (loop-out / rear wheel dropping). Right: falling forward.
      const b = h.balance;
      this.needle.style.left = `${(50 + b * 48).toFixed(1)}%`;
      this.meterDist.textContent = `${h.manual === "wheelie" ? "WHEELIE" : "STOPPIE"} ${h.manual_distance.toFixed(1)} m`;
      this.meterBest.textContent = h.best_wheelie > 0 ? `best ${h.best_wheelie.toFixed(1)} m` : "";
      this.meter.classList.toggle("sweet", Math.abs(b) < 0.3);
    }

    // Help fades once the player has been riding for a few seconds.
    if (inputUsed && !this.help.classList.contains("hidden")) {
      this.helpTimer += dt;
      if (this.helpTimer > 7) this.toggleHelp(false);
    }
  }

  onEvent(e: SimEvent): void {
    switch (e.type) {
      case "trick_landed": {
        this.popup(`${e.name.toUpperCase()}`, `+${e.points}`, "trick");
        const li = el("li", "", this.comboList, e.name);
        this.comboList.prepend(li);
        while (this.comboList.childElementCount > 4) this.comboList.lastElementChild?.remove();
        break;
      }
      case "combo_banked":
        if (e.tricks > 1) this.popup(`COMBO x${e.multiplier}`, `+${e.points.toLocaleString("en")}`, "combo");
        break;
      case "combo_lost":
        if (e.points > 0) this.popup("COMBO LOST", `-${e.points}`, "lost");
        break;
      case "crash": {
        const lines = CRASH_LINES[e.reason];
        this.popup(lines[Math.floor(Math.random() * lines.length)], "", "crash");
        break;
      }
      case "finish":
        this.popup(e.best ? "NEW BEST!" : "FINISH!", formatTime(e.time), "finish");
        break;
      case "checkpoint":
        this.popup("CHECKPOINT", "", "small");
        break;
      case "trick_started":
        break;
    }
  }

  private popup(title: string, sub: string, kind: string): void {
    const p = el("div", `popup popup-${kind}`, this.popups);
    el("div", "popup-title", p, title);
    if (sub) el("div", "popup-sub", p, sub);
    p.addEventListener("animationend", () => p.remove());
    while (this.popups.childElementCount > 4) this.popups.firstElementChild?.remove();
  }
}
