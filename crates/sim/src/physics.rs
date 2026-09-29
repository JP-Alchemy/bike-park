//! Thin wrapper around the Rapier world plus the collision-group and tagging rules.

use rapier2d::prelude::*;

use crate::track::Track;

/// Collision groups. Riders never collide with each other or themselves here: races use
/// non-colliding "ghost" riders. Party modes with collisions will add a contact filter.
pub(crate) const G_TERRAIN: Group = Group::GROUP_1;
pub(crate) const G_BIKE: Group = Group::GROUP_2;
pub(crate) const G_RIDER: Group = Group::GROUP_3;

pub(crate) fn groups(member: Group, filter: Group) -> InteractionGroups {
    InteractionGroups::new(member, filter, InteractionTestMode::And)
}

/// What a collider is, stored in its `user_data` as `kind | rider_id << 8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Part {
    Terrain = 1,
    Chassis = 2,
    Wheel = 3,
    RiderBody = 4,
    Ragdoll = 5,
}

pub(crate) fn tag(part: Part, rider: u32) -> u128 {
    part as u128 | ((rider as u128) << 8)
}

pub(crate) fn part_of(user_data: u128) -> u8 {
    (user_data & 0xff) as u8
}

pub(crate) struct Physics {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    soft_bodies: SoftBodySet,
    islands: IslandManager,
    broad_phase: BroadPhaseBvh,
    pub narrow_phase: NarrowPhase,
    ccd: CCDSolver,
    pipeline: PhysicsPipeline,
    pub params: IntegrationParameters,
    pub gravity: Vector,
}

impl Physics {
    pub fn new(dt: f32, gravity: f32, solver_iterations: usize) -> Self {
        let params = IntegrationParameters {
            dt,
            num_solver_iterations: solver_iterations.max(1),
            ..Default::default()
        };
        Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            soft_bodies: SoftBodySet::new(),
            islands: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            ccd: CCDSolver::new(),
            pipeline: PhysicsPipeline::new(),
            params,
            gravity: Vector::new(0.0, -gravity),
        }
    }

    /// Adds the track's ground as one fixed polyline collider.
    pub fn add_terrain(&mut self, track: &Track) -> ColliderHandle {
        // Oriented polylines keep the solid on the right of each segment, so the
        // vertices run right to left to put the solid ground below the line. They also
        // clamp contact normals at the joints, so wheels don't snag on segment seams.
        let vertices: Vec<Vector> = track
            .ground
            .iter()
            .rev()
            .map(|p| Vector::new(p[0], p[1]))
            .collect();
        let body = self.bodies.insert(RigidBodyBuilder::fixed());
        self.colliders.insert_with_parent(
            ColliderBuilder::oriented_polyline(vertices, None)
                .friction(1.0)
                .collision_groups(groups(G_TERRAIN, G_BIKE | G_RIDER))
                .user_data(tag(Part::Terrain, 0)),
            body,
            &mut self.bodies,
        )
    }

    pub fn step(&mut self) {
        self.pipeline.step(
            self.gravity,
            &self.params,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            &mut self.multibody_joints,
            &mut self.soft_bodies,
            &mut self.ccd,
            &(),
            &(),
        );
    }

    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.joints,
            &mut self.multibody_joints,
            &mut self.soft_bodies,
            true,
        );
    }

    /// Is `collider` touching the terrain (within `slack` metres)?
    pub fn touches_terrain(&self, collider: ColliderHandle, slack: f32) -> bool {
        self.narrow_phase.contact_pairs_with(collider).any(|pair| {
            let other = if pair.collider1 == collider {
                pair.collider2
            } else {
                pair.collider1
            };
            let is_terrain = self
                .colliders
                .get(other)
                .is_some_and(|c| part_of(c.user_data) == Part::Terrain as u8);
            is_terrain
                && pair
                    .manifolds()
                    .iter()
                    .any(|m| m.points.iter().any(|p| p.dist < slack))
        })
    }
}
