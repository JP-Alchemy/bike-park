//! Player input for one simulation tick.
//!
//! Inputs are quantized to bytes so that the client's prediction and the server's
//! authoritative simulation see bit-identical values, and so they are 4 bytes on the wire.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Input {
    /// 0 (off) ..= 255 (full throttle).
    pub throttle: u8,
    /// 0 (off) ..= 255 (full brake). Brakes both wheels.
    pub brake: u8,
    /// -127 (lean back, nose up) ..= 127 (lean forward, nose down).
    pub lean: i8,
    /// Bit flags, see [`Input::TRICK`] and [`Input::RESPAWN`].
    pub buttons: u8,
}

impl Input {
    /// The trick button. In the air it starts a named trick chosen by the held direction.
    pub const TRICK: u8 = 1 << 0;
    /// Ask to be put back on the track.
    pub const RESPAWN: u8 = 1 << 1;

    /// Builds a quantized input from analog values (throttle/brake in 0..1, lean in -1..1).
    pub fn new(throttle: f32, brake: f32, lean: f32, trick: bool, respawn: bool) -> Self {
        let mut buttons = 0;
        if trick {
            buttons |= Self::TRICK;
        }
        if respawn {
            buttons |= Self::RESPAWN;
        }
        Self {
            throttle: quantize_unit(throttle),
            brake: quantize_unit(brake),
            lean: (lean.clamp(-1.0, 1.0) * 127.0).round() as i8,
            buttons,
        }
    }

    pub fn throttle(&self) -> f32 {
        self.throttle as f32 / 255.0
    }

    pub fn brake(&self) -> f32 {
        self.brake as f32 / 255.0
    }

    /// Positive leans forward (nose down), negative leans back (nose up).
    pub fn lean(&self) -> f32 {
        (self.lean as f32 / 127.0).clamp(-1.0, 1.0)
    }

    pub fn trick(&self) -> bool {
        self.buttons & Self::TRICK != 0
    }

    pub fn respawn(&self) -> bool {
        self.buttons & Self::RESPAWN != 0
    }

    pub fn to_bytes(self) -> [u8; 4] {
        [self.throttle, self.brake, self.lean as u8, self.buttons]
    }

    pub fn from_bytes(b: [u8; 4]) -> Self {
        Self {
            throttle: b[0],
            brake: b[1],
            lean: b[2] as i8,
            buttons: b[3],
        }
    }
}

fn quantize_unit(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_bytes() {
        let i = Input::new(0.5, 0.0, -1.0, true, false);
        assert_eq!(Input::from_bytes(i.to_bytes()), i);
        assert_eq!(i.lean(), -1.0);
        assert!(i.trick() && !i.respawn());
        assert!((i.throttle() - 0.5).abs() < 0.01);
    }
}
