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

/// The relations file that belongs to a map file: `town.map.ron` → `town.relations.ron`, in the
/// same directory. A map path that does not end `.map.ron` gets [`FILE_SUFFIX`] appended to its stem.
pub fn path_for_map(map_path: &std::path::Path) -> std::path::PathBuf {
    let name = map_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = name.strip_suffix(".map.ron").or_else(|| name.strip_suffix(".ron")).unwrap_or(&name);
    map_path.with_file_name(format!("{stem}{FILE_SUFFIX}"))
}

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
    /// Read a relations file, refusing a version this build does not read and a distance that is
    /// negative or not finite (a negative corridor would contain nothing, so a typo would read as a
    /// clear doorway).
    pub fn parse(text: &str) -> Result<Relations, String> {
        let r: Relations = ron::from_str(text).map_err(|e| format!("relations: parse: {e}"))?;
        if r.version != RELATIONS_VERSION {
            return Err(format!(
                "relations: version {} is not the {RELATIONS_VERSION} this build reads",
                r.version
            ));
        }
        for (i, rel) in r.relations.iter().enumerate() {
            // Faces has no distance; 0.0 stands in for "nothing to check".
            let metres: [f32; 2] = match &rel.kind {
                Kind::Faces(_) => [0.0, 0.0],
                Kind::Near { within, .. } | Kind::InFrontOf { within, .. } => [*within, 0.0],
                Kind::Apart { at_least, .. } => [*at_least, 0.0],
                Kind::ClearInFront { depth, width } => [*depth, *width],
                Kind::VisibleFrom { clearance, .. } => [*clearance, 0.0],
            };
            if let Some(bad) = metres.iter().find(|m| !(m.is_finite() && **m >= 0.0)) {
                return Err(format!(
                    "relations: relation {i} (of {:?}) has distance {bad}; distances are finite metres, 0 or more",
                    rel.of
                ));
            }
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
#[serde(deny_unknown_fields)]
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
    /// Seen from a placement: no third placement's centre stands within `clearance` metres of the
    /// line between the two centres (a sign seen from the gate, a door a guard watches). What rests
    /// on either end ([`Placed::on`], at any depth) is part of that end and never blocks. Measures
    /// how far off the line the nearest blocker stands, or `clearance` when clear.
    VisibleFrom { from: String, clearance: f32 },
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
        Kind::VisibleFrom { from, clearance } => match other(from) {
            Ok(b) => {
                let nearest = blockers(placements, b, a, *clearance)
                    .into_iter()
                    .min_by(|x, y| x.1.abs().total_cmp(&y.1.abs()));
                match nearest {
                    Some((p, side)) => Outcome::Unmet {
                        measured: side.abs(),
                        by: Some(p.id.clone()),
                    },
                    None => Outcome::Met { measured: *clearance },
                }
            }
            Err(m) => m,
        },
    }
}

/// Whether `p` rests on `end`, directly or through a chain of hosts. A chain is never longer than
/// the map, so an `on` cycle in a malformed file ends.
fn rests_on(placements: &[Placed], p: &Placed, end: &str) -> bool {
    let mut host = p.on.as_deref();
    for _ in 0..placements.len() {
        match host {
            Some(h) if h == end => return true,
            Some(h) => host = find(placements, h).ok().and_then(|q| q.on.as_deref()),
            None => return false,
        }
    }
    false
}

/// The piece `p` rests on at the bottom of its `on` chain, or `p` itself. A chain is never longer
/// than the map, so an `on` cycle in a malformed file ends.
fn bottom<'a>(placements: &'a [Placed], mut p: &'a Placed) -> &'a Placed {
    for _ in 0..placements.len() {
        let Some(host) = p.on.as_deref().and_then(|h| find(placements, h).ok()) else {
            break;
        };
        p = host;
    }
    p
}

/// A sight line: its start, unit direction and length.
type Line = ((f32, f32), (f32, f32), f32);

/// The line from `from` to `to`; `None` when the two stand on one spot.
fn line(from: &Placed, to: &Placed) -> Option<Line> {
    let length = distance(from.at, to.at);
    (length > 1e-4).then(|| {
        let u = ((to.at.0 - from.at.0) / length, (to.at.1 - from.at.1) / length);
        (from.at, u, length)
    })
}

/// `p` in a line's frame: (along it, sideways toward `(uz, -ux)`).
fn on_line(start: (f32, f32), (ux, uz): (f32, f32), p: (f32, f32)) -> (f32, f32) {
    let (dx, dz) = (p.0 - start.0, p.1 - start.1);
    (ux * dx + uz * dz, uz * dx - ux * dz)
}

/// Every placement within `clearance` of the line from `from` to `to` and strictly between its
/// ends, other than the ends and what rests on them, with its signed distance off the line.
fn blockers<'a>(placements: &'a [Placed], from: &Placed, to: &Placed, clearance: f32) -> Vec<(&'a Placed, f32)> {
    let Some((start, u, length)) = line(from, to) else {
        return Vec::new();
    };
    placements
        .iter()
        .filter(|p| p.id != from.id && p.id != to.id)
        .filter_map(|p| {
            let (along, side) = on_line(start, u, p.at);
            (along > 0.0 && along < length && side.abs() <= clearance).then_some((p, side))
        })
        .filter(|(p, _)| !rests_on(placements, p, &from.id) && !rests_on(placements, p, &to.id))
        .collect()
}

/// The moves that clear the sight line to `a` from `b`: each blocker's bottom host, once, slid
/// sideways just far enough that it and everything resting on it stand past `clearance`, to
/// whichever side needs the shorter slide. Riders keep their place on the host.
fn clear_sight(placements: &[Placed], a: &Placed, b: &Placed, clearance: f32) -> Vec<Move> {
    let Some((start, u, _)) = line(b, a) else {
        return Vec::new();
    };
    let past = clearance * 1.1 + 0.05;
    let mut hosts: Vec<&Placed> = Vec::new();
    for (p, _) in blockers(placements, b, a, clearance) {
        let host = bottom(placements, p);
        if !hosts.iter().any(|h| h.id == host.id) {
            hosts.push(host);
        }
    }
    hosts
        .into_iter()
        .map(|host| {
            let sides = placements
                .iter()
                .filter(|q| q.id == host.id || rests_on(placements, q, &host.id))
                .map(|q| on_line(start, u, q.at).1);
            let (low, high) = sides.fold((f32::INFINITY, f32::NEG_INFINITY), |(l, h), s| (l.min(s), h.max(s)));
            let (up, down) = (past - low, -past - high);
            let shift = if up.abs() <= down.abs() { up } else { down };
            // Sideways is (uz, -ux), as `on_line` measures.
            let to = (host.at.0 + u.1 * shift, host.at.1 - u.0 * shift);
            (host.id.clone(), to, host.yaw)
        })
        .collect()
}

/// One proposed move: the placement, where it goes, and its yaw there.
type Move = (String, (f32, f32), f32);

/// What [`solve`] did to a map's placements.
#[derive(Clone, Debug, PartialEq)]
pub struct Solved {
    /// Every placement, moved where the solver moved it; the rest unchanged and in input order.
    pub placements: Vec<Placed>,
    /// The ids the solver placed, first move first. A piece carried on a moved host is not listed.
    pub moved: Vec<String>,
    /// [`Outcome::Unmet`] relations before and after. Missing ones are neither and never moved for.
    pub unmet_before: usize,
    pub unmet_after: usize,
}

/// How many passes over the relations [`solve`] makes before it gives up on the rest.
const SOLVE_ROUNDS: usize = 8;

/// **Nudge the pieces of every broken relation until it holds**, greedily and never for the worse.
///
/// For each [`Outcome::Unmet`] relation the solver proposes one move: [`Kind::Faces`] turns the
/// piece to its target, [`Kind::Near`] and [`Kind::Apart`] step it along the line between the two,
/// [`Kind::InFrontOf`] sets it on the other's forward, and [`Kind::ClearInFront`] moves the intruder
/// sideways out of the corridor (the door stays where it was hung), and [`Kind::VisibleFrom`] moves
/// the nearest blocker sideways off the sight line (both ends stay). A move is kept only when it
/// lowers the map's unmet count, so a relation is never mended by breaking another, and
/// contradictory ones end with as many met as one move each allows.
///
/// The moved piece is the relation's subject, or for a corridor the intruder; a piece resting on
/// another ([`Placed::on`]) is not pulled off it — an intruder on a host moves its host — and
/// whatever rests on a moved piece goes with it, turned about it.
pub fn solve(placements: &[Placed], relations: &Relations) -> Solved {
    let unmet = |ps: &[Placed]| {
        check(ps, relations)
            .iter()
            .filter(|o| matches!(o, Outcome::Unmet { .. }))
            .count()
    };
    let mut current = placements.to_vec();
    let unmet_before = unmet(&current);
    let mut count = unmet_before;
    let mut moved: Vec<String> = Vec::new();
    for _ in 0..SOLVE_ROUNDS {
        let mut progressed = false;
        for r in &relations.relations {
            let Outcome::Unmet { by, .. } = check_one(&current, r) else {
                continue;
            };
            let moves = propose(&current, r, by.as_deref());
            if moves.is_empty() {
                continue;
            }
            let mut trial = current.clone();
            let depth = trial.len();
            for (id, at, yaw) in &moves {
                move_piece(&mut trial, id, *at, *yaw, depth);
            }
            let trial_count = unmet(&trial);
            if trial_count < count {
                current = trial;
                count = trial_count;
                progressed = true;
                for (id, ..) in moves {
                    if !moved.contains(&id) {
                        moved.push(id);
                    }
                }
            }
        }
        if !progressed || count == 0 {
            break;
        }
    }
    Solved {
        placements: current,
        moved,
        unmet_before,
        unmet_after: count,
    }
}

/// The moves that would mend `r`: which pieces, and where they go. Empty when the piece to move
/// rests on another, or the relation names a piece the map lacks. Every kind moves one piece but
/// a sight line, which clears all its blockers at once (one at a time could never lower the count).
fn propose(placements: &[Placed], r: &Relation, by: Option<&str>) -> Vec<Move> {
    let Ok(a) = find(placements, &r.of) else {
        return Vec::new();
    };
    let other = |id: &str| find(placements, id).ok();
    let free = |p: &Placed| p.on.is_none();
    // `toward` from `from`, unit length; `fallback` when the two stand on one spot.
    let unit = |from: (f32, f32), to: (f32, f32), fallback: (f32, f32)| {
        let d = distance(from, to);
        if d > 1e-4 {
            ((to.0 - from.0) / d, (to.1 - from.1) / d)
        } else {
            fallback
        }
    };
    let forward = |p: &Placed| (p.yaw.sin(), p.yaw.cos());
    if let Kind::VisibleFrom { from, clearance } = &r.kind {
        return other(from).map_or_else(Vec::new, |b| clear_sight(placements, a, b, *clearance));
    }
    let one = (|| match &r.kind {
        Kind::Faces(to) => {
            let b = other(to)?;
            let (dx, dz) = (b.at.0 - a.at.0, b.at.1 - a.at.1);
            free(a).then(|| (a.id.clone(), a.at, dx.atan2(dz)))
        }
        Kind::Near { to, within } => {
            let b = other(to)?;
            let (ux, uz) = unit(b.at, a.at, forward(b));
            let d = within * 0.9;
            free(a).then(|| (a.id.clone(), (b.at.0 + ux * d, b.at.1 + uz * d), a.yaw))
        }
        Kind::Apart { from, at_least } => {
            let b = other(from)?;
            let back = forward(a);
            let (ux, uz) = unit(b.at, a.at, (-back.0, -back.1));
            let d = at_least * 1.1 + 0.05;
            free(a).then(|| (a.id.clone(), (b.at.0 + ux * d, b.at.1 + uz * d), a.yaw))
        }
        Kind::InFrontOf { of, within } => {
            let b = other(of)?;
            let (fx, fz) = forward(b);
            let d = distance(a.at, b.at).clamp(within * 0.1, within * 0.9);
            free(a).then(|| (a.id.clone(), (b.at.0 + fx * d, b.at.1 + fz * d), a.yaw))
        }
        Kind::ClearInFront { width, .. } => {
            // The intruder, or the piece it rests on: moving the host carries it out.
            let intruder = bottom(placements, other(by?)?);
            let (ahead, side) = local(a, intruder.at);
            let sign = if side < 0.0 { -1.0 } else { 1.0 };
            let side = sign * (width / 2.0 * 1.1 + 0.05);
            // Back from `a`'s frame: forward (fx, fz), sideways (fz, -fx), as `local` measures.
            let (fx, fz) = forward(a);
            let to = (
                a.at.0 + fx * ahead + fz * side,
                a.at.1 + fz * ahead - fx * side,
            );
            Some((intruder.id.clone(), to, intruder.yaw))
        }
        Kind::VisibleFrom { .. } => None,
    })();
    one.into_iter().collect()
}

/// Put `id` at `at` turned to `yaw`, carrying everything resting on it (and on that, and so on):
/// riders keep their offset in the host's frame, so they turn with it.
/// `depth` bounds the walk, so an `on` cycle in a malformed file ends.
fn move_piece(placements: &mut [Placed], id: &str, at: (f32, f32), yaw: f32, depth: usize) {
    if depth == 0 {
        return;
    }
    let Some(host) = placements.iter().position(|p| p.id == id) else {
        return;
    };
    let (from, turn) = (placements[host].at, yaw - placements[host].yaw);
    let (s, c) = turn.sin_cos();
    placements[host].at = at;
    placements[host].yaw = yaw;
    let riders: Vec<(String, (f32, f32), f32)> = placements
        .iter()
        .filter(|p| p.on.as_deref() == Some(id))
        .map(|p| {
            let (dx, dz) = (p.at.0 - from.0, p.at.1 - from.1);
            // Turning by `turn` about +y in this forward convention: (sin, cos) of yaw + turn.
            let (rx, rz) = (dx * c + dz * s, -dx * s + dz * c);
            (p.id.clone(), (at.0 + rx, at.1 + rz), p.yaw + turn)
        })
        .collect();
    for (rider, to, rider_yaw) in riders {
        move_piece(placements, &rider, to, rider_yaw, depth - 1);
    }
}
