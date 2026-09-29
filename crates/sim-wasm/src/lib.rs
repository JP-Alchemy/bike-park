//! WebAssembly bindings: the browser drives the same simulation the server runs.
//!
//! Kept deliberately thin. Bulk per-frame data (render state) crosses as a
//! `Float32Array`; low-frequency data (HUD, events, tuning, track) crosses as JSON.

use bike_sim::{BikeClass, BikeTuning, Input, Sim, Track, WorldTuning};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: &str);
}

#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        error(&format!("bike-sim panicked: {info}"))
    }));
}

fn class(id: &str) -> Result<BikeClass, JsError> {
    BikeClass::from_id(id).ok_or_else(|| JsError::new(&format!("unknown bike class '{id}'")))
}

#[wasm_bindgen]
pub struct Game {
    sim: Sim,
    render: Vec<f32>,
}

#[wasm_bindgen]
impl Game {
    /// A room on the prototype park.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game {
        Game {
            sim: Sim::new(Track::proving_grounds(), WorldTuning::default()),
            render: Vec::new(),
        }
    }

    #[wasm_bindgen(js_name = addRider)]
    pub fn add_rider(&mut self, class_id: &str, checkpoint: u32) -> Result<u32, JsError> {
        let tuning = class(class_id)?.default_tuning();
        Ok(self.sim.add_rider(tuning, checkpoint as usize))
    }

    #[wasm_bindgen(js_name = addBot)]
    pub fn add_bot(&mut self, class_id: &str, checkpoint: u32) -> Result<u32, JsError> {
        Ok(self.sim.add_bot(class(class_id)?, checkpoint as usize))
    }

    #[wasm_bindgen(js_name = removeRider)]
    pub fn remove_rider(&mut self, id: u32) {
        self.sim.remove_rider(id);
    }

    /// Throttle and brake in 0..1, lean in -1 (back) ..1 (forward).
    #[wasm_bindgen(js_name = setInput)]
    pub fn set_input(
        &mut self,
        id: u32,
        throttle: f32,
        brake: f32,
        lean: f32,
        trick: bool,
        respawn: bool,
    ) {
        self.sim
            .set_input(id, Input::new(throttle, brake, lean, trick, respawn));
    }

    /// Advances one fixed tick (1/60 s).
    pub fn step(&mut self) {
        self.sim.step();
    }

    pub fn tick(&self) -> u32 {
        self.sim.tick()
    }

    /// `renderStride()` floats per rider; layout in `web/src/sim/layout.ts`.
    #[wasm_bindgen(js_name = renderState)]
    pub fn render_state(&mut self) -> Vec<f32> {
        self.sim.render_state(&mut self.render);
        self.render.clone()
    }

    #[wasm_bindgen(js_name = hudJson)]
    pub fn hud_json(&self, id: u32) -> String {
        serde_json::to_string(&self.sim.hud(id)).unwrap_or_default()
    }

    /// Events since the last call, as a JSON array.
    #[wasm_bindgen(js_name = drainEventsJson)]
    pub fn drain_events_json(&mut self) -> String {
        serde_json::to_string(&self.sim.drain_events()).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = trackJson)]
    pub fn track_json(&self) -> String {
        self.sim.track().to_json()
    }

    #[wasm_bindgen(js_name = tuningJson)]
    pub fn tuning_json(&self, id: u32) -> String {
        self.sim
            .rider(id)
            .map(|r| r.tuning().to_json())
            .unwrap_or_default()
    }

    /// Replaces a rider's bike tuning (partial JSON is filled from defaults) and respawns them.
    #[wasm_bindgen(js_name = setTuningJson)]
    pub fn set_tuning_json(&mut self, id: u32, json: &str) -> Result<(), JsError> {
        let tuning = BikeTuning::from_json(json).map_err(|e| JsError::new(&e))?;
        self.sim.set_tuning(id, tuning);
        Ok(())
    }

    #[wasm_bindgen(js_name = worldTuningJson)]
    pub fn world_tuning_json(&self) -> String {
        serde_json::to_string_pretty(self.sim.world_tuning()).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = setWorldTuningJson)]
    pub fn set_world_tuning_json(&mut self, json: &str) -> Result<(), JsError> {
        let world: WorldTuning =
            serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
        self.sim.set_world_tuning(world);
        Ok(())
    }

    /// Hex hash of the full physics state, for determinism checks.
    #[wasm_bindgen(js_name = stateHash)]
    pub fn state_hash(&self) -> String {
        format!("{:016x}", self.sim.state_hash())
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_name = defaultTuningJson)]
pub fn default_tuning_json(class_id: &str) -> Result<String, JsError> {
    Ok(class(class_id)?.default_tuning().to_json())
}

#[wasm_bindgen(js_name = renderStride)]
pub fn render_stride() -> u32 {
    bike_sim::RENDER_STRIDE as u32
}

#[wasm_bindgen(js_name = tickRate)]
pub fn tick_rate() -> u32 {
    bike_sim::TICK_RATE
}

/// Runs the shared determinism scenario; must match the native result bit for bit.
#[wasm_bindgen(js_name = determinismScenario)]
pub fn determinism_scenario(ticks: u32) -> String {
    format!("{:016x}", bike_sim::check::determinism_scenario(ticks))
}
