//! Where water comes from and goes, saved beside a map (#100, F4).
//!
//! Engine-free. A [`WaterPlan`] is the author's input to `bevy_elements`' hydrology: springs
//! (`WaterSource`, m³/s over a footprint), sinks (`WaterSink`) and lakes (a level the water is held
//! to over a footprint). The field names match the crate's own `WaterSource` and `WaterSink`, so the
//! game's copy of the plugin reads the same numbers. The plan is data; running the simulation is a
//! separate, explicit step and never part of placing.

use serde::{Deserialize, Serialize};

/// Pours water in: `rate_m3_s` over the cells within `radius_m` of `(x, z)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spring {
    pub x: f32,
    pub z: f32,
    pub rate_m3_s: f32,
    pub radius_m: f32,
}

/// Drains water: up to `rate_m3_s` from the cells within `radius_m` of `(x, z)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sink {
    pub x: f32,
    pub z: f32,
    pub rate_m3_s: f32,
    pub radius_m: f32,
}

/// A lake: the water surface is held at `level_m` within `radius_m` of `(x, z)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lake {
    pub x: f32,
    pub z: f32,
    pub radius_m: f32,
    pub level_m: f32,
}

/// One thing to place.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Placed {
    Spring(Spring),
    Sink(Sink),
    Lake(Lake),
}

/// Every water source, sink and lake of a map, in the order they were placed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WaterPlan {
    #[serde(default)]
    pub springs: Vec<Spring>,
    #[serde(default)]
    pub sinks: Vec<Sink>,
    #[serde(default)]
    pub lakes: Vec<Lake>,
}

impl WaterPlan {
    pub fn is_empty(&self) -> bool {
        self.springs.is_empty() && self.sinks.is_empty() && self.lakes.is_empty()
    }

    /// Add one; a non-finite number or a non-positive radius or rate is refused, and nothing changes.
    pub fn place(&mut self, what: Placed) -> Result<(), String> {
        let (nums, radius, rate) = match what {
            Placed::Spring(s) => ([s.x, s.z, s.rate_m3_s, s.radius_m], s.radius_m, Some(s.rate_m3_s)),
            Placed::Sink(s) => ([s.x, s.z, s.rate_m3_s, s.radius_m], s.radius_m, Some(s.rate_m3_s)),
            Placed::Lake(l) => ([l.x, l.z, l.radius_m, l.level_m], l.radius_m, None),
        };
        if nums.iter().any(|n| !n.is_finite()) {
            return Err("every number must be finite".to_owned());
        }
        if radius <= 0.0 {
            return Err("the radius must be above zero".to_owned());
        }
        if rate.is_some_and(|r| r <= 0.0) {
            return Err("the rate must be above zero".to_owned());
        }
        match what {
            Placed::Spring(s) => self.springs.push(s),
            Placed::Sink(s) => self.sinks.push(s),
            Placed::Lake(l) => self.lakes.push(l),
        }
        Ok(())
    }
}
