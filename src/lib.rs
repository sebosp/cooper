//! cooper, an online SC2Replay analyser: the replays are read in the browser with s2protocol.

pub mod analysis;
#[cfg(target_arch = "wasm32")]
pub mod app;
#[cfg(target_arch = "wasm32")]
pub mod chart;
