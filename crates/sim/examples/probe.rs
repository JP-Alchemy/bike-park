//! Prints what a bike does under scripted inputs. Handy while tuning the feel.
//!
//! cargo run -p bike-sim --release --example probe -- \
//!     [settle|throttle|wheelie|backflip|brake] [e_dirt|fatbike] [checkpoint]
//!
//! Try `backflip e_dirt 7` (the Big Air section) or `wheelie e_dirt 2` (the wheelie strip).

use bike_sim::world::render_layout as L;
use bike_sim::{BikeClass, Input, Sim, Track, WorldTuning, DT, RENDER_STRIDE};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scenario = args.get(1).map(String::as_str).unwrap_or("ride");
    let class = args
        .get(2)
        .and_then(|c| BikeClass::from_id(c))
        .unwrap_or(BikeClass::EDirt);
    let checkpoint: usize = args.get(3).and_then(|c| c.parse().ok()).unwrap_or(0);

    let mut sim = Sim::new(Track::proving_grounds(), WorldTuning::default());
    let id = sim.add_rider(class.default_tuning(), checkpoint);
    let mut buf = Vec::new();
    let mut last_pitch = 0.0;

    let seconds = 10.0;
    let ticks = (seconds / DT) as u32;
    for tick in 0..ticks {
        let t = tick as f32 * DT;
        let input = match scenario {
            "settle" => Input::default(),
            "throttle" => Input::new(1.0, 0.0, 0.0, false, false),
            "wheelie" => {
                // Pop it with lean back, then feather the throttle around the balance point.
                let p = pitch(&sim);
                let rate = (p - last_pitch) / DT;
                last_pitch = p;
                let target = 0.72;
                if t < 0.5 {
                    Input::default()
                } else {
                    let throttle = (0.55 + 2.5 * (target - p) - 0.5 * rate).clamp(0.0, 1.0);
                    let brake = if p > target + 0.2 && rate > 0.0 {
                        1.0
                    } else {
                        0.0
                    };
                    let lean = if p < 0.25 { -1.0 } else { 0.0 };
                    Input::new(throttle, brake, lean, false, false)
                }
            }
            "backflip" => {
                // Full throttle to the lip, then lean back until nearly round, then level out.
                let hud = sim.hud(id).unwrap();
                if !hud.in_air {
                    Input::new(1.0, 0.0, 0.0, false, false)
                } else if hud.air_rotation < 5.2 {
                    Input::new(0.0, 0.0, -1.0, false, false)
                } else {
                    Input::new(0.0, 0.0, 0.3, false, false)
                }
            }
            "brake" => {
                if t < 3.0 {
                    Input::new(1.0, 0.0, 0.0, false, false)
                } else {
                    Input::new(0.0, 1.0, 0.0, false, false)
                }
            }
            _ => Input::new(1.0, 0.0, 0.0, false, false),
        };
        sim.set_input(id, input);
        sim.step();
        for e in sim.drain_events() {
            println!(
                "  t={:6.2} event {}",
                t,
                serde_json::to_string(&e.kind).unwrap()
            );
        }
        if scenario == "backflip" && sim.hud(id).unwrap().in_air && tick % 3 == 0 {
            let h = sim.hud(id).unwrap();
            println!(
                "  air t={:.2} rot={:.2} air_time={:.2}",
                t, h.air_rotation, h.air_time
            );
        }
        if tick % 15 == 0 && scenario != "backflip" {
            sim.render_state(&mut buf);
            let o = &buf[..RENDER_STRIDE];
            let hud = sim.hud(id).unwrap();
            println!(
                "t={:5.2} x={:7.2} y={:6.2} ang={:6.1}° v={:5.2} comp r/f={:6.3}/{:6.3} down r/f={}/{} hip=({:5.2},{:5.2}) status={} bal={:5.2} manual={:?} {:.1}m",
                t,
                o[L::CHASSIS],
                o[L::CHASSIS + 1],
                o[L::CHASSIS + 2].to_degrees(),
                o[L::SPEED],
                o[L::REAR_COMPRESSION],
                o[L::FRONT_COMPRESSION],
                (o[L::FLAGS] as u32) & 1,
                (o[L::FLAGS] as u32 >> 1) & 1,
                o[L::SKELETON + 4] - o[L::CHASSIS],
                o[L::SKELETON + 5] - o[L::CHASSIS + 1],
                o[L::STATUS],
                hud.balance,
                hud.manual,
                hud.manual_distance,
            );
        }
    }
    println!("hash {:016x}", sim.state_hash());
}

fn pitch(sim: &Sim) -> f32 {
    let mut buf = Vec::new();
    sim.render_state(&mut buf);
    buf[L::CHASSIS + 2] - sim.track().ground_angle(buf[L::CHASSIS], 2.0)
}
