//! A bot rides the park; prints its events. Shows how far a simple rider gets.
//!
//! cargo run -p bike-sim --release --example ride -- [e_dirt|fatbike] [seconds]

use bike_sim::{BikeClass, Sim, Track, WorldTuning, DT};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let class = args
        .get(1)
        .and_then(|c| BikeClass::from_id(c))
        .unwrap_or(BikeClass::EDirt);
    let seconds: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(90.0);
    let mut sim = Sim::new(Track::proving_grounds(), WorldTuning::default());
    let id = sim.add_bot(class, 0);
    let track = sim.track().clone();
    let mut crashes = 0;
    for tick in 0..(seconds / DT) as u32 {
        sim.step();
        for e in sim.drain_events() {
            let json = serde_json::to_string(&e.kind).unwrap();
            if json.contains("crash") {
                crashes += 1;
            }
            let hud = sim.hud(id).unwrap();
            let x = hud.progress * (track.finish_x - track.start_x()) + track.start_x();
            let zone = track
                .zones
                .iter()
                .find(|z| x >= z.x0 && x <= z.x1)
                .map_or("", |z| z.label.as_str());
            println!(
                "t={:6.2} x={:6.1} {:12} {}",
                tick as f32 * DT,
                x,
                zone,
                json
            );
        }
    }
    let hud = sim.hud(id).unwrap();
    println!(
        "crashes={crashes} total={} best_run={:?}",
        hud.total, hud.best_run
    );
}
