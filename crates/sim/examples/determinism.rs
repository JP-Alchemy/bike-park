//! Prints the hash of the shared determinism scenario (see `bike_sim::check`).
//! `scripts/check-determinism.mjs` compares it with the WebAssembly build's hash.
//!
//! cargo run -p bike-sim --release --example determinism -- [ticks]

fn main() {
    let ticks: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800);
    println!("{:016x}", bike_sim::check::determinism_scenario(ticks));
}
