//! The simulation: one track, any number of riders, advanced in fixed ticks.
//!
//! The same code runs in the browser (WebAssembly) and on the server. Given the same
//! starting state and the same inputs per tick, both produce bit-identical results;
//! [`Sim::state_hash`] makes that checkable.

use rapier2d::prelude::*;
use serde::Serialize;

use crate::bot::{Bot, BotView};
use crate::events::{Event, EventKind};
use crate::input::Input;
use crate::math::{angle_of, atan2, sin, wrap_angle};
use crate::physics::Physics;
use crate::rider::{riding_skeleton, Ragdoll, Skeleton, SKELETON_POINTS};
use crate::rig::{Controls, Rig};
use crate::scoring::{CrashReason, ManualKind, Observation, Score, TrickKind};
use crate::track::Track;
use crate::tuning::{BikeClass, BikeTuning, RiderTuning, WorldTuning};

/// Simulation ticks per second. Physics, input and scoring all run at this rate; the
/// server may send snapshots less often.
pub const TICK_RATE: u32 = 60;
pub const DT: f32 = 1.0 / TICK_RATE as f32;

/// Seconds after finishing before the rider is sent back to the start.
const FINISH_RESTART_DELAY: f32 = 3.0;
/// Contact slack for "this wheel is on the ground" (m).
const GROUND_SLACK: f32 = 0.04;
/// Seconds of throttle without getting anywhere before an automatic respawn.
const STUCK_RESPAWN_DELAY: f32 = 4.0;

pub type RiderId = u32;

struct CrashState {
    ticks: u32,
}

pub struct Rider {
    id: RiderId,
    tuning: BikeTuning,
    input: Input,
    prev_input: Input,
    rig: Rig,
    ragdoll: Option<Ragdoll>,
    crash: Option<CrashState>,
    score: Score,
    checkpoint: usize,
    run_ticks: Option<u32>,
    finished_ticks: Option<u32>,
    best_run_ticks: Option<u32>,
    rear_ground: bool,
    front_ground: bool,
    flipped_ticks: u32,
    stuck_ticks: u32,
    skeleton: Skeleton,
    bot: Option<Bot>,
    controls: Controls,
}

/// Everything the HUD shows for one rider.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HudState {
    pub speed: f32,
    pub total: u32,
    pub combo_points: u32,
    pub combo_tricks: u32,
    pub multiplier: u32,
    /// 1 = combo just extended, 0 = about to bank.
    pub combo_timer: f32,
    pub in_air: bool,
    pub air_time: f32,
    pub air_rotation: f32,
    pub trick: Option<TrickKind>,
    pub trick_extent: f32,
    pub trick_done: bool,
    pub manual: Option<ManualKind>,
    pub manual_distance: f32,
    /// Wheelie/stoppie meter: -1 falling back .. 0 tipping point .. 1 falling forward.
    pub balance: f32,
    pub best_wheelie: f32,
    pub run_time: Option<f32>,
    pub best_run: Option<f32>,
    pub checkpoint: usize,
    pub checkpoints: usize,
    pub crashed: bool,
    pub finished: bool,
    /// 0 at the start, 1 at the finish.
    pub progress: f32,
}

/// Floats per rider in [`Sim::render_state`]. Keep in sync with `web/src/sim/layout.ts`.
pub const RENDER_STRIDE: usize = 41;

pub mod render_layout {
    pub const ID: usize = 0;
    /// 0 riding, 1 crashed, 2 finished.
    pub const STATUS: usize = 1;
    /// 0 e-dirt, 1 fatbike.
    pub const CLASS: usize = 2;
    pub const CHASSIS: usize = 3;
    pub const REAR: usize = 6;
    pub const FRONT: usize = 9;
    /// 11 (x, y) points, see `rider::HEAD` etc.
    pub const SKELETON: usize = 12;
    pub const TRICK: usize = 34;
    pub const TRICK_EXTENT: usize = 35;
    pub const SPEED: usize = 36;
    pub const REAR_COMPRESSION: usize = 37;
    pub const FRONT_COMPRESSION: usize = 38;
    pub const THROTTLE: usize = 39;
    /// Bit 0 rear wheel down, 1 front wheel down, 2 in the air, 3 wheelie/stoppie.
    pub const FLAGS: usize = 40;
}

pub struct Sim {
    phys: Physics,
    track: Track,
    world: WorldTuning,
    rider_tuning: RiderTuning,
    riders: Vec<Rider>,
    next_id: RiderId,
    tick: u32,
    events: Vec<Event>,
}

impl Sim {
    pub fn new(track: Track, world: WorldTuning) -> Sim {
        let mut phys = Physics::new(DT, world.gravity, world.solver_iterations);
        phys.add_terrain(&track);
        Sim {
            phys,
            track,
            world,
            rider_tuning: RiderTuning::default(),
            riders: Vec::new(),
            next_id: 0,
            tick: 0,
            events: Vec::new(),
        }
    }

    pub fn track(&self) -> &Track {
        &self.track
    }

    pub fn tick(&self) -> u32 {
        self.tick
    }

    pub fn world_tuning(&self) -> &WorldTuning {
        &self.world
    }

    /// Adds a rider at the given checkpoint.
    pub fn add_rider(&mut self, tuning: BikeTuning, checkpoint: usize) -> RiderId {
        let id = self.next_id;
        self.next_id += 1;
        let checkpoint = checkpoint.min(self.track.checkpoints.len().saturating_sub(1));
        let (origin, angle) = spawn_pose(&self.track, &tuning, checkpoint);
        let rig = Rig::build(
            &mut self.phys,
            id,
            &tuning,
            &self.rider_tuning,
            origin,
            angle,
        );
        let mut score = Score::default();
        score.set_combo_window(self.world.combo_window);
        let mut rider = Rider {
            id,
            tuning,
            input: Input::default(),
            prev_input: Input::default(),
            rig,
            ragdoll: None,
            crash: None,
            score,
            checkpoint,
            run_ticks: None,
            finished_ticks: None,
            best_run_ticks: None,
            rear_ground: false,
            front_ground: false,
            flipped_ticks: 0,
            stuck_ticks: 0,
            skeleton: Skeleton::default(),
            bot: None,
            controls: Controls::default(),
        };
        rider.update_skeleton(&self.phys, &self.rider_tuning);
        self.riders.push(rider);
        id
    }

    /// Adds a bot rider. Bots pick their own inputs every tick.
    pub fn add_bot(&mut self, class: BikeClass, checkpoint: usize) -> RiderId {
        let id = self.add_rider(class.default_tuning(), checkpoint);
        let seed = id.wrapping_mul(7919).wrapping_add(17);
        self.rider_mut(id).unwrap().bot = Some(Bot::new(seed));
        id
    }

    pub fn remove_rider(&mut self, id: RiderId) {
        if let Some(i) = self.riders.iter().position(|r| r.id == id) {
            let r = self.riders.remove(i);
            r.rig.destroy(&mut self.phys);
            if let Some(rd) = r.ragdoll {
                rd.destroy(&mut self.phys);
            }
        }
    }

    pub fn riders(&self) -> impl Iterator<Item = &Rider> {
        self.riders.iter()
    }

    pub fn rider(&self, id: RiderId) -> Option<&Rider> {
        self.riders.iter().find(|r| r.id == id)
    }

    fn rider_mut(&mut self, id: RiderId) -> Option<&mut Rider> {
        self.riders.iter_mut().find(|r| r.id == id)
    }

    pub fn set_input(&mut self, id: RiderId, input: Input) {
        if let Some(r) = self.rider_mut(id) {
            r.input = input;
        }
    }

    /// Swaps a rider's bike tuning (e.g. a different class, or the tuning panel).
    /// The rider respawns at their current checkpoint.
    pub fn set_tuning(&mut self, id: RiderId, tuning: BikeTuning) {
        let Some(i) = self.riders.iter().position(|r| r.id == id) else {
            return;
        };
        self.riders[i].tuning = tuning;
        self.respawn(i);
    }

    pub fn set_rider_tuning(&mut self, tuning: RiderTuning) {
        self.rider_tuning = tuning;
        for i in 0..self.riders.len() {
            self.respawn(i);
        }
    }

    pub fn rider_tuning(&self) -> &RiderTuning {
        &self.rider_tuning
    }

    /// Changes gravity and timing; riders keep their state.
    pub fn set_world_tuning(&mut self, world: WorldTuning) {
        self.phys.gravity = Vector::new(0.0, -world.gravity);
        self.phys.params.num_solver_iterations = world.solver_iterations.max(1);
        for r in &mut self.riders {
            r.score.set_combo_window(world.combo_window);
        }
        self.world = world;
        // Spring preloads depend on gravity.
        for i in 0..self.riders.len() {
            self.respawn(i);
        }
    }

    /// Takes the events produced since the last call.
    pub fn drain_events(&mut self) -> Vec<Event> {
        core::mem::take(&mut self.events)
    }

    /// Advances the simulation by one tick ([`DT`] seconds).
    pub fn step(&mut self) {
        // Bots choose their inputs.
        for r in &mut self.riders {
            let view = r.bot.is_some().then(|| r.bot_view(&self.phys, &self.track));
            if let (Some(view), Some(bot)) = (view, &mut r.bot) {
                r.input = bot.think(&view);
            }
        }

        // Respawn requests.
        for i in 0..self.riders.len() {
            let r = &self.riders[i];
            if r.input.respawn() && !r.prev_input.respawn() {
                self.respawn(i);
            }
        }

        // Controls.
        for r in &mut self.riders {
            if r.crash.is_none() {
                r.controls = Controls {
                    throttle: r.input.throttle(),
                    brake: r.input.brake(),
                    lean: r.input.lean(),
                };
                r.rig.apply_controls(
                    &mut self.phys,
                    &r.tuning,
                    r.controls,
                    r.rear_ground,
                    r.front_ground,
                );
            } else {
                r.controls = Controls::default();
                r.rig.cut_power(&mut self.phys, &r.tuning);
            }
        }

        self.phys.step();
        self.tick += 1;

        for i in 0..self.riders.len() {
            self.post_step(i);
        }
    }

    fn post_step(&mut self, i: usize) {
        let tick = self.tick;
        let world = self.world.clone();
        let r = &mut self.riders[i];
        let id = r.id;
        let mut events: Vec<EventKind> = Vec::new();

        r.rear_ground = self.phys.touches_terrain(r.rig.rear_collider, GROUND_SLACK);
        r.front_ground = self
            .phys
            .touches_terrain(r.rig.front_collider, GROUND_SLACK);

        let mut respawn = false;
        if let Some(crash) = &mut r.crash {
            crash.ticks += 1;
            if crash.ticks as f32 * DT >= world.respawn_delay {
                respawn = true;
            }
        } else {
            let chassis = &self.phys.bodies[r.rig.chassis];
            let pos = chassis.translation();
            let speed = chassis.linvel().length();
            let angle = angle_of(chassis.rotation());
            let ground_angle = self.track.ground_angle(pos.x, 2.0);

            // Crash checks.
            let mut crash = None;
            if let Some(body) = &r.rig.rider {
                if self.phys.touches_terrain(body.head, 0.0)
                    || self.phys.touches_terrain(body.torso, 0.0)
                {
                    crash = Some(CrashReason::Head);
                }
            }
            if pos.y < self.track.kill_y {
                crash = Some(CrashReason::OutOfBounds);
            }
            let upside_down = wrap_angle(angle - ground_angle).abs() > 1.9;
            if upside_down && speed < 2.0 {
                r.flipped_ticks += 1;
                if r.flipped_ticks > 45 {
                    crash = Some(CrashReason::Flipped);
                }
            } else {
                r.flipped_ticks = 0;
            }
            // Wedged in a pit or against a wall: put them back without a crash.
            if speed < 0.5 && r.input.throttle > 0 {
                r.stuck_ticks += 1;
                if r.stuck_ticks as f32 * DT >= STUCK_RESPAWN_DELAY {
                    respawn = true;
                }
            } else {
                r.stuck_ticks = 0;
            }

            // Scoring.
            if crash.is_none() {
                let com = r.rig.center_of_mass(&self.phys);
                let balance_of = |wheel: RigidBodyHandle, radius: f32| {
                    let contact_x =
                        self.phys.bodies[wheel].translation().x + radius * sin(ground_angle);
                    com.x - contact_x
                };
                let obs = Observation {
                    dt: DT,
                    rear_ground: r.rear_ground,
                    front_ground: r.front_ground,
                    angle,
                    x: pos.x,
                    ground_angle,
                    rear_balance: balance_of(r.rig.rear, r.tuning.rear_wheel_radius),
                    front_balance: balance_of(r.rig.front, r.tuning.front_wheel_radius),
                    input: r.input,
                    prev_input: r.prev_input,
                };
                crash = r.score.update(&obs, &mut events);
            }

            if let Some(reason) = crash {
                r.crash_now(&mut self.phys, &self.rider_tuning, reason, &mut events);
            } else {
                // Checkpoints, run timer and finish line.
                let cps = &self.track.checkpoints;
                while r.checkpoint + 1 < cps.len() && pos.x >= cps[r.checkpoint + 1] {
                    r.checkpoint += 1;
                    events.push(EventKind::Checkpoint {
                        index: r.checkpoint,
                    });
                }
                if r.run_ticks.is_none()
                    && r.finished_ticks.is_none()
                    && r.checkpoint == 0
                    && r.input.throttle > 0
                {
                    r.run_ticks = Some(0);
                    events.push(EventKind::RunStart);
                }
                if r.finished_ticks.is_none() {
                    if let Some(t) = &mut r.run_ticks {
                        *t += 1;
                        if pos.x >= self.track.finish_x {
                            let t = *t;
                            let best = r.best_run_ticks.is_none_or(|b| t < b);
                            if best {
                                r.best_run_ticks = Some(t);
                            }
                            r.finished_ticks = Some(0);
                            events.push(EventKind::Finish {
                                time: t as f32 * DT,
                                best,
                            });
                        }
                    }
                }
            }
        }
        if let Some(f) = &mut r.finished_ticks {
            *f += 1;
            if *f as f32 * DT >= FINISH_RESTART_DELAY && r.crash.is_none() {
                // Back to the start; a combo still open after the finish line counts.
                r.score.bank(&mut events);
                r.checkpoint = 0;
                respawn = true;
            }
        }
        r.update_skeleton(&self.phys, &self.rider_tuning);
        r.prev_input = r.input;

        self.events.extend(events.into_iter().map(|kind| Event {
            rider: id,
            tick,
            kind,
        }));
        if respawn {
            self.respawn(i);
        }
    }

    fn respawn(&mut self, i: usize) {
        let r = &mut self.riders[i];
        if r.crash.is_none() {
            // A manual respawn mid-combo still loses the combo.
            let mut lost = Vec::new();
            r.score.crash(&mut lost);
            self.events.extend(lost.into_iter().map(|kind| Event {
                rider: r.id,
                tick: self.tick,
                kind,
            }));
        }
        if r.checkpoint == 0 {
            r.run_ticks = None;
            r.finished_ticks = None;
        }
        let (origin, angle) = spawn_pose(&self.track, &r.tuning, r.checkpoint);
        let old = core::mem::replace(
            &mut r.rig,
            Rig::build(
                &mut self.phys,
                r.id,
                &r.tuning,
                &self.rider_tuning,
                origin,
                angle,
            ),
        );
        old.destroy(&mut self.phys);
        if let Some(rd) = r.ragdoll.take() {
            rd.destroy(&mut self.phys);
        }
        r.crash = None;
        r.flipped_ticks = 0;
        r.stuck_ticks = 0;
        r.rear_ground = false;
        r.front_ground = false;
        r.score.reset_run_state();
        r.update_skeleton(&self.phys, &self.rider_tuning);
        self.events.push(Event {
            rider: r.id,
            tick: self.tick,
            kind: EventKind::Respawn {
                checkpoint: r.checkpoint,
            },
        });
    }

    /// Writes [`RENDER_STRIDE`] floats per rider into `out` (cleared first).
    pub fn render_state(&self, out: &mut Vec<f32>) {
        use render_layout::*;
        out.clear();
        for r in &self.riders {
            let base = out.len();
            out.resize(base + RENDER_STRIDE, 0.0);
            let o = &mut out[base..];
            let b = |h: RigidBodyHandle| {
                let body = &self.phys.bodies[h];
                let p = body.translation();
                [p.x, p.y, angle_of(body.rotation())]
            };
            o[ID] = r.id as f32;
            o[STATUS] = if r.crash.is_some() {
                1.0
            } else if r.finished_ticks.is_some() {
                2.0
            } else {
                0.0
            };
            o[CLASS] = match r.tuning.class {
                BikeClass::EDirt => 0.0,
                BikeClass::Fatbike => 1.0,
            };
            o[CHASSIS..CHASSIS + 3].copy_from_slice(&b(r.rig.chassis));
            o[REAR..REAR + 3].copy_from_slice(&b(r.rig.rear));
            o[FRONT..FRONT + 3].copy_from_slice(&b(r.rig.front));
            for (k, p) in r.skeleton.points.iter().enumerate() {
                o[SKELETON + 2 * k] = p.x;
                o[SKELETON + 2 * k + 1] = p.y;
            }
            if let Some(t) = &r.score.trick {
                o[TRICK] = t.kind as u8 as f32;
                o[TRICK_EXTENT] = t.extent;
            }
            o[SPEED] = self.phys.bodies[r.rig.chassis].linvel().length();
            let (rc, fc) = r.rig.compression(&self.phys);
            o[REAR_COMPRESSION] = rc;
            o[FRONT_COMPRESSION] = fc;
            o[THROTTLE] = r.controls.throttle;
            o[FLAGS] = (r.rear_ground as u32
                | (r.front_ground as u32) << 1
                | (r.score.in_air as u32) << 2
                | (r.score.manual.is_some() as u32) << 3) as f32;
        }
    }

    pub fn hud(&self, id: RiderId) -> Option<HudState> {
        let r = self.rider(id)?;
        let s = &r.score;
        let chassis = &self.phys.bodies[r.rig.chassis];
        let x = chassis.translation().x;
        let pivot = match s.manual.map(|m| m.kind) {
            Some(ManualKind::Stoppie) => Some(r.rig.front),
            Some(ManualKind::Wheelie) => Some(r.rig.rear),
            None if r.rear_ground && !r.front_ground => Some(r.rig.rear),
            None => None,
        };
        let balance = pivot.map_or(0.0, |wheel| {
            let com = r.rig.center_of_mass(&self.phys);
            Score::balance_meter(com.x - self.phys.bodies[wheel].translation().x)
        });
        let (start, finish) = (self.track.start_x(), self.track.finish_x);
        Some(HudState {
            speed: chassis.linvel().length(),
            total: s.total,
            combo_points: s.combo_points,
            combo_tricks: s.combo_tricks,
            multiplier: s.multiplier(),
            combo_timer: if self.world.combo_window > 0.0 {
                (s.combo_timer / self.world.combo_window).clamp(0.0, 1.0)
            } else {
                0.0
            },
            in_air: s.in_air,
            air_time: s.air_time,
            air_rotation: s.air_rotation,
            trick: s.trick.map(|t| t.kind),
            trick_extent: s.trick.map_or(0.0, |t| t.extent),
            trick_done: s.trick.is_some_and(|t| t.done),
            manual: s.manual.map(|m| m.kind),
            manual_distance: s.manual.map_or(0.0, |m| m.distance),
            balance,
            best_wheelie: s.best_wheelie,
            run_time: r.run_ticks.map(|t| t as f32 * DT),
            best_run: r.best_run_ticks.map(|t| t as f32 * DT),
            checkpoint: r.checkpoint,
            checkpoints: self.track.checkpoints.len(),
            crashed: r.crash.is_some(),
            finished: r.finished_ticks.is_some(),
            progress: ((x - start) / (finish - start).max(1.0)).clamp(0.0, 1.0),
        })
    }

    /// A hash of every body's exact position and velocity, plus scores. Two sims that
    /// were given the same inputs must report the same hash on every platform.
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv::new();
        h.u32(self.tick);
        for r in &self.riders {
            h.u32(r.id);
            h.u32(r.score.total);
            h.u32(r.score.combo_points);
            h.u32(r.checkpoint as u32);
            let bodies: Vec<RigidBodyHandle> = r
                .rig
                .bodies()
                .chain(r.ragdoll.iter().flat_map(|rd| rd.bodies()))
                .collect();
            for handle in bodies {
                let b = &self.phys.bodies[handle];
                let p = b.position();
                for v in [
                    p.translation.x,
                    p.translation.y,
                    p.rotation.re,
                    p.rotation.im,
                    b.linvel().x,
                    b.linvel().y,
                    b.angvel(),
                ] {
                    h.u32(v.to_bits());
                }
            }
        }
        h.finish()
    }
}

impl Rider {
    pub fn id(&self) -> RiderId {
        self.id
    }

    pub fn tuning(&self) -> &BikeTuning {
        &self.tuning
    }

    pub fn score(&self) -> &Score {
        &self.score
    }

    pub fn is_crashed(&self) -> bool {
        self.crash.is_some()
    }

    pub fn is_bot(&self) -> bool {
        self.bot.is_some()
    }

    pub fn skeleton(&self) -> &Skeleton {
        &self.skeleton
    }

    pub fn checkpoint(&self) -> usize {
        self.checkpoint
    }

    pub fn wheels_down(&self) -> (bool, bool) {
        (self.rear_ground, self.front_ground)
    }

    fn crash_now(
        &mut self,
        phys: &mut Physics,
        rt: &RiderTuning,
        reason: CrashReason,
        events: &mut Vec<EventKind>,
    ) {
        let speed = phys.bodies[self.rig.chassis].linvel().length();
        self.score.crash(events);
        self.update_skeleton(phys, rt);
        if let Some(body) = self.rig.rider.take() {
            let torso_pose = *phys.bodies[body.body].position();
            self.ragdoll = Some(Ragdoll::spawn(
                phys,
                self.id,
                rt,
                &self.skeleton,
                torso_pose,
                body.body,
            ));
            phys.remove_body(body.body);
        }
        self.rig.cut_power(phys, &self.tuning);
        self.crash = Some(CrashState { ticks: 0 });
        events.push(EventKind::Crash { reason, speed });
    }

    fn update_skeleton(&mut self, phys: &Physics, rt: &RiderTuning) {
        if let Some(rd) = &self.ragdoll {
            self.skeleton = rd.skeleton(phys);
            return;
        }
        let body = self.rig.rider.as_ref().map(|b| b.body);
        if let Some(body) = body {
            let chassis = phys.bodies[self.rig.chassis].position();
            let rider = phys.bodies[body].position();
            let trick = self.score.trick.map(|t| {
                let e = if t.kind == TrickKind::TailWhip {
                    0.0
                } else {
                    t.extent
                };
                (t.kind, e)
            });
            self.skeleton = riding_skeleton(chassis, rider, &self.tuning, rt, trick);
        }
    }

    fn bot_view(&self, phys: &Physics, track: &Track) -> BotView {
        let chassis = &phys.bodies[self.rig.chassis];
        let pos = chassis.translation();
        let vel = chassis.linvel();
        let gravity = phys.gravity.y;
        // Bots know the track: predict where a ballistic arc meets the ground.
        let mut landing_x = pos.x + 1.0;
        let mut t = 0.0;
        while t < 3.0 {
            t += 0.05;
            let x = pos.x + vel.x * t;
            let y = pos.y + vel.y * t + 0.5 * gravity * t * t;
            if y < track.ground_y(x) + 0.4 {
                landing_x = x;
                break;
            }
        }
        BotView {
            crashed: self.crash.is_some(),
            in_air: self.score.in_air,
            angle: angle_of(chassis.rotation()),
            angvel: chassis.angvel(),
            speed: vel.length(),
            travel_angle: atan2(vel.y, vel.x),
            ground_angle: track.ground_angle(pos.x, 2.0),
            landing_angle: track.ground_angle(landing_x, 1.5),
        }
    }
}

/// Where a rider appears at a checkpoint: on the ground, level with it.
fn spawn_pose(track: &Track, t: &BikeTuning, checkpoint: usize) -> (Vector, f32) {
    let x = track.checkpoints.get(checkpoint).copied().unwrap_or(0.0) + 2.0;
    let xr = x + t.rear_axle[0];
    let xf = x + t.front_axle[0];
    let yr = track.ground_y(xr) + t.rear_wheel_radius - t.rear_axle[1];
    let yf = track.ground_y(xf) + t.front_wheel_radius - t.front_axle[1];
    let angle = atan2(yf - yr, xf - xr);
    (Vector::new((xr + xf) * 0.5, (yr + yf) * 0.5 + 0.02), angle)
}

/// FNV-1a, 64 bit. Tiny, stable across platforms and Rust versions.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    fn u32(&mut self, v: u32) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

// Keep the skeleton layout and the render layout in lockstep.
const _: () = assert!(render_layout::SKELETON + 2 * SKELETON_POINTS == render_layout::TRICK);
