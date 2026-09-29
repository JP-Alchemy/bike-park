//! The rider's visible skeleton, and the ragdoll that takes over after a crash.
//!
//! While riding, the rider is one physical body (hips, torso, head) sprung to the bike;
//! arms and legs are posed with two-bone IK onto the pegs and bars, and named tricks
//! blend the limbs into their poses. On a crash that body is swapped for a ragdoll that
//! starts in the same pose and at the same velocity, so the fall reads as continuous.

use rapier2d::prelude::*;

use crate::math::{acos, angle_of, atan2, cos, rotate, sin, v2};
use crate::physics::{groups, tag, Part, Physics, G_RIDER, G_TERRAIN};
use crate::scoring::TrickKind;
use crate::tuning::{BikeTuning, RiderTuning};

pub const HEAD: usize = 0;
pub const NECK: usize = 1;
pub const HIP: usize = 2;
pub const KNEE_L: usize = 3;
pub const FOOT_L: usize = 4;
pub const KNEE_R: usize = 5;
pub const FOOT_R: usize = 6;
pub const ELBOW_L: usize = 7;
pub const HAND_L: usize = 8;
pub const ELBOW_R: usize = 9;
pub const HAND_R: usize = 10;
pub const SKELETON_POINTS: usize = 11;

/// World-space joint positions, used by the renderer and to start the ragdoll.
/// "Left" is the side nearer the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skeleton {
    pub points: [Vector; SKELETON_POINTS],
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            points: [Vector::ZERO; SKELETON_POINTS],
        }
    }
}

/// Two-bone IK: the middle joint for a limb from `root` reaching for `target`.
/// `bend` is +1 to bend counter-clockwise of the root→target line (knees, facing right)
/// and -1 for clockwise (elbows).
fn two_bone(root: Vector, target: Vector, l1: f32, l2: f32, bend: f32) -> Vector {
    let d = target - root;
    let dist = d.length().clamp(1e-4, (l1 + l2) * 0.9999);
    let cos_a = (l1 * l1 + dist * dist - l2 * l2) / (2.0 * l1 * dist);
    let a = acos(cos_a) * bend;
    let base = atan2(d.y, d.x);
    root + Vector::new(cos(base + a), sin(base + a)) * l1
}

/// The riding pose, with an optional trick blended in (`extent` 0..1).
pub(crate) fn riding_skeleton(
    chassis: &Pose,
    rider: &Pose,
    t: &BikeTuning,
    rt: &RiderTuning,
    trick: Option<(TrickKind, f32)>,
) -> Skeleton {
    let angle = angle_of(&chassis.rotation);
    let local = |p: Vector| rotate(p, angle);
    let hip = rider.translation;
    let neck = rider.transform_point(v2(rt.neck_offset));
    let head = rider.transform_point(v2(rt.head_offset));
    let pegs = chassis.transform_point(v2(t.pegs));
    let bars = chassis.transform_point(v2(t.bars));

    let (mut foot_l, mut foot_r, mut hand_l, mut hand_r) = (pegs, pegs, bars, bars);
    if let Some((kind, e)) = trick {
        let blend = |from: Vector, to: Vector| from + (to - from) * e;
        match kind {
            TrickKind::NoHander => {
                hand_l = blend(bars, neck + local(Vector::new(-0.12, 0.50)));
                hand_r = blend(bars, neck + local(Vector::new(0.06, 0.52)));
            }
            TrickKind::Superman => {
                foot_l = blend(pegs, hip + local(Vector::new(-0.80, 0.34)));
                foot_r = blend(pegs, hip + local(Vector::new(-0.78, 0.22)));
            }
            TrickKind::CanCan => {
                foot_l = blend(pegs, hip + local(Vector::new(0.62, 0.02)));
            }
            TrickKind::TailWhip => {}
        }
    }

    let mut s = Skeleton::default();
    s.points[HEAD] = head;
    s.points[NECK] = neck;
    s.points[HIP] = hip;
    s.points[FOOT_L] = foot_l;
    s.points[FOOT_R] = foot_r;
    s.points[HAND_L] = hand_l;
    s.points[HAND_R] = hand_r;
    s.points[KNEE_L] = two_bone(hip, foot_l, rt.thigh, rt.shin, 1.0);
    s.points[KNEE_R] = two_bone(hip, foot_r, rt.thigh, rt.shin, 1.0);
    s.points[ELBOW_L] = two_bone(neck, hand_l, rt.upper_arm, rt.forearm, -1.0);
    s.points[ELBOW_R] = two_bone(neck, hand_r, rt.upper_arm, rt.forearm, -1.0);
    s
}

struct Limb {
    body: RigidBodyHandle,
    len: f32,
}

/// Relative joint angle ranges (child minus parent, rad), facing right.
const HIP_LIMITS: [f32; 2] = [-2.3, 0.85];
const KNEE_LIMITS: [f32; 2] = [-2.7, 0.0];
const ELBOW_LIMITS: [f32; 2] = [0.0, 2.7];

/// One ragdoll joint: `child` hangs off `parent` at `anchor` (parent space), with
/// optional angle limits and a little "muscle tone" (N·m) so limbs don't flop freely.
struct JointSpec {
    parent: RigidBodyHandle,
    anchor: Vector,
    child: RigidBodyHandle,
    limits: Option<[f32; 2]>,
    tone: f32,
}

impl JointSpec {
    fn new(
        parent: RigidBodyHandle,
        anchor: Vector,
        child: RigidBodyHandle,
        limits: Option<[f32; 2]>,
        tone: f32,
    ) -> Self {
        Self {
            parent,
            anchor,
            child,
            limits,
            tone,
        }
    }
}

pub(crate) struct Ragdoll {
    /// Hips, torso and head; same local frame as the rider body it replaced.
    torso: RigidBodyHandle,
    neck_offset: Vector,
    head_offset: Vector,
    /// thigh L, shin L, thigh R, shin R, upper arm L, forearm L, upper arm R, forearm R.
    limbs: Vec<Limb>,
}

impl Ragdoll {
    /// Spawns a ragdoll matching `pose`, moving like `source` (the rider body).
    pub fn spawn(
        phys: &mut Physics,
        id: u32,
        rt: &RiderTuning,
        pose: &Skeleton,
        torso_pose: Pose,
        source: RigidBodyHandle,
    ) -> Ragdoll {
        let src = &phys.bodies[source];
        let (src_linvel, src_angvel, src_com) = (src.linvel(), src.angvel(), src.center_of_mass());
        let velocity_at = |p: Vector| {
            let r = p - src_com;
            src_linvel + Vector::new(-r.y, r.x) * src_angvel
        };
        let rider_groups = groups(G_RIDER, G_TERRAIN);
        let collider = |shape: ColliderBuilder, mass: f32| {
            shape
                .mass(mass)
                .friction(0.7)
                .restitution(0.1)
                .collision_groups(rider_groups)
                .user_data(tag(Part::Ragdoll, id))
        };

        let torso = phys.bodies.insert(
            RigidBodyBuilder::dynamic()
                .pose(torso_pose)
                .linvel(velocity_at(torso_pose.translation))
                .angvel(src_angvel)
                .angular_damping(0.6)
                .user_data(id as u128),
        );
        phys.colliders.insert_with_parent(
            collider(
                ColliderBuilder::capsule_from_endpoints(
                    Vector::new(0.0, 0.06),
                    v2(rt.neck_offset),
                    rt.torso_radius,
                ),
                0.46 * rt.mass,
            ),
            torso,
            &mut phys.bodies,
        );
        phys.colliders.insert_with_parent(
            collider(ColliderBuilder::ball(rt.head_radius), 0.1 * rt.mass)
                .translation(v2(rt.head_offset)),
            torso,
            &mut phys.bodies,
        );

        // (from, to, radius, mass share)
        let p = &pose.points;
        let specs = [
            (p[HIP], p[KNEE_L], 0.075, 0.10),
            (p[KNEE_L], p[FOOT_L], 0.06, 0.07),
            (p[HIP], p[KNEE_R], 0.075, 0.10),
            (p[KNEE_R], p[FOOT_R], 0.06, 0.07),
            (p[NECK], p[ELBOW_L], 0.05, 0.04),
            (p[ELBOW_L], p[HAND_L], 0.045, 0.03),
            (p[NECK], p[ELBOW_R], 0.05, 0.04),
            (p[ELBOW_R], p[HAND_R], 0.045, 0.03),
        ];
        let limbs: Vec<Limb> = specs
            .iter()
            .map(|&(a, b, radius, share)| {
                let d = b - a;
                let len = d.length().max(0.05);
                let body = phys.bodies.insert(
                    RigidBodyBuilder::dynamic()
                        .pose(crate::math::pose(a, atan2(d.y, d.x)))
                        .linvel(velocity_at(a + d * 0.5))
                        .angvel(src_angvel)
                        .angular_damping(0.6)
                        .user_data(id as u128),
                );
                phys.colliders.insert_with_parent(
                    collider(
                        ColliderBuilder::capsule_from_endpoints(
                            Vector::ZERO,
                            Vector::new(len, 0.0),
                            radius,
                        ),
                        share * rt.mass,
                    ),
                    body,
                    &mut phys.bodies,
                );
                Limb { body, len }
            })
            .collect();

        let ragdoll = Ragdoll {
            torso,
            neck_offset: v2(rt.neck_offset),
            head_offset: v2(rt.head_offset),
            limbs,
        };

        let l = &ragdoll.limbs;
        let end = |i: usize| Vector::new(l[i].len, 0.0);
        let neck = v2(rt.neck_offset);
        let joints = [
            JointSpec::new(torso, Vector::ZERO, l[0].body, Some(HIP_LIMITS), 10.0),
            JointSpec::new(l[0].body, end(0), l[1].body, Some(KNEE_LIMITS), 6.0),
            JointSpec::new(torso, Vector::ZERO, l[2].body, Some(HIP_LIMITS), 10.0),
            JointSpec::new(l[2].body, end(2), l[3].body, Some(KNEE_LIMITS), 6.0),
            JointSpec::new(torso, neck, l[4].body, None, 3.0),
            JointSpec::new(l[4].body, end(4), l[5].body, Some(ELBOW_LIMITS), 2.0),
            JointSpec::new(torso, neck, l[6].body, None, 3.0),
            JointSpec::new(l[6].body, end(6), l[7].body, Some(ELBOW_LIMITS), 2.0),
        ];
        for JointSpec {
            parent,
            anchor,
            child,
            limits,
            tone,
        } in joints
        {
            let mut joint = RevoluteJointBuilder::new()
                .local_anchor1(anchor)
                .local_anchor2(Vector::ZERO)
                .motor_model(MotorModel::ForceBased)
                .motor_velocity(0.0, 50.0)
                .motor_max_force(tone)
                .contacts_enabled(false);
            if let Some([lo, hi]) = limits {
                // Never start outside the limits, or the joint would snap on the first step.
                let rel = crate::math::wrap_angle(
                    angle_of(phys.bodies[child].rotation())
                        - angle_of(phys.bodies[parent].rotation()),
                );
                joint = joint.limits([lo.min(rel), hi.max(rel)]);
            }
            phys.joints.insert(parent, child, joint, true);
        }
        ragdoll
    }

    pub fn bodies(&self) -> impl Iterator<Item = RigidBodyHandle> + '_ {
        core::iter::once(self.torso).chain(self.limbs.iter().map(|l| l.body))
    }

    pub fn skeleton(&self, phys: &Physics) -> Skeleton {
        let torso = phys.bodies[self.torso].position();
        let end = |i: usize| {
            let l = &self.limbs[i];
            phys.bodies[l.body]
                .position()
                .transform_point(Vector::new(l.len, 0.0))
        };
        let mut s = Skeleton::default();
        s.points[HIP] = torso.translation;
        s.points[NECK] = torso.transform_point(self.neck_offset);
        s.points[HEAD] = torso.transform_point(self.head_offset);
        s.points[KNEE_L] = end(0);
        s.points[FOOT_L] = end(1);
        s.points[KNEE_R] = end(2);
        s.points[FOOT_R] = end(3);
        s.points[ELBOW_L] = end(4);
        s.points[HAND_L] = end(5);
        s.points[ELBOW_R] = end(6);
        s.points[HAND_R] = end(7);
        s
    }

    pub fn destroy(self, phys: &mut Physics) {
        for b in self.bodies().collect::<Vec<_>>() {
            phys.remove_body(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_bone_keeps_bone_lengths() {
        let root = Vector::new(0.0, 1.0);
        let target = Vector::new(0.2, 0.2);
        let mid = two_bone(root, target, 0.45, 0.45, 1.0);
        assert!(((mid - root).length() - 0.45).abs() < 1e-4);
        assert!(((target - mid).length() - 0.45).abs() < 1e-3);
        // Knees (bend +1) point forward, towards +x, for a leg hanging down.
        assert!(mid.x > 0.1);
    }

    #[test]
    fn two_bone_straightens_when_out_of_reach() {
        let mid = two_bone(Vector::ZERO, Vector::new(3.0, 0.0), 0.5, 0.5, 1.0);
        assert!((mid - Vector::new(0.5, 0.0)).length() < 1e-2);
    }
}
