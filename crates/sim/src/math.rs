//! Deterministic math helpers.
//!
//! `f32::sin` and friends call the platform's libm, which can differ in the last bit
//! between the native server and the WebAssembly client. Everything in the simulation
//! goes through the pure-Rust `libm` crate instead, so both sides compute identical
//! results. (`sqrt` is correctly rounded everywhere, so glam's `length()` is fine.)

pub use rapier2d::math::{Pose, Real, Rotation, Vector};

pub const PI: f32 = core::f32::consts::PI;
pub const TAU: f32 = core::f32::consts::TAU;

#[inline]
pub fn sin(x: f32) -> f32 {
    libm::sinf(x)
}

#[inline]
pub fn cos(x: f32) -> f32 {
    libm::cosf(x)
}

#[inline]
pub fn atan2(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}

#[inline]
pub fn acos(x: f32) -> f32 {
    libm::acosf(x.clamp(-1.0, 1.0))
}

/// Wraps an angle into `(-PI, PI]`.
pub fn wrap_angle(a: f32) -> f32 {
    let mut a = a % TAU;
    if a <= -PI {
        a += TAU;
    } else if a > PI {
        a -= TAU;
    }
    a
}

/// Rotates `v` counter-clockwise by `angle` radians.
#[inline]
pub fn rotate(v: Vector, angle: f32) -> Vector {
    let (s, c) = (sin(angle), cos(angle));
    Vector::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

/// Builds a pose from a translation and an angle using deterministic trig.
#[inline]
pub fn pose(translation: Vector, angle: f32) -> Pose {
    Pose::from_parts(
        translation,
        Rotation::from_cos_sin_unchecked(cos(angle), sin(angle)),
    )
}

/// The angle of a rotation, computed deterministically.
#[inline]
pub fn angle_of(rot: &Rotation) -> f32 {
    atan2(rot.im, rot.re)
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Moves `current` towards `target` by at most `max_delta`.
#[inline]
pub fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    if current < target {
        (current + max_delta).min(target)
    } else {
        (current - max_delta).max(target)
    }
}

#[inline]
pub fn v2(a: [f32; 2]) -> Vector {
    Vector::new(a[0], a[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_angle_stays_in_range() {
        for i in -40..40 {
            let a = i as f32 * 0.7;
            let w = wrap_angle(a);
            assert!(w > -PI - 1e-5 && w <= PI + 1e-5, "{a} -> {w}");
            assert!((sin(w) - sin(a)).abs() < 1e-4);
        }
    }

    #[test]
    fn rotate_quarter_turn() {
        let r = rotate(Vector::new(1.0, 0.0), PI / 2.0);
        assert!(r.x.abs() < 1e-6 && (r.y - 1.0).abs() < 1e-6);
    }
}
