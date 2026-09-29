//! Bike Park simulation core.
//!
//! A deterministic 2D riding simulation (Rapier 2D) shared by the browser client,
//! compiled to WebAssembly, and the authoritative game server. See
//! `docs/architecture.md` for how the pieces fit together.

pub mod bot;
pub mod check;
pub mod events;
pub mod input;
pub mod math;
mod physics;
pub mod rider;
mod rig;
pub mod scoring;
pub mod track;
pub mod tuning;
pub mod world;

pub use events::{Event, EventKind};
pub use input::Input;
pub use scoring::{CrashReason, ManualKind, TrickKind};
pub use track::Track;
pub use tuning::{BikeClass, BikeTuning, RiderTuning, WorldTuning};
pub use world::{HudState, RiderId, Sim, DT, RENDER_STRIDE, TICK_RATE};
