//! The simulation must be deterministic: same start, same inputs, same result.
//! (Cross-platform native vs WebAssembly is checked by `scripts/check-determinism.mjs`.)

use bike_sim::check::{determinism_scenario, scripted_input};
use bike_sim::{BikeClass, Input, Sim, Track, WorldTuning};

fn scenario() -> (Sim, u32, u32) {
    let mut sim = Sim::new(Track::proving_grounds(), WorldTuning::default());
    let a = sim.add_rider(BikeClass::EDirt.default_tuning(), 0);
    let b = sim.add_rider(BikeClass::Fatbike.default_tuning(), 2);
    sim.add_bot(BikeClass::EDirt, 1);
    sim.add_bot(BikeClass::Fatbike, 4);
    (sim, a, b)
}

#[test]
fn same_inputs_give_the_same_state_every_tick() {
    let (mut s1, a1, b1) = scenario();
    let (mut s2, a2, b2) = scenario();
    for t in 0..1500 {
        s1.set_input(a1, scripted_input(t));
        s2.set_input(a2, scripted_input(t));
        s1.set_input(b1, scripted_input(t + 99));
        s2.set_input(b2, scripted_input(t + 99));
        s1.step();
        s2.step();
        assert_eq!(
            s1.drain_events(),
            s2.drain_events(),
            "events diverged at tick {t}"
        );
        if t % 30 == 0 {
            assert_eq!(
                s1.state_hash(),
                s2.state_hash(),
                "state diverged at tick {t}"
            );
        }
    }
    assert_eq!(s1.state_hash(), s2.state_hash());
}

#[test]
fn a_single_different_input_changes_the_outcome() {
    let (mut s1, a1, _) = scenario();
    let (mut s2, a2, _) = scenario();
    let mut diverged = false;
    for t in 0..300 {
        s1.set_input(a1, Input::new(1.0, 0.0, 0.0, false, false));
        let input = if t == 100 {
            Input::new(1.0, 0.0, -1.0, false, false)
        } else {
            Input::new(1.0, 0.0, 0.0, false, false)
        };
        s2.set_input(a2, input);
        s1.step();
        s2.step();
        if t < 100 {
            assert_eq!(s1.state_hash(), s2.state_hash());
        } else {
            diverged |= s1.state_hash() != s2.state_hash();
        }
    }
    assert!(diverged, "one tick of lean should change the state");
}

#[test]
fn the_shared_scenario_is_repeatable() {
    assert_eq!(determinism_scenario(900), determinism_scenario(900));
}
