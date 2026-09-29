//! The physical bike: a chassis, two sprung wheels and a sprung rider.
//!
//! ```text
//!            rider body (hips, torso, head) ── 2-axis spring: lean shift + legs
//!                 │
//!   rear wheel ── chassis ── front wheel      each wheel: pin-slot joint = spring
//!                                             along the suspension axis, free spin,
//!                                             angular motor for drive and brakes
//! ```
//!
//! Wheelies, stoppies, "throttle lifts the nose" and "brake drops the nose" all come out
//! of the physics: the drive and brake motors push on the wheel and the frame equally
//! and oppositely, and the friction force at the tyre sits below the centre of mass.

use rapier2d::prelude::*;

use crate::math::{rotate, v2};
use crate::physics::{groups, tag, Part, Physics, G_BIKE, G_RIDER, G_TERRAIN};
use crate::tuning::{BikeTuning, RiderTuning, Suspension};

/// A high motor damping turns the velocity motors into "hold this speed, using at most
/// `max_force`": exactly an electric motor's torque limit, or a brake's grip.
const MOTOR_DAMPING: f32 = 5000.0;

pub(crate) struct Rig {
    pub chassis: RigidBodyHandle,
    pub rear: RigidBodyHandle,
    pub front: RigidBodyHandle,
    pub rear_joint: ImpulseJointHandle,
    pub front_joint: ImpulseJointHandle,
    pub rear_collider: ColliderHandle,
    pub front_collider: ColliderHandle,
    /// The rider while still on the bike; `None` once they have been thrown off.
    pub rider: Option<RiderBody>,
}

pub(crate) struct RiderBody {
    pub body: RigidBodyHandle,
    pub joint: ImpulseJointHandle,
    pub head: ColliderHandle,
    pub torso: ColliderHandle,
}

/// Per-tick commands for the rig, already mapped from player input.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Controls {
    pub throttle: f32,
    pub brake: f32,
    /// Positive leans forward.
    pub lean: f32,
}

fn unit(a: [f32; 2]) -> Vector {
    let v = v2(a);
    let len = v.length();
    if len > 1e-6 {
        v / len
    } else {
        Vector::new(0.0, 1.0)
    }
}

impl Rig {
    /// Builds the bike (and rider) with the chassis origin at `origin`, rotated by `angle`.
    pub fn build(
        phys: &mut Physics,
        id: u32,
        t: &BikeTuning,
        rt: &RiderTuning,
        origin: Vector,
        angle: f32,
    ) -> Rig {
        let g = -phys.gravity.y;
        let world = |p: [f32; 2]| origin + rotate(v2(p), angle);
        let bike_groups = groups(G_BIKE, G_TERRAIN);

        let chassis = phys.bodies.insert(
            RigidBodyBuilder::dynamic()
                .pose(crate::math::pose(origin, angle))
                .additional_mass_properties(MassProperties::new(
                    v2(t.chassis_com),
                    t.chassis_mass,
                    t.chassis_inertia,
                ))
                .user_data(id as u128),
        );
        phys.colliders.insert_with_parent(
            ColliderBuilder::capsule_from_endpoints(
                v2(t.frame_rear),
                v2(t.frame_front),
                t.frame_radius,
            )
            .density(0.0)
            .friction(0.6)
            .collision_groups(bike_groups)
            .user_data(tag(Part::Chassis, id)),
            chassis,
            &mut phys.bodies,
        );

        // Static load on each spring, used to preload it so the bike rests at its design pose.
        let sprung = t.chassis_mass + rt.mass;
        let rider_com_x = t.hips[0] + rt.com_offset[0];
        let com_x = (t.chassis_mass * t.chassis_com[0] + rt.mass * rider_com_x) / sprung;
        let wheelbase = t.front_axle[0] - t.rear_axle[0];
        let rear_share = ((t.front_axle[0] - com_x) / wheelbase).clamp(0.05, 0.95);
        let rear_load = rear_share * sprung * g;
        let front_load = (1.0 - rear_share) * sprung * g;

        let mut make_wheel = |axle: [f32; 2], radius: f32, s: &Suspension, load: f32| {
            let body = phys.bodies.insert(
                RigidBodyBuilder::dynamic()
                    .pose(crate::math::pose(world(axle), angle))
                    .additional_mass_properties(MassProperties::new(
                        Vector::ZERO,
                        t.wheel_mass,
                        t.wheel_inertia_factor * t.wheel_mass * radius * radius,
                    ))
                    .ccd_enabled(true)
                    .user_data(id as u128),
            );
            let collider = phys.colliders.insert_with_parent(
                ColliderBuilder::ball(radius)
                    .density(0.0)
                    .friction(t.tyre_friction)
                    .restitution(t.tyre_restitution)
                    .collision_groups(bike_groups)
                    .user_data(tag(Part::Wheel, id)),
                body,
                &mut phys.bodies,
            );
            let axis = unit(s.axis);
            // The spring pushes along `axis`; its vertical share carries the load.
            let preload = -(load / axis.y.max(0.3)) / s.stiffness.max(1.0);
            // Lock sideways motion, leave compression and spin free. Compression is
            // positive along the joint's X axis (the suspension axis, pointing up).
            let joint = GenericJointBuilder::new(JointAxesMask::LIN_Y)
                .local_anchor1(v2(axle))
                .local_axis1(axis)
                .local_anchor2(Vector::ZERO)
                .limits(JointAxis::LinX, [-s.droop, s.travel])
                .motor_model(JointAxis::LinX, MotorModel::ForceBased)
                .motor_position(JointAxis::LinX, preload, s.stiffness, s.damping)
                .motor_model(JointAxis::AngX, MotorModel::ForceBased)
                .motor_velocity(JointAxis::AngX, 0.0, MOTOR_DAMPING)
                .motor_max_force(JointAxis::AngX, t.coast_torque)
                .contacts_enabled(false)
                .build();
            let joint = phys.joints.insert(chassis, body, joint, true);
            (body, collider, joint)
        };

        let (rear, rear_collider, rear_joint) = make_wheel(
            t.rear_axle,
            t.rear_wheel_radius,
            &t.rear_suspension,
            rear_load,
        );
        let (front, front_collider, front_joint) = make_wheel(
            t.front_axle,
            t.front_wheel_radius,
            &t.front_suspension,
            front_load,
        );

        let rider = Some(RiderBody::build(
            phys,
            id,
            t,
            rt,
            chassis,
            world(t.hips),
            angle,
        ));

        Rig {
            chassis,
            rear,
            front,
            rear_joint,
            front_joint,
            rear_collider,
            front_collider,
            rider,
        }
    }

    pub fn bodies(&self) -> impl Iterator<Item = RigidBodyHandle> + '_ {
        [self.chassis, self.rear, self.front]
            .into_iter()
            .chain(self.rider.as_ref().map(|r| r.body))
    }

    pub fn destroy(self, phys: &mut Physics) {
        for b in self.bodies().collect::<Vec<_>>() {
            phys.remove_body(b);
        }
    }

    /// Applies throttle, brakes, lean and air drag for this tick.
    pub fn apply_controls(
        &mut self,
        phys: &mut Physics,
        t: &BikeTuning,
        c: Controls,
        rear_ground: bool,
        front_ground: bool,
    ) {
        let chassis = &phys.bodies[self.chassis];
        let chassis_angvel = chassis.angvel();
        let chassis_vel = chassis.linvel();
        let rel_spin = phys.bodies[self.rear].angvel() - chassis_angvel;

        // Rear wheel: brake beats throttle; otherwise the motor delivers torque up to its
        // flat-torque / constant-power curve until the wheel reaches top speed.
        let (rear_target, rear_force) = if c.brake > 0.0 {
            (0.0, c.brake * t.rear_brake_torque)
        } else if c.throttle > 0.0 {
            let spin = rel_spin.abs().max(1e-3);
            let available = t.max_torque.min(t.max_power / spin);
            (
                -t.top_speed / t.rear_wheel_radius,
                c.throttle * available.max(t.coast_torque),
            )
        } else {
            (0.0, t.coast_torque)
        };
        let front_force = if c.brake > 0.0 {
            c.brake * t.front_brake_torque
        } else {
            t.coast_torque * 0.25
        };
        set_spin_motor(phys, self.rear_joint, rear_target, rear_force);
        set_spin_motor(phys, self.front_joint, 0.0, front_force);

        // Rider weight shift.
        if let Some(r) = &self.rider {
            if let Some(j) = phys.joints.get_mut(r.joint, false) {
                let m = j.data.motor(JointAxis::LinX).copied().unwrap_or_default();
                j.data.set_motor_position(
                    JointAxis::LinX,
                    c.lean * t.rider_lean_shift,
                    m.stiffness,
                    m.damping,
                );
            }
        }

        // Lean torque: a direct push on the ground, a spin-rate controller in the air.
        let torque = if rear_ground || front_ground {
            let mut torque = -c.lean * t.ground_lean_torque;
            if rear_ground && !front_ground {
                torque -= t.wheelie_assist * chassis_angvel;
            }
            torque
        } else if c.lean.abs() > 0.01 {
            let target = -c.lean * t.air_spin_rate;
            (t.air_spin_gain * (target - chassis_angvel))
                .clamp(-t.air_spin_max_torque, t.air_spin_max_torque)
        } else {
            -t.air_spin_damping * chassis_angvel
        };

        let speed = chassis_vel.length();
        let drag = -chassis_vel * (t.air_drag * speed);

        let chassis = &mut phys.bodies[self.chassis];
        chassis.reset_forces(false);
        chassis.reset_torques(false);
        chassis.add_torque(torque, true);
        chassis.add_force(drag, true);
    }

    /// Engine off, brakes released: used once the rider has been thrown off.
    pub fn cut_power(&mut self, phys: &mut Physics, t: &BikeTuning) {
        set_spin_motor(phys, self.rear_joint, 0.0, t.coast_torque);
        set_spin_motor(phys, self.front_joint, 0.0, t.coast_torque * 0.25);
        let chassis = &mut phys.bodies[self.chassis];
        chassis.reset_forces(false);
        chassis.reset_torques(false);
    }

    /// Suspension compression of (rear, front), in metres from the rest pose.
    pub fn compression(&self, phys: &Physics) -> (f32, f32) {
        let comp = |joint: ImpulseJointHandle, wheel: RigidBodyHandle| {
            let Some(j) = phys.joints.get(joint) else {
                return 0.0;
            };
            let frame1 = *phys.bodies[self.chassis].position() * j.data.local_frame1;
            let axis = frame1.rotation.transform_vector(Vector::X);
            (phys.bodies[wheel].translation() - frame1.translation).dot(axis)
        };
        (
            comp(self.rear_joint, self.rear),
            comp(self.front_joint, self.front),
        )
    }

    /// Mass-weighted centre of the whole rig (bike + rider if still aboard).
    pub fn center_of_mass(&self, phys: &Physics) -> Vector {
        let mut sum = Vector::ZERO;
        let mut mass = 0.0;
        for h in self.bodies() {
            let b = &phys.bodies[h];
            sum += b.center_of_mass() * b.mass();
            mass += b.mass();
        }
        sum / mass.max(1e-6)
    }
}

fn set_spin_motor(phys: &mut Physics, joint: ImpulseJointHandle, target_vel: f32, max_torque: f32) {
    if let Some(j) = phys.joints.get_mut(joint, true) {
        j.data
            .set_motor_velocity(JointAxis::AngX, target_vel, MOTOR_DAMPING)
            .set_motor_max_force(JointAxis::AngX, max_torque.max(0.0));
    }
}

impl RiderBody {
    fn build(
        phys: &mut Physics,
        id: u32,
        t: &BikeTuning,
        rt: &RiderTuning,
        chassis: RigidBodyHandle,
        hips: Vector,
        angle: f32,
    ) -> RiderBody {
        let g = -phys.gravity.y;
        let body = phys.bodies.insert(
            RigidBodyBuilder::dynamic()
                .pose(crate::math::pose(hips, angle))
                .additional_mass_properties(MassProperties::new(
                    v2(rt.com_offset),
                    rt.mass,
                    rt.inertia,
                ))
                .user_data(id as u128),
        );
        let rider_groups = groups(G_RIDER, G_TERRAIN);
        let head = phys.colliders.insert_with_parent(
            ColliderBuilder::ball(rt.head_radius)
                .translation(v2(rt.head_offset))
                .density(0.0)
                .friction(0.8)
                .collision_groups(rider_groups)
                .user_data(tag(Part::RiderBody, id)),
            body,
            &mut phys.bodies,
        );
        let torso = phys.colliders.insert_with_parent(
            ColliderBuilder::capsule_from_endpoints(
                Vector::new(0.0, 0.08),
                v2(rt.neck_offset),
                rt.torso_radius,
            )
            .density(0.0)
            .friction(0.8)
            .collision_groups(rider_groups)
            .user_data(tag(Part::RiderBody, id)),
            body,
            &mut phys.bodies,
        );
        let leg_preload = rt.mass * g / rt.leg_stiffness.max(1.0);
        let shift = t.rider_lean_shift + 0.05;
        // Rider pitch is locked to the bike; hips slide fore/aft (lean) and up/down (legs).
        let joint = GenericJointBuilder::new(JointAxesMask::ANG_X)
            .local_anchor1(v2(t.hips))
            .local_anchor2(Vector::ZERO)
            .limits(JointAxis::LinX, [-shift, shift])
            .limits(JointAxis::LinY, [-rt.leg_squat, rt.leg_extend])
            .motor_model(JointAxis::LinX, MotorModel::ForceBased)
            .motor_position(JointAxis::LinX, 0.0, rt.lean_stiffness, rt.lean_damping)
            .motor_model(JointAxis::LinY, MotorModel::ForceBased)
            .motor_position(
                JointAxis::LinY,
                leg_preload,
                rt.leg_stiffness,
                rt.leg_damping,
            )
            .contacts_enabled(false)
            .build();
        let joint = phys.joints.insert(chassis, body, joint, true);
        RiderBody {
            body,
            joint,
            head,
            torso,
        }
    }
}
