//! How expensive is one full room on the server? Simulates 12 bots for a few minutes of
//! game time and reports the cost per tick, i.e. how many rooms one CPU core can host.
//!
//! cargo run -p bike-sim --release --example room_bench -- [riders] [seconds]

use std::time::Instant;

use bike_sim::{BikeClass, Sim, Track, WorldTuning, TICK_RATE};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let riders: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(12);
    let seconds: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(180);

    let mut sim = Sim::new(Track::proving_grounds(), WorldTuning::default());
    let checkpoints = sim.track().checkpoints.len();
    for i in 0..riders {
        let class = if i % 2 == 0 {
            BikeClass::EDirt
        } else {
            BikeClass::Fatbike
        };
        sim.add_bot(class, i % checkpoints);
    }

    let ticks = seconds * TICK_RATE;
    let mut crashes = 0;
    let mut worst = 0.0f64;
    let start = Instant::now();
    for _ in 0..ticks {
        let t0 = Instant::now();
        sim.step();
        worst = worst.max(t0.elapsed().as_secs_f64());
        crashes += sim
            .drain_events()
            .iter()
            .filter(|e| matches!(e.kind, bike_sim::EventKind::Crash { .. }))
            .count();
    }
    let per_tick = start.elapsed().as_secs_f64() / ticks as f64;
    let core_share = per_tick * TICK_RATE as f64;
    println!("{riders} riders, {seconds} s of game time ({ticks} ticks), {crashes} crashes");
    println!(
        "{:.1} µs per tick on average, {:.1} µs worst",
        per_tick * 1e6,
        worst * 1e6
    );
    println!(
        "one room uses {:.2}% of a core at {} ticks/s → ~{:.0} rooms per core (physics only)",
        core_share * 100.0,
        TICK_RATE,
        1.0 / core_share
    );
}
