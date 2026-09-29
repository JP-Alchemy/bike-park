// Mirror of `render_layout` in crates/sim/src/world.rs: floats per rider in
// Game.renderState(). Keep the two in sync (the Rust side asserts its own consistency).

export const L = {
  ID: 0,
  /** 0 riding, 1 crashed, 2 finished. */
  STATUS: 1,
  /** 0 e-dirt, 1 fatbike. */
  CLASS: 2,
  CHASSIS: 3,
  REAR: 6,
  FRONT: 9,
  /** 11 (x, y) skeleton points, see SK. */
  SKELETON: 12,
  TRICK: 34,
  TRICK_EXTENT: 35,
  SPEED: 36,
  REAR_COMPRESSION: 37,
  FRONT_COMPRESSION: 38,
  THROTTLE: 39,
  /** Bit 0 rear down, 1 front down, 2 in the air, 3 wheelie/stoppie. */
  FLAGS: 40,
} as const;

/** Skeleton point indices (rider.rs). "Left" is the side nearer the camera. */
export const SK = {
  HEAD: 0,
  NECK: 1,
  HIP: 2,
  KNEE_L: 3,
  FOOT_L: 4,
  KNEE_R: 5,
  FOOT_R: 6,
  ELBOW_L: 7,
  HAND_L: 8,
  ELBOW_R: 9,
  HAND_R: 10,
} as const;

export const SKELETON_POINTS = 11;

/** Numeric trick ids in the render state. */
export const TRICK_IDS = ["", "no_hander", "superman", "can_can", "tail_whip"] as const;
