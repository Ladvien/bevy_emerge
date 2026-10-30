//! **Which repeats a player will notice** (#136, research lens #5).
//!
//! A count of copies says nothing useful: a town holds hundreds of identical fence pieces and wall
//! segments because structure is *meant* to tile, and a handful of barrels that should not look
//! stamped. Shipped level-art practice treats the two differently — repeated clutter reads as
//! copy-paste, repeated structure reads as a building — so this weighs a repeat by the role of the
//! piece repeated.
//!
//! # What counts as a repeat
//!
//! A **visible repeat** is two placements of the same descriptor (patch included), at the same yaw
//! within [`YAW_TOLERANCE_DEG`] (a full turn is the same look), with the same tip, closer together
//! than [`RepetitionRules::radius_m`]. Turning or tipping a copy is the cheapest variation an author
//! has, so it ends the repeat; a copy out of sight of the other is no repeat either.
//!
//! A piece's **role** is read off its `kind` tokens: any token in [`RepetitionRules::structure`]
//! makes it [`Role::Structure`], everything else is [`Role::Clutter`]. The score of a descriptor is
//! the number of its placements that have a look-alike in range, times its role's weight — linear in
//! pieces rather than pairs, so a long fence run does not grow quadratically past a cluster of crates.
//!
//! Engine-free: reads a map's placements and a library, nothing else.

use serde::{Deserialize, Serialize};

use crate::library::Library;
use crate::map::Placed;
use crate::stack::descriptor_of;

/// Two yaws closer than this, in degrees, are one look.
pub const YAW_TOLERANCE_DEG: f32 = 1.0;

/// What a repeat is, and what it costs, as data an author can tune.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RepetitionRules {
    /// Two look-alikes closer than this, in metres, are seen together.
    pub radius_m: f32,
    /// `kind` tokens that make a piece structure: meant to repeat.
    pub structure: Vec<String>,
    /// What one repeated structure piece weighs against one repeated clutter piece (1.0).
    pub structure_weight: f32,
}

impl Default for RepetitionRules {
    fn default() -> Self {
        Self {
            radius_m: 6.0,
            structure: ["building-module", "fence", "tile", "landmark", "grave", "door"]
                .map(str::to_owned)
                .to_vec(),
            structure_weight: 0.1,
        }
    }
}

/// Whether a piece is meant to repeat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Walls, fences, tiles: a run of copies is the design.
    Structure,
    /// Props and dressing: a run of copies reads as stamped.
    Clutter,
}

/// One descriptor's visible repeats.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Repeat {
    /// The descriptor repeated. A patched copy is a different look, so one descriptor can head two
    /// rows: its plain copies and its copies under one patch.
    pub descriptor: String,
    pub role: Role,
    /// Pairs of look-alikes in range of each other.
    pub pairs: usize,
    /// Placements with at least one look-alike in range.
    pub pieces: usize,
    /// `pieces` times the role's weight. The report is sorted by this, highest first.
    pub score: f32,
}

/// Every descriptor of `placements` that repeats visibly, the one a player notices first, first.
///
/// Placements whose descriptor the library does not hold are skipped: they have no role to weigh.
/// Ties are broken by descriptor id, so the report is deterministic.
pub fn repeats(placements: &[Placed], library: &Library, rules: &RepetitionRules) -> Vec<Repeat> {
    // Group by the look: descriptor id plus patch, so a patched copy is a different piece.
    let mut groups: Vec<(String, Role, Vec<&Placed>)> = Vec::new();
    for p in placements {
        let Some(d) = descriptor_of(library, p) else {
            continue;
        };
        let key = look_key(p);
        match groups.iter_mut().find(|(k, ..)| *k == key) {
            Some((.., rows)) => rows.push(p),
            None => {
                let role = if d.kind.iter().any(|k| rules.structure.contains(k)) {
                    Role::Structure
                } else {
                    Role::Clutter
                };
                groups.push((key, role, vec![p]));
            }
        }
    }

    let radius2 = rules.radius_m * rules.radius_m;
    let mut report: Vec<Repeat> = groups
        .into_iter()
        .filter_map(|(_, role, rows)| {
            let mut paired = vec![false; rows.len()];
            let mut pairs = 0;
            for i in 0..rows.len() {
                for j in i + 1..rows.len() {
                    if look_alike(rows[i], rows[j], radius2) {
                        pairs += 1;
                        paired[i] = true;
                        paired[j] = true;
                    }
                }
            }
            if pairs == 0 {
                return None;
            }
            let pieces = paired.iter().filter(|p| **p).count();
            let weight = match role {
                Role::Structure => rules.structure_weight,
                Role::Clutter => 1.0,
            };
            Some(Repeat {
                descriptor: rows[0].descriptor.clone(),
                role,
                pairs,
                pieces,
                score: pieces as f32 * weight,
            })
        })
        .collect();
    report.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.descriptor.cmp(&b.descriptor))
    });
    report
}

/// A descriptor and its patch, as one comparable key.
fn look_key(p: &Placed) -> String {
    match &p.patch {
        None => p.descriptor.clone(),
        Some(patch) => format!("{}{patch:?}", p.descriptor),
    }
}

fn look_alike(a: &Placed, b: &Placed, radius2: f32) -> bool {
    let (dx, dz) = (a.at.0 - b.at.0, a.at.1 - b.at.1);
    let turn = (a.yaw - b.yaw).rem_euclid(360.0);
    a.tip == b.tip
        && dx * dx + dz * dz < radius2
        && turn.min(360.0 - turn) < YAW_TOLERANCE_DEG
}
