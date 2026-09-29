//! Wheelies, tricks and combos.
//!
//! This is a pure state machine fed one [`Observation`] per tick, so it can be tested
//! without physics. Rules:
//! - A wheelie (rear wheel only) or stoppie (front wheel only) scores by distance, with
//!   a style bonus for holding it near the tipping point.
//! - Flips are counted from the bike's rotation between take-off and landing.
//! - Named tricks start with the trick button in the air (holding it through take-off
//!   works too); the held direction picks the trick. Landing before the rider is back
//!   on the bike is a bail (crash).
//! - Every landed trick joins the open combo; the combo multiplier is the number of
//!   tricks in it. A combo is banked once the rider has been on the ground, not
//!   balancing a wheelie, for `combo_window` seconds. A crash loses the unbanked combo.

use serde::{Deserialize, Serialize};

use crate::events::EventKind;
use crate::input::Input;
use crate::math::{wrap_angle, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrickKind {
    NoHander = 1,
    Superman = 2,
    CanCan = 3,
    TailWhip = 4,
}

impl TrickKind {
    pub fn name(self) -> &'static str {
        match self {
            TrickKind::NoHander => "No-Hander",
            TrickKind::Superman => "Superman",
            TrickKind::CanCan => "Can-Can",
            TrickKind::TailWhip => "Tail-Whip",
        }
    }

    pub fn points(self) -> u32 {
        match self {
            TrickKind::NoHander => 200,
            TrickKind::Superman => 300,
            TrickKind::CanCan => 250,
            TrickKind::TailWhip => 400,
        }
    }

    /// Trick + direction: up (throttle) = Superman, down (brake) = Can-Can,
    /// forward/back (lean) = Tail-Whip, no direction = No-Hander.
    fn from_input(input: &Input) -> TrickKind {
        if input.throttle() > 0.5 {
            TrickKind::Superman
        } else if input.brake() > 0.5 {
            TrickKind::CanCan
        } else if input.lean().abs() > 0.5 {
            TrickKind::TailWhip
        } else {
            TrickKind::NoHander
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualKind {
    Wheelie,
    Stoppie,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashReason {
    /// Head or body hit the ground.
    Head,
    /// Landed before getting back on the bike after a trick.
    Bail,
    /// Bike stuck upside down.
    Flipped,
    /// Fell off the world.
    OutOfBounds,
}

/// What the scoring needs to know about the bike this tick.
#[derive(Clone, Copy, Debug, Default)]
pub struct Observation {
    pub dt: f32,
    pub rear_ground: bool,
    pub front_ground: bool,
    pub angle: f32,
    /// Chassis position along the track.
    pub x: f32,
    /// Slope angle of the ground under the bike.
    pub ground_angle: f32,
    /// Horizontal distance of the rig's centre of mass ahead of the rear axle (m).
    /// Zero is the tipping point of a wheelie.
    pub rear_balance: f32,
    /// Same, ahead of the front axle: zero is the tipping point of a stoppie.
    pub front_balance: f32,
    pub input: Input,
    pub prev_input: Input,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveTrick {
    pub kind: TrickKind,
    /// 0 = on the bike, 1 = fully into the trick (for Tail-Whip: spin progress).
    pub extent: f32,
    held: f32,
    pub done: bool,
}

impl ActiveTrick {
    /// Would landing now be a bail?
    fn is_bail(&self) -> bool {
        match self.kind {
            TrickKind::TailWhip => self.extent > 0.12 && self.extent < 0.88,
            _ => self.extent > 0.35,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Manual {
    pub kind: ManualKind,
    pub distance: f32,
    points: f32,
    last_x: f32,
    lost_ticks: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct PendingLanding {
    tricks: Vec<(String, u32)>,
    ticks_left: u32,
}

/// Tunable scoring constants.
const TAKEOFF_TICKS: u32 = 5;
const MANUAL_START_TICKS: u32 = 12;
const MANUAL_END_TICKS: u32 = 6;
const MANUAL_MIN_DISTANCE: f32 = 2.0;
const LANDING_GRACE_TICKS: u32 = 18;
const TRICK_MIN_AIR_TIME: f32 = 0.12;
const TRICK_EXTEND_TIME: f32 = 0.22;
const TRICK_RETRACT_TIME: f32 = 0.18;
const TRICK_HOLD_TIME: f32 = 0.2;
const TAIL_WHIP_TIME: f32 = 0.6;
const FLIP_TOLERANCE: f32 = 0.7;
const MULTIPLIER_CAP: u32 = 10;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Score {
    /// Banked points this session.
    pub total: u32,
    pub combo_points: u32,
    pub combo_tricks: u32,
    /// Seconds until the open combo is banked.
    pub combo_timer: f32,
    pub in_air: bool,
    pub air_time: f32,
    /// Rotation since take-off (rad, counter-clockwise positive = backflip direction).
    pub air_rotation: f32,
    pub trick: Option<ActiveTrick>,
    pub manual: Option<Manual>,
    pub best_wheelie: f32,
    air_ticks: u32,
    last_angle: Option<f32>,
    air_tricks: Vec<(String, u32)>,
    pending: Option<PendingLanding>,
    manual_candidate: Option<(ManualKind, u32)>,
    trick_latch: bool,
    combo_window: f32,
}

impl Score {
    pub fn multiplier(&self) -> u32 {
        self.combo_tricks.clamp(1, MULTIPLIER_CAP)
    }

    /// Advances one tick. Returns a crash reason if the rider bailed a trick.
    pub fn update(&mut self, o: &Observation, events: &mut Vec<EventKind>) -> Option<CrashReason> {
        let grounded = o.rear_ground || o.front_ground;
        let d_angle = self
            .last_angle
            .map_or(0.0, |last| wrap_angle(o.angle - last));
        self.last_angle = Some(o.angle);

        // --- Air ---
        if grounded {
            self.air_ticks = 0;
        } else {
            self.air_ticks += 1;
            self.air_rotation += d_angle;
        }
        if !self.in_air && self.air_ticks >= TAKEOFF_TICKS {
            self.in_air = true;
            self.air_time = self.air_ticks as f32 * o.dt;
            self.air_tricks.clear();
            events.push(EventKind::TakeOff);
        }
        let mut crash = None;
        if self.in_air {
            if grounded {
                crash = self.land(events);
            } else {
                self.air_time += o.dt;
                self.update_trick(o, events);
            }
        } else if grounded {
            self.air_rotation = 0.0;
        }

        // --- Landing confirmation: tricks count once the rider rides away ---
        if let Some(p) = &mut self.pending {
            if p.ticks_left == 0 {
                let p = self.pending.take().unwrap();
                for (name, points) in p.tricks {
                    self.add_trick(name, points, events);
                }
            } else {
                p.ticks_left -= 1;
            }
        }

        // --- Wheelies and stoppies ---
        self.update_manual(o, events);

        // --- Combo banking ---
        let busy = self.in_air || self.manual.is_some() || self.pending.is_some();
        if self.combo_tricks > 0 && !busy {
            self.combo_timer -= o.dt;
            if self.combo_timer <= 0.0 {
                self.bank(events);
            }
        }
        crash
    }

    /// Banks the open combo now: its points times the multiplier join the total.
    pub fn bank(&mut self, events: &mut Vec<EventKind>) {
        if self.combo_tricks == 0 {
            return;
        }
        let multiplier = self.multiplier();
        let points = self.combo_points * multiplier;
        self.total += points;
        events.push(EventKind::ComboBanked {
            points,
            tricks: self.combo_tricks,
            multiplier,
        });
        self.combo_points = 0;
        self.combo_tricks = 0;
    }

    fn update_trick(&mut self, o: &Observation, events: &mut Vec<EventKind>) {
        // Holding the button through take-off starts the trick as soon as it is allowed;
        // it must be released between tricks so one long press doesn't chain them.
        if !o.input.trick() {
            self.trick_latch = false;
        }
        if self.trick.is_none()
            && o.input.trick()
            && !self.trick_latch
            && self.air_time >= TRICK_MIN_AIR_TIME
        {
            self.trick_latch = true;
            let kind = TrickKind::from_input(&o.input);
            self.trick = Some(ActiveTrick {
                kind,
                extent: 0.0,
                held: 0.0,
                done: false,
            });
            events.push(EventKind::TrickStarted { trick: kind });
        }
        let Some(t) = &mut self.trick else { return };
        let finished = if t.kind == TrickKind::TailWhip {
            t.extent = (t.extent + o.dt / TAIL_WHIP_TIME).min(1.0);
            t.done = t.extent >= 1.0;
            t.done
        } else if o.input.trick() {
            t.extent = (t.extent + o.dt / TRICK_EXTEND_TIME).min(1.0);
            if t.extent >= 1.0 {
                t.held += o.dt;
                t.done |= t.held >= TRICK_HOLD_TIME;
            }
            false
        } else {
            t.extent = (t.extent - o.dt / TRICK_RETRACT_TIME).max(0.0);
            t.extent <= 0.0
        };
        if finished {
            let t = self.trick.take().unwrap();
            if t.done {
                self.air_tricks
                    .push((t.kind.name().to_string(), t.kind.points()));
            }
        }
    }

    fn land(&mut self, events: &mut Vec<EventKind>) -> Option<CrashReason> {
        self.in_air = false;
        events.push(EventKind::Landed {
            air_time: self.air_time,
        });
        if let Some(t) = self.trick.take() {
            if t.is_bail() {
                return Some(CrashReason::Bail);
            }
            if t.done {
                self.air_tricks
                    .push((t.kind.name().to_string(), t.kind.points()));
            }
        }
        let mut tricks = core::mem::take(&mut self.air_tricks);
        let flips = ((self.air_rotation.abs() + FLIP_TOLERANCE) / TAU) as u32;
        if flips > 0 {
            let back = self.air_rotation > 0.0;
            let base = if back { 500 } else { 600 };
            let prefix = match flips {
                1 => String::new(),
                2 => "Double ".into(),
                3 => "Triple ".into(),
                n => format!("{n}x "),
            };
            let name = format!("{prefix}{}", if back { "Backflip" } else { "Frontflip" });
            // Each extra rotation is worth more than the last.
            let points = base * flips * (flips + 1) / 2;
            tricks.push((name, points));
        }
        if self.air_time >= 1.2 {
            tricks.push(("Big Air".into(), (self.air_time * 100.0) as u32));
        }
        self.air_rotation = 0.0;
        if !tricks.is_empty() {
            self.pending = Some(PendingLanding {
                tricks,
                ticks_left: LANDING_GRACE_TICKS,
            });
        }
        None
    }

    fn update_manual(&mut self, o: &Observation, events: &mut Vec<EventKind>) {
        let pitch = wrap_angle(o.angle - o.ground_angle);
        let candidate = if self.in_air {
            None
        } else if o.rear_ground && !o.front_ground && pitch > 0.06 {
            Some(ManualKind::Wheelie)
        } else if o.front_ground && !o.rear_ground && pitch < -0.06 {
            Some(ManualKind::Stoppie)
        } else {
            None
        };

        if let Some(m) = &mut self.manual {
            if candidate == Some(m.kind) {
                m.lost_ticks = 0;
                let dx = (o.x - m.last_x).abs();
                m.last_x = o.x;
                // Style: up to double points for riding right at the tipping point.
                let balance = match m.kind {
                    ManualKind::Wheelie => o.rear_balance,
                    ManualKind::Stoppie => o.front_balance,
                };
                let style = 1.0 + (1.0 - balance.abs() / 0.25).clamp(0.0, 1.0);
                let rate = if m.kind == ManualKind::Wheelie {
                    10.0
                } else {
                    15.0
                };
                m.distance += dx;
                m.points += dx * rate * style;
            } else {
                m.lost_ticks += 1;
                if m.lost_ticks >= MANUAL_END_TICKS {
                    self.end_manual(events);
                }
            }
            return;
        }

        match (candidate, self.manual_candidate) {
            (Some(kind), Some((k, n))) if k == kind => {
                if n + 1 >= MANUAL_START_TICKS {
                    self.manual = Some(Manual {
                        kind,
                        distance: 0.0,
                        points: 0.0,
                        last_x: o.x,
                        lost_ticks: 0,
                    });
                    self.manual_candidate = None;
                    events.push(EventKind::ManualStart { kind });
                } else {
                    self.manual_candidate = Some((kind, n + 1));
                }
            }
            (Some(kind), _) => self.manual_candidate = Some((kind, 1)),
            (None, _) => self.manual_candidate = None,
        }
    }

    fn end_manual(&mut self, events: &mut Vec<EventKind>) {
        let Some(m) = self.manual.take() else { return };
        events.push(EventKind::ManualEnd {
            kind: m.kind,
            distance: m.distance,
        });
        if m.distance >= MANUAL_MIN_DISTANCE {
            if m.kind == ManualKind::Wheelie {
                self.best_wheelie = self.best_wheelie.max(m.distance);
            }
            let label = if m.kind == ManualKind::Wheelie {
                "Wheelie"
            } else {
                "Stoppie"
            };
            let name = format!("{label} {:.1} m", m.distance);
            self.add_trick(name, m.points as u32, events);
        }
    }

    fn add_trick(&mut self, name: String, points: u32, events: &mut Vec<EventKind>) {
        self.combo_points += points;
        self.combo_tricks += 1;
        events.push(EventKind::TrickLanded { name, points });
        self.combo_timer = self.combo_window;
    }

    /// The rider crashed: everything not yet banked is lost.
    pub fn crash(&mut self, events: &mut Vec<EventKind>) {
        if self.combo_tricks > 0 || self.pending.is_some() || !self.air_tricks.is_empty() {
            events.push(EventKind::ComboLost {
                points: self.combo_points,
                tricks: self.combo_tricks,
            });
        }
        self.reset_run_state();
    }

    /// Clears in-progress state (used on crash and respawn); banked points are kept.
    pub fn reset_run_state(&mut self) {
        let (total, best, window) = (self.total, self.best_wheelie, self.combo_window);
        *self = Score {
            total,
            best_wheelie: best,
            combo_window: window,
            ..Default::default()
        };
    }

    /// Wheelie meter reading in -1..1: 0 is the tipping point, -1 is about to loop out,
    /// +1 is about to drop the front wheel.
    pub fn balance_meter(balance: f32) -> f32 {
        (balance / 0.4).clamp(-1.0, 1.0)
    }

    pub fn set_combo_window(&mut self, seconds: f32) {
        self.combo_window = seconds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn obs(rear: bool, front: bool, angle: f32, x: f32) -> Observation {
        Observation {
            dt: DT,
            rear_ground: rear,
            front_ground: front,
            angle,
            x,
            ..Default::default()
        }
    }

    fn score() -> Score {
        let mut s = Score::default();
        s.set_combo_window(1.0);
        s
    }

    fn run(s: &mut Score, o: Observation, ticks: usize) -> Vec<EventKind> {
        let mut events = Vec::new();
        for _ in 0..ticks {
            assert_eq!(s.update(&o, &mut events), None);
        }
        events
    }

    #[test]
    fn wheelie_scores_distance_and_banks() {
        let mut s = score();
        let mut events = Vec::new();
        for i in 0..240 {
            let o = obs(true, false, 0.4, i as f32 * 0.1);
            s.update(&o, &mut events);
        }
        assert!(s.manual.is_some());
        let events2 = run(&mut s, obs(true, true, 0.0, 24.0), 120);
        let ended = events2
            .iter()
            .any(|e| matches!(e, EventKind::ManualEnd { distance, .. } if *distance > 20.0));
        assert!(ended, "{events2:?}");
        assert!(s.total > 0, "combo should bank after the window");
        assert!(s.best_wheelie > 20.0);
    }

    #[test]
    fn backflip_is_counted_on_landing() {
        let mut s = score();
        run(&mut s, obs(true, true, 0.0, 0.0), 5);
        let mut events = Vec::new();
        // One full counter-clockwise turn in 60 ticks.
        for i in 0..=60 {
            let angle = wrap_angle(TAU * i as f32 / 60.0);
            s.update(&obs(false, false, angle, 0.0), &mut events);
        }
        run(&mut s, obs(true, true, 0.0, 0.0), 30);
        assert_eq!(s.combo_tricks, 1);
        assert_eq!(s.combo_points, 500);
    }

    #[test]
    fn landing_mid_trick_is_a_bail() {
        let mut s = score();
        let mut events = Vec::new();
        let mut o = obs(false, false, 0.0, 0.0);
        for _ in 0..20 {
            s.update(&o, &mut events);
        }
        o.prev_input = o.input;
        o.input = Input::new(0.0, 0.0, 0.0, true, false);
        for _ in 0..10 {
            s.update(&o, &mut events);
            o.prev_input = o.input;
        }
        assert!(events.iter().any(|e| matches!(
            e,
            EventKind::TrickStarted {
                trick: TrickKind::NoHander
            }
        )));
        let crash = s.update(&obs(true, true, 0.0, 0.0), &mut events);
        assert_eq!(crash, Some(CrashReason::Bail));
    }

    #[test]
    fn crash_loses_the_combo() {
        let mut s = score();
        let mut events = Vec::new();
        for i in 0..200 {
            s.update(&obs(true, false, 0.4, i as f32 * 0.1), &mut events);
        }
        run(&mut s, obs(true, true, 0.0, 20.0), 10);
        assert_eq!(s.combo_tricks, 1);
        s.crash(&mut events);
        assert_eq!(s.combo_tricks, 0);
        assert_eq!(s.total, 0);
        assert!(events
            .iter()
            .any(|e| matches!(e, EventKind::ComboLost { .. })));
    }
}
