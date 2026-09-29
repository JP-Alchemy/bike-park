//! Events the simulation reports to the game (HUD, audio, camera, analytics).

use serde::Serialize;

use crate::scoring::{CrashReason, ManualKind, TrickKind};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    TakeOff,
    Landed {
        air_time: f32,
    },
    TrickStarted {
        trick: TrickKind,
    },
    TrickLanded {
        name: String,
        points: u32,
    },
    ManualStart {
        kind: ManualKind,
    },
    ManualEnd {
        kind: ManualKind,
        distance: f32,
    },
    ComboBanked {
        points: u32,
        tricks: u32,
        multiplier: u32,
    },
    ComboLost {
        points: u32,
        tricks: u32,
    },
    Crash {
        reason: CrashReason,
        speed: f32,
    },
    Respawn {
        checkpoint: usize,
    },
    Checkpoint {
        index: usize,
    },
    RunStart,
    Finish {
        time: f32,
        best: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Event {
    pub rider: u32,
    pub tick: u32,
    #[serde(flatten)]
    pub kind: EventKind,
}
