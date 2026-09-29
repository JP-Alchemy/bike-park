// Synthesised sound: no audio files to download. An electric whine that follows the
// wheel speed, tyre rumble, and short effects for landings, tricks and crashes.
// Browsers only allow audio after a user gesture, so everything starts on `unlock()`.

export class Sound {
  private ctx?: AudioContext;
  private master?: GainNode;
  private whine?: OscillatorNode;
  private whine2?: OscillatorNode;
  private whineGain?: GainNode;
  private rumbleGain?: GainNode;
  private rumbleFilter?: BiquadFilterNode;
  private noise?: AudioBuffer;
  muted = false;

  constructor() {
    try {
      this.muted = localStorage.getItem("bikepark.muted") === "1";
    } catch {
      // Storage can be unavailable (private mode, blocked site data): sound stays on.
    }
  }

  unlock(): void {
    if (this.ctx) {
      if (this.ctx.state === "suspended") void this.ctx.resume();
      return;
    }
    const Ctx = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!Ctx) return;
    const ctx = new Ctx();
    this.ctx = ctx;
    this.master = ctx.createGain();
    this.master.gain.value = this.muted ? 0 : 0.8;
    this.master.connect(ctx.destination);

    // Motor: two detuned oscillators through a band-pass, like an e-motor's whine.
    const filter = ctx.createBiquadFilter();
    filter.type = "bandpass";
    filter.frequency.value = 900;
    filter.Q.value = 0.8;
    this.whineGain = ctx.createGain();
    this.whineGain.gain.value = 0;
    this.whine = ctx.createOscillator();
    this.whine.type = "sawtooth";
    this.whine2 = ctx.createOscillator();
    this.whine2.type = "square";
    this.whine.connect(filter);
    this.whine2.connect(filter);
    filter.connect(this.whineGain).connect(this.master);
    this.whine.start();
    this.whine2.start();

    // Tyre rumble: looping brown noise through a low-pass.
    const len = ctx.sampleRate * 2;
    this.noise = ctx.createBuffer(1, len, ctx.sampleRate);
    const data = this.noise.getChannelData(0);
    let last = 0;
    for (let i = 0; i < len; i++) {
      last = (last + 0.02 * (Math.random() * 2 - 1)) / 1.02;
      data[i] = last * 3.5;
    }
    const rumble = ctx.createBufferSource();
    rumble.buffer = this.noise;
    rumble.loop = true;
    this.rumbleFilter = ctx.createBiquadFilter();
    this.rumbleFilter.type = "lowpass";
    this.rumbleFilter.frequency.value = 300;
    this.rumbleGain = ctx.createGain();
    this.rumbleGain.gain.value = 0;
    rumble.connect(this.rumbleFilter).connect(this.rumbleGain).connect(this.master);
    rumble.start();
  }

  toggleMute(): boolean {
    this.muted = !this.muted;
    try {
      localStorage.setItem("bikepark.muted", this.muted ? "1" : "0");
    } catch {
      // Not persisted; fine.
    }
    if (this.master && this.ctx) this.master.gain.setTargetAtTime(this.muted ? 0 : 0.8, this.ctx.currentTime, 0.05);
    return this.muted;
  }

  /** Called every frame with the local rider's state. */
  update(speed: number, throttle: number, grounded: boolean, fatbike: boolean, crashed: boolean): void {
    const ctx = this.ctx;
    if (!ctx || !this.whine || !this.whine2 || !this.whineGain || !this.rumbleGain || !this.rumbleFilter) return;
    const t = ctx.currentTime;
    const base = fatbike ? 70 + speed * 24 : 110 + speed * 42;
    this.whine.frequency.setTargetAtTime(base, t, 0.05);
    this.whine2.frequency.setTargetAtTime(base * 1.51, t, 0.05);
    const level = crashed ? 0 : (fatbike ? 0.02 : 0.018) + throttle * (fatbike ? 0.035 : 0.05) + Math.min(speed, 15) * 0.001;
    this.whineGain.gain.setTargetAtTime(level, t, 0.06);
    const rumble = grounded && !crashed ? Math.min(0.5, speed * 0.035) : 0;
    this.rumbleGain.gain.setTargetAtTime(rumble, t, 0.08);
    this.rumbleFilter.frequency.setTargetAtTime(160 + speed * 25, t, 0.1);
  }

  private tone(freq: number, start: number, duration: number, type: OscillatorType, gain: number, slideTo?: number): void {
    const ctx = this.ctx;
    if (!ctx || !this.master) return;
    const t = ctx.currentTime + start;
    const osc = ctx.createOscillator();
    const g = ctx.createGain();
    osc.type = type;
    osc.frequency.setValueAtTime(freq, t);
    if (slideTo) osc.frequency.exponentialRampToValueAtTime(slideTo, t + duration);
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(gain, t + 0.01);
    g.gain.exponentialRampToValueAtTime(0.0001, t + duration);
    osc.connect(g).connect(this.master);
    osc.start(t);
    osc.stop(t + duration + 0.02);
  }

  private thud(gain: number, cutoff: number, duration: number): void {
    const ctx = this.ctx;
    if (!ctx || !this.master || !this.noise) return;
    const t = ctx.currentTime;
    const src = ctx.createBufferSource();
    src.buffer = this.noise;
    const f = ctx.createBiquadFilter();
    f.type = "lowpass";
    f.frequency.setValueAtTime(cutoff, t);
    f.frequency.exponentialRampToValueAtTime(60, t + duration);
    const g = ctx.createGain();
    g.gain.setValueAtTime(gain, t);
    g.gain.exponentialRampToValueAtTime(0.0001, t + duration);
    src.connect(f).connect(g).connect(this.master);
    src.start(t, Math.random());
    src.stop(t + duration);
  }

  land(airTime: number): void {
    this.thud(Math.min(1.2, 0.3 + airTime * 0.6), 900, 0.25);
  }

  crash(): void {
    this.thud(1.4, 2400, 0.5);
    this.tone(160, 0, 0.35, "sine", 0.5, 45);
  }

  trick(): void {
    this.tone(660, 0, 0.12, "triangle", 0.25);
    this.tone(990, 0.08, 0.2, "triangle", 0.25);
  }

  combo(multiplier: number): void {
    const notes = [523, 659, 784, 1047];
    for (let i = 0; i < Math.min(4, multiplier + 1); i++) this.tone(notes[i], i * 0.07, 0.25, "square", 0.08);
  }

  checkpoint(): void {
    this.tone(880, 0, 0.1, "sine", 0.2);
  }

  finish(): void {
    [523, 659, 784, 1047, 784, 1047].forEach((f, i) => this.tone(f, i * 0.09, 0.3, "square", 0.08));
  }
}
