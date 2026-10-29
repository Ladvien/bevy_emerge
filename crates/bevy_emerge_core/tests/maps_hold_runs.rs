//! **A map holds a run, not the rows it lays** (#115, slice 1: data, `expanded()`, validation, version).
//!
//! A fence of hundreds of posts is one line. The map stores the run (descriptor, path, pitch, gaps)
//! and `Map::expanded` lays it, so moving an end is one edit and the rows follow.

use bevy_emerge_core::descriptor::{Descriptor, Extent};
use bevy_emerge_core::library::{LIBRARY_VERSION, Library};
use bevy_emerge_core::map::{MAP_VERSION, Map, Placed};
use bevy_emerge_core::run::{Run, RunPlaced};

const W: f32 = 2.064;

fn library() -> Library {
    Library {
        version: LIBRARY_VERSION,
        note: None,
        descriptors: vec![Descriptor {
            id: "kit/fence".to_owned(),
            extent: Extent { footprint: Some((W, 0.12)), height: Some(0.838) },
            ..Default::default()
        }],
    }
}

fn run(to: f32) -> Run {
    Run {
        descriptor: "kit/fence".to_owned(),
        pitch: None,
        from: (0.0, 0.0),
        to: (to, 0.0),
        corners: Vec::new(),
        gaps: Vec::new(),
    }
}

fn map_with(runs: Vec<RunPlaced>) -> Map {
    Map {
        name: "yard".to_owned(),
        bounds: (100.0, 10.0, 100.0),
        runs,
        ..Map::default()
    }
}

fn placed(id: &str, run: Run) -> RunPlaced {
    RunPlaced { id: id.to_owned(), run, owned: false, owned_because: None }
}

fn ron_of(map: &Map) -> String {
    ron::ser::to_string_pretty(map, ron::ser::PrettyConfig::default()).unwrap()
}

/// Criterion 1: a run of ten posts round-trips and expands to ten rows `<id>/0..9` at the pitch.
#[test]
fn a_run_round_trips_and_expands_to_its_rows() {
    let map = map_with(vec![placed("ring", run(19.0))]);
    let back = Map::parse(&ron_of(&map)).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, map);
    assert!(back.placements.is_empty(), "the rows are never stored: {:?}", back.placements);

    let out = back.expanded(&[], &library()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out.placements.len(), 10);
    for (n, row) in out.placements.iter().enumerate() {
        assert_eq!(row.id, format!("ring/{n}"));
        assert_eq!(row.descriptor, "kit/fence");
        assert!((row.at.0 - W * n as f32).abs() < 1e-4, "{row:?}");
    }
    assert_eq!(out.runs, back.runs, "the reference survives the expansion");
}

/// An owned run lays owned rows; a hand row beside it is untouched.
#[test]
fn rows_are_owned_like_their_run() {
    let mut r = placed("ring", run(5.0));
    r.owned = true;
    r.owned_because = Some("the graveyard ring".to_owned());
    let mut map = map_with(vec![r]);
    map.placements.push(Placed {
        id: "well".to_owned(),
        descriptor: "kit/fence".to_owned(),
        ..Placed::default()
    });
    let out = map.expanded(&[], &library()).unwrap();
    let rows: Vec<_> = out.placements.iter().filter(|p| p.id.starts_with("ring/")).collect();
    assert!(!rows.is_empty() && rows.iter().all(|p| p.owned && p.owned_because.is_some()));
    assert!(!out.placements.iter().find(|p| p.id == "well").unwrap().owned);
}

/// Edge cases: a missing piece refuses naming the run; an id colliding with a placement refuses.
#[test]
fn a_run_that_cannot_lay_or_collides_refuses() {
    let mut bad = run(10.0);
    bad.descriptor = "kit/nothing".to_owned();
    let err = map_with(vec![placed("ring", bad)])
        .expanded(&[], &library())
        .expect_err("a missing piece");
    assert!(err.contains("`ring`") && err.contains("kit/nothing"), "{err}");

    let mut m = map_with(vec![placed("fence", run(10.0))]);
    m.placements.push(Placed { id: "fence".to_owned(), descriptor: "kit/fence".to_owned(), ..Placed::default() });
    let err = m.validate().expect_err("an id shared with a placement");
    assert!(err.contains("run `fence`"), "{err}");

    let m = map_with(vec![placed("a", run(10.0)), placed("a", run(10.0))]);
    assert!(m.validate().expect_err("twice").contains("used twice"));
}

/// Criterion 4: a version-7 map (no `runs`) loads and re-saves at the current version, with nothing
/// written for the empty list; a file numbered above this build is refused.
#[test]
fn an_older_map_loads_and_a_newer_one_is_refused() {
    assert_eq!(MAP_VERSION, 8);
    let current = ron_of(&map_with(Vec::new()));
    assert!(!current.contains("runs"), "an empty list is not written: {current}");
    let old = current.replace(&format!("version: {MAP_VERSION}"), "version: 7");
    assert!(old.contains("version: 7"), "{old}");
    let loaded = Map::parse(&old).unwrap_or_else(|e| panic!("a v7 map must load: {e}"));
    assert!(loaded.runs.is_empty());

    let with = ron_of(&map_with(vec![placed("ring", run(19.0))]));
    assert!(with.contains("runs"), "{with}");
    let newer = with.replace(&format!("version: {MAP_VERSION}"), &format!("version: {}", MAP_VERSION + 1));
    let err = Map::parse(&newer).expect_err("a newer map");
    assert!(err.contains("newer tool"), "{err}");
}
