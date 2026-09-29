//! Tick-by-tick trace of a bot through a section of track, for debugging the feel.
//!
//! cargo run -p bike-sim --release --example trace -- <checkpoint> <from_x> <to_x> [class]

use bike_sim::world::render_layout as L;
use bike_sim::{BikeClass, Sim, Track, WorldTuning, DT};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cp: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let from: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let to: f32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1e9);
    let class = args
        .get(4)
        .and_then(|c| BikeClass::from_id(c))
        .unwrap_or(BikeClass::EDirt);
    let mut sim = Sim::new(Track::proving_grounds(), WorldTuning::default());
    sim.add_bot(class, cp);
    let mut buf = Vec::new();
    for tick in 0..(40.0 / DT) as u32 {
        sim.step();
        sim.render_state(&mut buf);
        let o = &buf;
        let x = o[L::CHASSIS];
        let events: Vec<String> = sim
            .drain_events()
            .iter()
            .map(|e| serde_json::to_string(&e.kind).unwrap())
            .collect();
        let in_range = x >= from && x <= to;
        if in_range && (tick % 3 == 0 || !events.is_empty()) {
            let g = sim.track().ground_y(x);
            let ga = sim.track().ground_angle(x, 2.0);
            println!(
                "t={:6.2} x={:6.2} h={:5.2} ang={:6.1} slope={:5.1} v={:5.2} thr={:.1} flags={} {}",
                tick as f32 * DT,
                x,
                o[L::CHASSIS + 1] - g,
                o[L::CHASSIS + 2].to_degrees(),
                ga.to_degrees(),
                o[L::SPEED],
                o[L::THROTTLE],
                o[L::FLAGS],
                events.join(" ")
            );
        }
        if x > to {
            break;
        }
    }
}
