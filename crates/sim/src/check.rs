//! A fixed scenario for checking that every platform simulates identically.
//!
//! The native test suite and the WebAssembly build both run [`determinism_scenario`];
//! `scripts/check-determinism.mjs` compares the two hashes. If they ever differ, some
//! code path is using platform-dependent math.

use crate::input::Input;
use crate::track::Track;
use crate::tuning::{BikeClass, WorldTuning};
use crate::world::Sim;

/// Pseudo-random but fixed inputs: throttle most of the time, leans both ways, brakes,
/// trick presses and the odd respawn. Changes every 40 ticks.
pub fn scripted_input(tick: u32) -> Input {
    let segment = tick / 40;
    let mut h = segment.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    let throttle = if h & 0b11 == 0 { 0.0 } else { 1.0 };
    let brake = if (h >> 2) & 0b111 == 0 { 1.0 } else { 0.0 };
    let lean = match (h >> 5) & 0b11 {
        0 => -1.0,
        1 => 1.0,
        2 => -0.4,
        _ => 0.0,
    };
    let trick = (h >> 7) & 0b111 == 0 && tick % 40 < 20;
    let respawn = (h >> 10) & 0x3f == 0 && tick.is_multiple_of(40);
    Input::new(throttle, brake, lean, trick, respawn)
}

/// Two scripted riders and two bots for `ticks` ticks; returns the final state hash.
pub fn determinism_scenario(ticks: u32) -> u64 {
    let mut sim = Sim::new(Track::proving_grounds(), WorldTuning::default());
    let a = sim.add_rider(BikeClass::EDirt.default_tuning(), 0);
    let b = sim.add_rider(BikeClass::Fatbike.default_tuning(), 3);
    sim.add_bot(BikeClass::EDirt, 1);
    sim.add_bot(BikeClass::Fatbike, 6);
    for t in 0..ticks {
        sim.set_input(a, scripted_input(t));
        sim.set_input(b, scripted_input(t.wrapping_mul(7).wrapping_add(13)));
        sim.step();
        sim.drain_events();
    }
    sim.state_hash()
}
