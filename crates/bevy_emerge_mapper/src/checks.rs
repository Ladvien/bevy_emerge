//! **What a change broke (E-074).** Run after every operation that changes a document, so the
//! answer an agent reads says what its change introduced, not only that it applied.
//!
//! The brief asks for this in as many words (`emerge-feature-loop.md`:201): *"Relevant validators run
//! after every mutation. Their failures appear in the tool result."* Until now an agent's
//! `emerge_apply_operation` answer said `"applied"` and nothing else, so a `placement_set_yaw` that
//! opened an edge seam on the far side of a 720-placement map read exactly like one that changed
//! nothing but the facing.
//!
//! # Only two kinds of finding, and that is the scope
//!
//! **Edge faults** — two abutting tiles whose declared face tokens disagree
//! ([`bevy_emerge_core::adjacency`]). **Overlaps** — two placements whose plans interpenetrate on the
//! same layer ([`bevy_emerge_core::stack::overlaps`]). Both were already computable; neither was ever
//! asked on the way out of a mutation.
//!
//! Two more the scope names are *not* here, and each is filed as its own item rather than half-built:
//!
//! - **A piece standing in a doorway.** [`bevy_emerge_core::stack::blocking`]'s own doc bounds the
//!   claim to *within* a layer, so nothing compares a crate against an opening's clearance (E-079).
//! - **How varied a layout is.** `range::measure` needs a solver grid and the editor discards it
//!   on the way back from `grammar::learn` (E-080).
//!
//! # What "new" means, and why the diff is the report
//!
//! A finding that was there before is not what this change broke. An agent that reads twenty
//! standing faults every time it moves a crate learns to ignore the field, which is the same as not
//! reporting them. So the answer names the **difference** and states the total beside it, and the
//! reader can tell "I introduced one" from "this map has twenty and I made none".
//!
//! The comparison is on a finding's **identity**, not on its sentence: an edge fault is
//! `(a, b, dir)`, an overlap is its pair of placement ids. A message that gained a word is not a new
//! finding, and an overlap whose two placements were reordered in the file is the same overlap.

use bevy_emerge_core::adjacency::{self, Fault};
use serde_json::{Value, json};

/// How many new findings of each kind an answer names. The totals are always exact.
///
/// There is a cap because an operation that replaces a whole map — `map_open`, `layout_accept` on a
/// freshly generated layout — can produce hundreds of findings at once, and an answer carrying all of
/// them is a tool result no agent can read. [`NEW_CAP`] is what a person can take in before deciding
/// the change was wrong for a reason the list already shows.
pub const NEW_CAP: usize = 20;

/// The map's findings at one moment.
pub struct Found {
    /// **Which map these are findings about**, as its file path.
    ///
    /// A `before` and an `after` are only comparable when they are the same map. Two maps from one
    /// kit both hold `wall@0` and `pane@1`, so comparing across a `map_open` would match a fault in
    /// the map that was closed against an unrelated one in the map that was opened and call the
    /// second *old* — a silent under-report, which is the one failure mode a checks field cannot
    /// afford. See [`report`].
    pub map: std::path::PathBuf,
    /// Every place the map disagrees with its tiles' declared edge tokens, in the pass's own order.
    pub faults: Vec<Fault>,
    /// Pairs of placement ids, each pair sorted (smaller id first) so a reorder is not a change.
    pub overlaps: Vec<(String, String)>,
}

/// Whether `operation` gets checks: every operation that changes a document (`Undo` is not `None`),
/// plus `map_open`, which replaces the whole map and declares `Undo::None`.
///
/// # The registry is the list, not a second one
///
/// "Which operations change a document" is already answered by [`crate::ops::Operation::undo`], and a
/// second list here would be the kind of two-answer problem this crate keeps meeting. An operation
/// that grows an inverse is checked the day it grows one, with nothing to remember.
///
/// **`map_open` is the exception and it has to be named.** It declares no inverse — the map it loaded
/// had no counterpart in the map it replaced — and it is exactly the operation whose findings a reader
/// most needs, because it is the one that brings a whole file's worth onto the screen. The name is a
/// literal rather than a lookup because the exception is the point: an operation that gains
/// `Undo::Inverse` is checked by the rule, and this line is the one operation the rule cannot cover.
pub fn wanted(operation: &str) -> bool {
    crate::ops::find(operation).is_some_and(|o| o.undo() != crate::ops::Undo::None)
        || operation == "map_open"
}

/// The findings for the map in `ctx`, or `None` when `ctx.project` or `ctx.open` is `None`.
///
/// Both are needed and neither is optional: a check compares placements against the library that
/// gives them their extents and tokens, and there is no map on the menu screen. The menu is not a
/// failure — `map_create` runs there and changes no map — so this answers `None` rather than
/// refusing.
pub fn find(ctx: &crate::ops::Ctx) -> Option<Found> {
    let project = ctx.project.as_ref()?;
    let open = ctx.open.as_ref()?;
    let faults = adjacency::faults(&open.map, &project.library, project.lattice.face_bands);
    let overlaps = bevy_emerge_core::stack::overlaps(&open.map, &project.library)
        .into_iter()
        .map(|(i, j)| {
            let a = open.map.placements[i].id.clone();
            let b = open.map.placements[j].id.clone();
            // **Sorted by id, not by index**, so the identity of an overlap survives the two
            // placements changing order in the file — which a save, a patch or a generator's output
            // can do without the map meaning anything different.
            if a <= b { (a, b) } else { (b, a) }
        })
        .collect();
    Some(Found {
        map: open.map_path.clone(),
        faults,
        overlaps,
    })
}

/// The `checks` value of an answer: what `after` has that `before` did not.
///
/// `before == None` means everything in `after` is new. That is `map_open` from the menu, where the
/// map that was open before does not exist — and it is also the honest answer for a caller that
/// could not be asked (no project loaded), because the finding is new to the answer either way.
///
/// **A `before` from a different map is discarded, not compared.** `map_open` hands this the findings
/// of the map it is about to replace, and findings are matched by placement **id** — which two maps
/// from one kit share. So every finding in the newly opened map would be checked against an unrelated
/// piece that happened to have the same id, and a real fault would be reported as nothing new. A
/// different map means nothing in this one is old.
pub fn report(before: Option<&Found>, after: &Found) -> Value {
    let before = before.filter(|b| b.map == after.map);
    let is_new_fault = |f: &Fault| {
        !before.is_some_and(|b| {
            b.faults
                .iter()
                .any(|o| o.a == f.a && o.b == f.b && o.dir == f.dir)
        })
    };
    let is_new_overlap = |o: &(String, String)| {
        !before.is_some_and(|b| b.overlaps.iter().any(|p| p == o))
    };

    let new_faults: Vec<&Fault> = after.faults.iter().filter(|f| is_new_fault(f)).collect();
    let new_overlaps: Vec<&(String, String)> = after
        .overlaps
        .iter()
        .filter(|o| is_new_overlap(o))
        .collect();

    json!({
        "edge_faults": {
            "new": new_faults
                .iter()
                .take(NEW_CAP)
                .map(|f| json!({
                    "a": f.a,
                    "b": f.b,
                    "dir": adjacency::dir_name(f.dir),
                    "message": f.message,
                }))
                .collect::<Vec<_>>(),
            "new_total": new_faults.len(),
            "total": after.faults.len(),
        },
        "overlaps": {
            "new": new_overlaps
                .iter()
                .take(NEW_CAP)
                .map(|(a, b)| json!({"a": a, "b": b}))
                .collect::<Vec<_>>(),
            "new_total": new_overlaps.len(),
            "total": after.overlaps.len(),
        },
    })
}

#[cfg(test)]
mod tests {
    use bevy_emerge_core::wfc::E;

    use super::*;

    /// **The registry decides, and `map_open` is the one exception.** Pin both halves: the rule (an
    /// operation with an inverse is checked, a read-only one is not) and the named exception, so a
    /// `map_open` that was quietly dropped from the list fails here.
    #[test]
    fn wanted_follows_the_registry() {
        assert!(wanted("placement_add"), "a placement changes the map");
        assert!(wanted("map_open"), "opening a map replaces it whole");
        assert!(!wanted("brush_arm"), "arming a brush changes nothing");
        assert!(!wanted("layout_propose"), "a proposal is not map data");
        assert!(!wanted("no_such_op"), "an unknown name has no undo to ask");
    }

    /// **A finding already there is not new, and the total still counts it.** The two numbers answer
    /// two different questions — "did I break this" and "how bad is this map" — and a change that
    /// touched neither must leave both alone.
    #[test]
    fn a_finding_already_there_is_not_new() {
        let fault = |b: &str, dir| Fault {
            a: "wall@0".into(),
            b: b.into(),
            dir,
            a_face: vec![None],
            b_face: vec![None],
            message: format!("wall@0 and {b} disagree"),
        };
        let before = Found {
            map: "m.map.ron".into(),
            faults: vec![fault("pane@1", E)],
            overlaps: vec![("a".into(), "b".into())],
        };
        let after = Found {
            map: "m.map.ron".into(),
            faults: vec![fault("pane@1", E)],
            overlaps: vec![("a".into(), "b".into()), ("c".into(), "d".into())],
        };
        let r = report(Some(&before), &after);
        assert_eq!(r["overlaps"]["new_total"], 1);
        assert_eq!(r["overlaps"]["total"], 2);
        assert_eq!(r["overlaps"]["new"][0], json!({"a": "c", "b": "d"}));
        assert_eq!(r["edge_faults"]["new_total"], 0, "the standing fault is not new");
        assert_eq!(r["edge_faults"]["total"], 1);
    }

    /// **A `before` from another map is not compared.** `map_open` takes its `before` from the map it
    /// is about to replace, and two maps from one kit share their placement ids — so comparing them
    /// would match a fault in the closed map against an unrelated piece in the opened one and call a
    /// real fault *old*. The one failure mode a checks field cannot afford is a silent under-report.
    #[test]
    fn a_before_from_another_map_is_not_compared() {
        let fault = |b: &str| Fault {
            a: "wall@0".into(),
            b: b.into(),
            dir: E,
            a_face: vec![None],
            b_face: vec![None],
            message: String::new(),
        };
        let closed = Found {
            map: "closed.map.ron".into(),
            faults: vec![fault("pane@1")],
            overlaps: vec![("a".into(), "b".into())],
        };
        // The same ids, a different piece of the project: this map's fault is new to this answer.
        let opened = Found {
            map: "opened.map.ron".into(),
            faults: vec![fault("pane@1")],
            overlaps: vec![("a".into(), "b".into())],
        };
        let r = report(Some(&closed), &opened);
        assert_eq!(
            r["edge_faults"]["new_total"], 1,
            "the opened map's fault is its own, not the closed map's"
        );
        assert_eq!(r["overlaps"]["new_total"], 1);
        // And the same map still diffs, which is the case above; this only pins the discard.
        let r = report(Some(&opened), &opened);
        assert_eq!(r["edge_faults"]["new_total"], 0);
    }
}
