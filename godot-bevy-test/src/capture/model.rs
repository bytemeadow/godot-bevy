use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureManifest {
    pub version: u32,
    pub example: String,
    pub scenario: String,
    pub scene: String,
    pub pacing: Pacing,
    pub extensions: BTreeMap<String, serde_json::Value>,
    pub frames: u32,
    pub seed: u64,
    pub viewport: [u32; 2],
    pub clocks: Clocks,
    pub settle: Settle,
    pub checkpoints: Vec<Checkpoint>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clocks {
    pub physics_hz: u32,
    pub bevy_step_ns: u64,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settle {
    pub stable_frames: u32,
    pub nodes: Vec<String>,
    pub classes: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub frame: u32,
    pub nodes: Vec<NodeExpectation>,
    pub regions: Vec<RegionExpectation>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeExpectation {
    pub path: String,
    pub position: [f64; 2],
    pub tolerance: f64,
    pub visible: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionExpectation {
    pub name: String,
    pub rect: [u32; 4],
    pub non_blank: NonBlank,
    pub dominant_colour: DominantColour,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonBlank {
    pub background: [u8; 3],
    pub tolerance: u8,
    pub min_fraction: f64,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DominantColour {
    pub rgb: [u8; 3],
    pub tolerance: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Pacing {
    Fixed,
    Realtime,
}
