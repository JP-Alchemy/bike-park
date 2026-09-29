//! Tuning data: everything that decides how a bike feels.
//!
//! The riding feel is the whole game, so every number that shapes it lives here as
//! plain data. The web client's tuning panel edits these live and can export them as
//! JSON; paste an exported set back into the defaults below once it feels right.
//!
//! Units are SI: metres, kilograms, seconds, newtons, newton-metres, watts.
//! Bike space: origin halfway between the two axles at axle height, +x forward, +y up.

use serde::{Deserialize, Serialize};

/// The playable bike classes. All designs are generic: no real brands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BikeClass {
    /// Punchy torque, light, quiet, easy wheelies.
    EDirt,
    /// Heavy, grippy, stable, slow to flip.
    Fatbike,
}

impl BikeClass {
    pub const ALL: [BikeClass; 2] = [BikeClass::EDirt, BikeClass::Fatbike];

    pub fn id(self) -> &'static str {
        match self {
            BikeClass::EDirt => "e_dirt",
            BikeClass::Fatbike => "fatbike",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }

    pub fn default_tuning(self) -> BikeTuning {
        match self {
            BikeClass::EDirt => BikeTuning::e_dirt(),
            BikeClass::Fatbike => BikeTuning::fatbike(),
        }
    }
}

/// One wheel's suspension: a spring-damper sliding along `axis`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Suspension {
    /// Direction the wheel moves when the suspension compresses (bike space, need not be unit).
    pub axis: [f32; 2],
    /// How far the wheel can move up from its rest position (m).
    pub travel: f32,
    /// How far the wheel can droop below its rest position when unloaded (m).
    pub droop: f32,
    /// Spring rate at the wheel (N/m). Preload is derived so the bike sits at its rest pose.
    pub stiffness: f32,
    /// Damping at the wheel (N·s/m).
    pub damping: f32,
}

impl Default for Suspension {
    fn default() -> Self {
        Self {
            axis: [0.0, 1.0],
            travel: 0.15,
            droop: 0.04,
            stiffness: 9000.0,
            damping: 700.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BikeTuning {
    pub class: BikeClass,
    pub name: String,

    // --- Geometry (bike space) ---
    pub rear_axle: [f32; 2],
    pub front_axle: [f32; 2],
    pub rear_wheel_radius: f32,
    pub front_wheel_radius: f32,
    /// Frame collision capsule, from `frame_rear` to `frame_front`.
    pub frame_rear: [f32; 2],
    pub frame_front: [f32; 2],
    pub frame_radius: f32,
    /// Where the rider's feet, hands and hips sit.
    pub pegs: [f32; 2],
    pub bars: [f32; 2],
    pub hips: [f32; 2],

    // --- Mass ---
    pub chassis_mass: f32,
    /// Centre of mass of the frame (bike space).
    pub chassis_com: [f32; 2],
    pub chassis_inertia: f32,
    pub wheel_mass: f32,
    /// Wheel inertia as a fraction of m·r² (0.5 = solid disc, 1.0 = all mass at the rim).
    pub wheel_inertia_factor: f32,

    // --- Suspension ---
    pub rear_suspension: Suspension,
    pub front_suspension: Suspension,

    // --- Tyres ---
    pub tyre_friction: f32,
    pub tyre_restitution: f32,

    // --- Motor (electric: flat torque, then constant power) ---
    /// Peak torque at the rear wheel (N·m).
    pub max_torque: f32,
    /// Peak power (W). Above the "base speed" torque falls off as power / wheel speed.
    pub max_power: f32,
    /// Top speed on the flat (m/s); the motor stops pushing past this.
    pub top_speed: f32,
    /// Motor drag when coasting (N·m).
    pub coast_torque: f32,
    /// Quadratic air drag (N per (m/s)²).
    pub air_drag: f32,

    // --- Brakes (N·m) ---
    pub rear_brake_torque: f32,
    pub front_brake_torque: f32,

    // --- Lean and air control ---
    /// Torque from leaning while at least one wheel is down (N·m).
    pub ground_lean_torque: f32,
    /// Pitch damping while balancing a wheelie; gives kids more time to react (N·m·s).
    pub wheelie_assist: f32,
    /// Spin speed at full lean in the air (rad/s).
    pub air_spin_rate: f32,
    /// How hard the air control chases `air_spin_rate` (N·m per rad/s).
    pub air_spin_gain: f32,
    /// Cap on the air control torque (N·m).
    pub air_spin_max_torque: f32,
    /// Spin damping in the air with no lean input (N·m·s).
    pub air_spin_damping: f32,
    /// How far the rider shifts their hips fore/aft at full lean (m).
    pub rider_lean_shift: f32,
}

/// The rider: the same kid on every bike.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RiderTuning {
    pub mass: f32,
    pub inertia: f32,
    /// Centre of mass relative to the hips (bike axes).
    pub com_offset: [f32; 2],
    /// Head centre relative to the hips (bike axes). The head is the crash sensor.
    pub head_offset: [f32; 2],
    pub head_radius: f32,
    /// Torso capsule from the hips to `neck_offset`.
    pub neck_offset: [f32; 2],
    pub torso_radius: f32,
    pub thigh: f32,
    pub shin: f32,
    pub upper_arm: f32,
    pub forearm: f32,
    /// Fore/aft spring holding the hips (N/m, N·s/m).
    pub lean_stiffness: f32,
    pub lean_damping: f32,
    /// Legs: vertical spring holding the hips (N/m, N·s/m) and its range (m).
    pub leg_stiffness: f32,
    pub leg_damping: f32,
    pub leg_squat: f32,
    pub leg_extend: f32,
}

/// World-level settings shared by every rider in a room.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldTuning {
    /// Downward gravity (m/s²). Higher than Earth's so jumps feel snappy, not floaty.
    pub gravity: f32,
    /// Solver substeps per tick.
    pub solver_iterations: usize,
    /// Seconds between a crash and the automatic respawn.
    pub respawn_delay: f32,
    /// Seconds a combo stays open after its last trick before it is banked.
    pub combo_window: f32,
}

impl Default for WorldTuning {
    fn default() -> Self {
        Self {
            gravity: 14.0,
            solver_iterations: 6,
            respawn_delay: 1.6,
            combo_window: 2.0,
        }
    }
}

impl Default for RiderTuning {
    fn default() -> Self {
        Self {
            mass: 48.0,
            inertia: 3.5,
            com_offset: [0.04, 0.16],
            head_offset: [0.22, 0.66],
            head_radius: 0.14,
            neck_offset: [0.16, 0.48],
            torso_radius: 0.13,
            thigh: 0.42,
            shin: 0.42,
            upper_arm: 0.29,
            forearm: 0.28,
            lean_stiffness: 14000.0,
            lean_damping: 1100.0,
            leg_stiffness: 16000.0,
            leg_damping: 1300.0,
            leg_squat: 0.22,
            leg_extend: 0.08,
        }
    }
}

impl Default for BikeTuning {
    fn default() -> Self {
        Self::e_dirt()
    }
}

impl BikeTuning {
    /// Electric dirt bike: punchy torque, light, quiet, easy wheelies.
    pub fn e_dirt() -> Self {
        Self {
            class: BikeClass::EDirt,
            name: "E-Dirt".into(),
            rear_axle: [-0.62, 0.0],
            front_axle: [0.62, 0.0],
            rear_wheel_radius: 0.33,
            front_wheel_radius: 0.33,
            frame_rear: [-0.38, 0.34],
            frame_front: [0.34, 0.42],
            frame_radius: 0.11,
            pegs: [-0.06, 0.03],
            bars: [0.30, 0.74],
            hips: [-0.20, 0.62],
            chassis_mass: 38.0,
            chassis_com: [0.02, 0.22],
            chassis_inertia: 5.5,
            wheel_mass: 4.5,
            wheel_inertia_factor: 0.75,
            rear_suspension: Suspension {
                axis: [-0.15, 1.0],
                travel: 0.20,
                droop: 0.05,
                stiffness: 9000.0,
                damping: 620.0,
            },
            front_suspension: Suspension {
                axis: [-0.42, 0.91],
                travel: 0.20,
                droop: 0.05,
                stiffness: 7500.0,
                damping: 520.0,
            },
            tyre_friction: 1.2,
            tyre_restitution: 0.0,
            max_torque: 250.0,
            max_power: 6500.0,
            top_speed: 17.0,
            coast_torque: 6.0,
            air_drag: 0.35,
            rear_brake_torque: 420.0,
            front_brake_torque: 380.0,
            ground_lean_torque: 110.0,
            wheelie_assist: 110.0,
            air_spin_rate: 6.8,
            air_spin_gain: 420.0,
            air_spin_max_torque: 820.0,
            air_spin_damping: 4.0,
            rider_lean_shift: 0.20,
        }
    }

    /// Fatbike: heavy, grippy, stable, slow to flip.
    pub fn fatbike() -> Self {
        Self {
            class: BikeClass::Fatbike,
            name: "Fatbike".into(),
            rear_axle: [-0.60, 0.0],
            front_axle: [0.60, 0.0],
            rear_wheel_radius: 0.34,
            front_wheel_radius: 0.34,
            frame_rear: [-0.36, 0.30],
            frame_front: [0.30, 0.38],
            frame_radius: 0.10,
            pegs: [-0.02, 0.02],
            bars: [0.26, 0.72],
            hips: [-0.22, 0.64],
            chassis_mass: 52.0,
            chassis_com: [0.08, 0.14],
            chassis_inertia: 8.5,
            wheel_mass: 6.5,
            wheel_inertia_factor: 0.8,
            rear_suspension: Suspension {
                axis: [0.0, 1.0],
                travel: 0.07,
                droop: 0.02,
                stiffness: 16000.0,
                damping: 1100.0,
            },
            front_suspension: Suspension {
                axis: [-0.36, 0.93],
                travel: 0.09,
                droop: 0.02,
                stiffness: 12000.0,
                damping: 900.0,
            },
            tyre_friction: 1.6,
            tyre_restitution: 0.0,
            max_torque: 300.0,
            max_power: 3000.0,
            top_speed: 13.0,
            coast_torque: 9.0,
            air_drag: 0.45,
            rear_brake_torque: 380.0,
            front_brake_torque: 300.0,
            ground_lean_torque: 160.0,
            wheelie_assist: 140.0,
            air_spin_rate: 4.8,
            air_spin_gain: 420.0,
            air_spin_max_torque: 640.0,
            air_spin_damping: 8.0,
            rider_lean_shift: 0.16,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("tuning serializes")
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_and_partial_json_uses_defaults() {
        for class in BikeClass::ALL {
            let t = class.default_tuning();
            assert_eq!(BikeTuning::from_json(&t.to_json()).unwrap(), t);
        }
        let partial = BikeTuning::from_json(r#"{"max_torque": 123.0}"#).unwrap();
        assert_eq!(partial.max_torque, 123.0);
        assert_eq!(partial.top_speed, BikeTuning::e_dirt().top_speed);
    }

    #[test]
    fn class_ids_roundtrip() {
        for class in BikeClass::ALL {
            assert_eq!(BikeClass::from_id(class.id()), Some(class));
        }
    }
}
