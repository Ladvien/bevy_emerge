//! **Relations, not coordinates** (#137, research lens #2).
//!
//! An agent placing pieces by number writes coordinates, and what it *meant* is lost: "the chair
//! faces the table", "nothing stands in front of the door". This module keeps that intent as data
//! beside the map (`<map>.relations.ron`, [`FILE_SUFFIX`]) and checks a map's placements against it,
//! so a broken relation is named with what was measured rather than discovered by looking.
//!
//! Each [`Relation`] is about one placement, [`Relation::of`], by its stable id
//! ([`crate::map::Placed::id`]); a kind that needs a second piece names it too. [`check`] returns one
//! [`Outcome`] per relation, in order. A relation naming an id the map does not hold is
//! [`Outcome::Missing`], never silently met.
//!
//! Forward is `(sin yaw, cos yaw)`, the convention [`crate::placement::ir::facing_cosine`] scores
//! with. Distances are between placement centres on the floor plan ([`crate::map::Placed::at`]).
//!
//! Engine-free: reads placements, nothing else. Runs and stamps are not expanded here; a relation is
//! about a placement row.

use serde::{Deserialize, Serialize};

use crate::map::Placed;

/// The format version of a relations file.
pub const RELATIONS_VERSION: u32 = 1;

/// A map's relations live in `<map name>` + this, beside the map file.
pub const FILE_SUFFIX: &str = ".relations.ron";

/// How far off dead-on, in degrees, a piece may point and still face its target.
pub const FACING_TOLERANCE_DEG: f32 = 30.0;

/// Every relation one map is meant to keep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relations {
    pub version: u32,
    #[serde(default)]
    pub relations: Vec<Relation>,
}

impl Default for Relations {
    fn default() -> Self {
        Relations {
            version: RELATIONS_VERSION,
            relations: Vec::new(),
        }
    }
}

impl Relations {
    /// Read a relations file, refusing a version this build does not read.
    pub fn parse(text: &str) -> Result<Relations, String> {
        let r: Relations = ron::from_str(text).map_err(|e| format!("relations: parse: {e}"))?;
        if r.version != RELATIONS_VERSION {
            return Err(format!(
                "relations: version {} is not the {RELATIONS_VERSION} this build reads",
                r.version
            ));
        }
        Ok(r)
    }

    /// The file's text.
    pub fn to_ron(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("relations: write: {e}"))
    }
}

/// One thing a placement is meant to keep true.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    /// The placement id this relation is about.
    pub of: String,
    pub kind: Kind,
}

/// What a relation asks of [`Relation::of`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    /// Points at this placement, within [`FACING_TOLERANCE_DEG`]. Measures the angle off, degrees.
    Faces(String),
    /// Stands within this many metres of a placement. Measures the distance.
    Near { to: String, within: f32 },
    /// Stands at least this many metres from a placement. Measures the distance.
    Apart { from: String, at_least: f32 },
    /// Stands in front of a placement: ahead of its forward, inside a 45° half-cone, and no more than
    /// `within` metres along it. Measures the distance along the other's forward.
    InFrontOf { of: String, within: f32 },
    /// No other placement's centre stands in the `width` × `depth` corridor in front of this one (a
    /// door's swing, a counter's working side). Measures how far in the nearest intruder stands, or
    /// `depth` when clear.
    ClearInFront { depth: f32, width: f32 },
}

/// What [`check`] found for one relation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Outcome {
    Met {
        measured: f32,
    },
    /// Broken. `by` names the placement that breaks it, where one does (the intruder in a clear
    /// corridor).
    Unmet {
        measured: f32,
        by: Option<String>,
    },
    /// A placement the relation names is not on the map.
    Missing {
        id: String,
    },
}

/// Check every relation against a map's placements: one outcome per relation, in order.
pub fn check(placements: &[Placed], relations: &Relations) -> Vec<Outcome> {
    relations
        .relations
        .iter()
        .map(|r| check_one(placements, r))
        .collect()
}

fn find<'a>(placements: &'a [Placed], id: &str) -> Result<&'a Placed, Outcome> {
    placements
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| Outcome::Missing { id: id.to_owned() })
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

/// `p` in `from`'s frame: (along its forward, sideways).
fn local(from: &Placed, p: (f32, f32)) -> (f32, f32) {
    let (fx, fz) = (from.yaw.sin(), from.yaw.cos());
    let (dx, dz) = (p.0 - from.at.0, p.1 - from.at.1);
    (fx * dx + fz * dz, fz * dx - fx * dz)
}

fn verdict(ok: bool, measured: f32) -> Outcome {
    if ok {
        Outcome::Met { measured }
    } else {
        Outcome::Unmet { measured, by: None }
    }
}

fn check_one(placements: &[Placed], r: &Relation) -> Outcome {
    let a = match find(placements, &r.of) {
        Ok(a) => a,
        Err(missing) => return missing,
    };
    let other = |id: &str| find(placements, id);
    match &r.kind {
        Kind::Faces(to) => match other(to) {
            Ok(b) => {
                let cos = crate::placement::ir::facing_cosine(a.at, a.yaw, b.at).clamp(-1.0, 1.0);
                let off = cos.acos().to_degrees();
                verdict(off <= FACING_TOLERANCE_DEG, off)
            }
            Err(m) => m,
        },
        Kind::Near { to, within } => match other(to) {
            Ok(b) => {
                let d = distance(a.at, b.at);
                verdict(d <= *within, d)
            }
            Err(m) => m,
        },
        Kind::Apart { from, at_least } => match other(from) {
            Ok(b) => {
                let d = distance(a.at, b.at);
                verdict(d >= *at_least, d)
            }
            Err(m) => m,
        },
        Kind::InFrontOf { of, within } => match other(of) {
            Ok(b) => {
                let (ahead, side) = local(b, a.at);
                verdict(
                    ahead > 0.0 && ahead <= *within && side.abs() <= ahead,
                    ahead,
                )
            }
            Err(m) => m,
        },
        Kind::ClearInFront { depth, width } => {
            let intruder = placements
                .iter()
                .filter(|p| p.id != a.id)
                .map(|p| (p, local(a, p.at)))
                .filter(|(_, (ahead, side))| {
                    *ahead > 0.0 && ahead <= depth && side.abs() <= width / 2.0
                })
                .min_by(|x, y| x.1 .0.total_cmp(&y.1 .0));
            match intruder {
                Some((p, (ahead, _))) => Outcome::Unmet {
                    measured: ahead,
                    by: Some(p.id.clone()),
                },
                None => Outcome::Met { measured: *depth },
            }
        }
    }
}
