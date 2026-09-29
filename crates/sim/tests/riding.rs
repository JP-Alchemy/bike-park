//! Riding behaviour: the things the prototype must get right before anything else.
//! These pin down the feel so tuning changes can't silently break it.

use bike_sim::world::render_layout as L;
use bike_sim::{
    BikeClass, CrashReason, EventKind, HudState, Input, ManualKind, RiderId, Sim, Track,
    WorldTuning, DT,
};

const THROTTLE: Input = Input {
    throttle: 255,
    brake: 0,
    lean: 0,
    buttons: 0,
};

fn sim_with(class: BikeClass, checkpoint: usize) -> (Sim, RiderId) {
    let mut sim = Sim::new(Track::proving_grounds(), WorldTuning::default());
    let id = sim.add_rider(class.default_tuning(), checkpoint);
    (sim, id)
}

fn seconds(s: f32) -> u32 {
    (s / DT) as u32
}

struct Frame {
    hud: HudState,
    angle: f32,
    events: Vec<EventKind>,
}

/// Runs `ticks` ticks, choosing each input from the previous frame.
fn drive(
    sim: &mut Sim,
    id: RiderId,
    ticks: u32,
    mut control: impl FnMut(&Sim, u32) -> Input,
) -> Vec<Frame> {
    let mut frames = Vec::new();
    let mut buf = Vec::new();
    for t in 0..ticks {
        let input = control(sim, t);
        sim.set_input(id, input);
        sim.step();
        sim.render_state(&mut buf);
        frames.push(Frame {
            hud: sim.hud(id).unwrap(),
            angle: buf[L::CHASSIS + 2],
            events: sim.drain_events().into_iter().map(|e| e.kind).collect(),
        });
    }
    frames
}

fn pitch(sim: &Sim) -> f32 {
    let mut buf = Vec::new();
    sim.render_state(&mut buf);
    buf[L::CHASSIS + 2] - sim.track().ground_angle(buf[L::CHASSIS], 2.0)
}

fn crashed(frames: &[Frame]) -> Option<CrashReason> {
    frames.iter().flat_map(|f| &f.events).find_map(|e| match e {
        EventKind::Crash { reason, .. } => Some(*reason),
        _ => None,
    })
}

#[test]
fn bikes_settle_on_both_wheels() {
    for class in BikeClass::ALL {
        let (mut sim, id) = sim_with(class, 0);
        drive(&mut sim, id, seconds(2.0), |_, _| Input::default());
        let rider = sim.rider(id).unwrap();
        assert_eq!(rider.wheels_down(), (true, true), "{class:?}");
        assert!(!rider.is_crashed());
        assert!(pitch(&sim).abs() < 0.05, "{class:?} rests level");
        assert!(sim.hud(id).unwrap().speed < 0.1, "{class:?} stays put");
    }
}

#[test]
fn full_throttle_accelerates_without_looping_out() {
    let (mut sim, id) = sim_with(BikeClass::EDirt, 0);
    let frames = drive(&mut sim, id, seconds(1.6), |_, _| THROTTLE);
    assert_eq!(crashed(&frames), None);
    assert!(sim.hud(id).unwrap().speed > 9.0);
    let max_angle = frames.iter().map(|f| f.angle).fold(f32::MIN, f32::max);
    assert!(
        max_angle < 0.35,
        "nose rose to {max_angle} rad with throttle alone"
    );
}

#[test]
fn e_dirt_is_quicker_than_the_fatbike() {
    let speed_after = |class| {
        let (mut sim, id) = sim_with(class, 0);
        drive(&mut sim, id, seconds(3.0), |_, _| THROTTLE);
        sim.hud(id).unwrap().speed
    };
    assert!(speed_after(BikeClass::EDirt) > speed_after(BikeClass::Fatbike) + 1.0);
}

/// Pop with lean back, then feather the throttle like a player holding the balance point.
fn wheelie_controller(target: f32) -> impl FnMut(&Sim, u32) -> Input {
    let mut last = 0.0;
    move |sim, _| {
        let p = pitch(sim);
        let rate = (p - last) / DT;
        last = p;
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

#[test]
fn a_wheelie_can_be_popped_and_held() {
    // Checkpoint 1 is the start of the wheelie strip.
    let (mut sim, id) = sim_with(BikeClass::EDirt, 1);
    let frames = drive(&mut sim, id, seconds(4.0), wheelie_controller(0.72));
    assert_eq!(crashed(&frames), None);
    let hud = &frames.last().unwrap().hud;
    assert_eq!(hud.manual, Some(ManualKind::Wheelie));
    assert!(
        hud.manual_distance > 5.0,
        "held for {} m",
        hud.manual_distance
    );
}

#[test]
fn fatbike_is_slower_to_pitch_up() {
    let peak = |class| {
        let (mut sim, id) = sim_with(class, 2);
        let frames = drive(&mut sim, id, seconds(0.7), |_, _| {
            Input::new(1.0, 0.0, -1.0, false, false)
        });
        frames.iter().map(|f| f.angle).fold(f32::MIN, f32::max)
    };
    let (e_dirt, fatbike) = (peak(BikeClass::EDirt), peak(BikeClass::Fatbike));
    assert!(
        fatbike < e_dirt * 0.6,
        "fatbike {fatbike} vs e-dirt {e_dirt}"
    );
}

#[test]
fn looping_out_crashes_then_respawns() {
    let (mut sim, id) = sim_with(BikeClass::EDirt, 0);
    let frames = drive(&mut sim, id, seconds(5.0), |_, _| {
        Input::new(1.0, 0.0, -1.0, false, false)
    });
    assert_eq!(crashed(&frames), Some(CrashReason::Head));
    let respawned = frames
        .iter()
        .flat_map(|f| &f.events)
        .any(|e| matches!(e, EventKind::Respawn { checkpoint: 0 }));
    assert!(respawned, "respawns after the crash");
}

#[test]
fn backflip_off_big_air_scores() {
    let (mut sim, id) = sim_with(BikeClass::EDirt, 7);
    let frames = drive(&mut sim, id, seconds(8.0), |sim, _| {
        let hud = sim.hud(id).unwrap();
        if !hud.in_air {
            THROTTLE
        } else if hud.air_rotation < 5.2 {
            Input::new(0.0, 0.0, -1.0, false, false)
        } else {
            Input::new(0.0, 0.0, 0.3, false, false)
        }
    });
    assert_eq!(crashed(&frames), None);
    let flipped = frames.iter().flat_map(|f| &f.events).any(
        |e| matches!(e, EventKind::TrickLanded { name, points } if name == "Backflip" && *points == 500),
    );
    assert!(flipped);
}

#[test]
fn landing_mid_trick_is_a_bail() {
    let (mut sim, id) = sim_with(BikeClass::EDirt, 7);
    let frames = drive(&mut sim, id, seconds(8.0), |sim, _| {
        let hud = sim.hud(id).unwrap();
        if !hud.in_air {
            return THROTTLE;
        }
        // Superman (trick + up) held all the way down, keeping the bike level.
        let up = hud.trick.is_none();
        let lean = (1.5 * pitch(sim)).clamp(-1.0, 1.0);
        Input::new(if up { 1.0 } else { 0.0 }, 0.0, lean, true, false)
    });
    let started = frames.iter().flat_map(|f| &f.events).any(|e| {
        matches!(
            e,
            EventKind::TrickStarted {
                trick: bike_sim::TrickKind::Superman
            }
        )
    });
    assert!(started);
    assert_eq!(crashed(&frames), Some(CrashReason::Bail));
}

#[test]
fn respawn_button_returns_to_the_checkpoint() {
    let (mut sim, id) = sim_with(BikeClass::EDirt, 1);
    drive(&mut sim, id, seconds(2.0), |_, _| THROTTLE);
    let before = sim.hud(id).unwrap().progress;
    let frames = drive(&mut sim, id, 2, |_, t| {
        Input::new(0.0, 0.0, 0.0, false, t == 0)
    });
    let after = sim.hud(id).unwrap().progress;
    assert!(frames[0]
        .events
        .iter()
        .any(|e| matches!(e, EventKind::Respawn { checkpoint: 1 })));
    assert!(after < before);
}

#[test]
fn bots_ride_the_whole_park() {
    for class in BikeClass::ALL {
        let mut sim = Sim::new(Track::proving_grounds(), WorldTuning::default());
        let id = sim.add_bot(class, 0);
        let mut finished = false;
        for _ in 0..seconds(80.0) {
            sim.step();
            finished |= sim
                .drain_events()
                .iter()
                .any(|e| e.rider == id && matches!(e.kind, EventKind::Finish { .. }));
            if finished {
                break;
            }
        }
        assert!(finished, "{class:?} bot should finish the park");
    }
}

#[test]
fn changing_tuning_respawns_with_the_new_bike() {
    let (mut sim, id) = sim_with(BikeClass::EDirt, 0);
    drive(&mut sim, id, seconds(1.0), |_, _| THROTTLE);
    sim.set_tuning(id, BikeClass::Fatbike.default_tuning());
    assert_eq!(sim.rider(id).unwrap().tuning().class, BikeClass::Fatbike);
    drive(&mut sim, id, seconds(1.0), |_, _| Input::default());
    assert_eq!(sim.rider(id).unwrap().wheels_down(), (true, true));
}
