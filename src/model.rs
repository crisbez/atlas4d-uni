
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position { pub lat: f64, pub lon: f64, pub alt_m: f64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub obs_id: u64,
    pub entity_id: Uuid,
    pub t: DateTime<Utc>,
    pub pos: Position,
    pub quality: f32,
    pub sigma_m: Option<f64>,
    pub source: Option<serde_json::Value>,
}

impl Observation {
    pub fn new(entity_id: Uuid, t: DateTime<Utc>, pos: Position, sigma_m: Option<f64>, quality: f32, source: Option<serde_json::Value>) -> Self {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        entity_id.hash(&mut hasher);
        ((t.timestamp_nanos_opt().unwrap_or(0)) as i128).hash(&mut hasher);
        (pos.lat.to_bits()).hash(&mut hasher);
        (pos.lon.to_bits()).hash(&mut hasher);
        let obs_id = (hasher.finish()) as u64;
        Observation { obs_id, entity_id, t, pos, quality, sigma_m, source }
    }
}
