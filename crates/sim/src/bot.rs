//! Simple bots so no one ever rides alone. They run inside the simulation, so the
//! server can fill quiet rooms with them, and they are deterministic (seeded RNG).

use crate::input::Input;
use crate::math::wrap_angle;

/// What a bot can see of its own bike.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BotView {
    pub crashed: bool,
    pub in_air: bool,
    pub angle: f32,
    pub angvel: f32,
    pub speed: f32,
    /// Direction of travel (rad). On the ground this follows the ground under the rear
    /// wheel, which is more reliable than sampling the track at ramp lips.
    pub travel_angle: f32,
    /// Slope under the bike.
    pub ground_angle: f32,
    /// Slope where the bike will land if it is in the air.
    pub landing_angle: f32,
}

#[derive(Clone, Debug)]
pub struct Bot {
    rng: u32,
    /// Ticks left in the current wheelie attempt.
    wheelie: u32,
    /// Ticks left holding the trick button.
    trick: u32,
    /// 0..1: how often this bot shows off.
    style: f32,
    /// Preferred cruising speed (m/s).
    cruise: f32,
}

impl Bot {
    pub fn new(seed: u32) -> Self {
        let mut bot = Self {
            rng: seed.wrapping_mul(0x9E37_79B9) | 1,
            wheelie: 0,
            trick: 0,
            style: 0.0,
            cruise: 0.0,
        };
        bot.style = bot.next_f32();
        bot.cruise = 9.0 + 6.0 * bot.next_f32();
        bot
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub(crate) fn think(&mut self, v: &BotView) -> Input {
        if v.crashed {
            self.wheelie = 0;
            self.trick = 0;
            return Input::default();
        }
        if v.in_air {
            // Level out to match the landing slope; now and then throw a trick.
            let err = wrap_angle(v.angle - v.landing_angle);
            let lean = (1.6 * err + 0.35 * v.angvel).clamp(-1.0, 1.0);
            if self.trick == 0 && self.next_f32() < 0.004 * self.style {
                self.trick = 14;
            }
            let trick = self.trick > 0;
            self.trick = self.trick.saturating_sub(1);
            return Input::new(0.0, 0.0, lean, trick, false);
        }
        self.trick = 0;

        let pitch = if v.speed > 1.0 {
            wrap_angle(v.angle - v.travel_angle)
        } else {
            wrap_angle(v.angle - v.ground_angle)
        };
        let flat = v.ground_angle.abs() < 0.05 && v.landing_angle.abs() < 0.05;
        if self.wheelie == 0 && flat && v.speed > 4.0 && self.next_f32() < 0.01 * self.style {
            self.wheelie = 90 + (self.next_u32() % 150);
        }
        if self.wheelie > 0 {
            self.wheelie -= 1;
            // Hold the nose near the balance point with throttle and lean.
            let target = 0.55;
            let throttle = if pitch < target { 1.0 } else { 0.0 };
            let brake = if pitch > target + 0.25 { 1.0 } else { 0.0 };
            let lean = if pitch < target - 0.2 { -1.0 } else { 0.0 };
            if !flat {
                self.wheelie = 0;
            }
            return Input::new(throttle, brake, lean, false, false);
        }
        let mut throttle: f32 = if v.speed < self.cruise { 1.0 } else { 0.3 };
        // Keep the front down on climbs, over crests and after landings.
        let lean = if pitch > 0.18 {
            throttle = 0.0;
            1.0
        } else if pitch > 0.08 {
            throttle = throttle.min(0.4);
            0.6
        } else if v.ground_angle > 0.2 {
            0.5
        } else {
            0.0
        };
        Input::new(throttle, 0.0, lean, false, false)
    }
}
