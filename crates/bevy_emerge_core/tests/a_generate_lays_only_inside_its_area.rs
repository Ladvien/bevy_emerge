//! **A generate lays only inside its area, and joins the map around it** (#192, research lens #6:
//! graybox first, a generator that runs only inside the request's region).
//!
//! The fixture is a 6 × 6 checkerboard of two pieces, `a` and `b`, at one-metre cells. Learned from
//! the whole board, the grammar lets `a` stand only beside `b` and `b` only beside `a`, so the two
//! colourings of any hole are both legal in themselves: only the cells around the hole say which
//! one joins the board. A solve that reads its neighbours lays the board's own colouring every time.

use bevy_emerge_core::grammar::{self, Area};
use bevy_emerge_core::map::{Map, Placed};

/// The board's piece at cell `(x, z)`, as the checkerboard colours it.
fn colour(x: usize, z: usize) -> &'static str {
    if (x + z).is_multiple_of(2) { "a" } else { "b" }
}

/// The centre of cell `(x, z)` on a 6 m map whose floor runs from −3 to 3.
fn centre(x: usize, z: usize) -> (f32, f32) {
    (x as f32 - 2.5, z as f32 - 2.5)
}

/// The whole board, less the cells `hole` names.
fn board(hole: impl Fn(usize, usize) -> bool) -> Map {
    let mut m = Map {
        name: "board".into(),
        bounds: (6.0, 3.0, 6.0),
        ..Map::default()
    };
    for z in 0..6 {
        for x in 0..6 {
            if hole(x, z) {
                continue;
            }
            m.placements.push(Placed {
                id: format!("p{x}_{z}"),
                descriptor: colour(x, z).to_owned(),
                at: centre(x, z),
                ..Placed::default()
            });
        }
    }
    m
}

/// Cells 1..4 on both axes: a 3 × 3 hole one cell in from the north-west corner.
fn in_hole(x: usize, z: usize) -> bool {
    (1..4).contains(&x) && (1..4).contains(&z)
}

/// The area over that hole, corners in the opposite order to show either order reads.
const HOLE: Area = Area {
    from: (0.5, 0.5),
    to: (-1.5, -1.5),
};

fn ids() -> impl FnMut() -> String {
    let mut n = 0;
    move || {
        n += 1;
        format!("gen@{n}")
    }
}

#[test]
fn a_solve_in_an_area_lays_its_cells_and_no_other() {
    let grammar = grammar::learn(&board(|_, _| false), 1.0).unwrap_or_else(|e| panic!("{e}"));
    let holed = board(in_hole);
    let solved =
        grammar::solve_in(&holed, &grammar, 1.0, HOLE, 7, ids()).unwrap_or_else(|e| panic!("{e}"));

    let inside: Vec<(f32, f32)> = (1..4)
        .flat_map(|z| (1..4).map(move |x| centre(x, z)))
        .collect();
    let laid: Vec<(f32, f32)> = solved.placements.iter().map(|p| p.at).collect();
    for p in &solved.placements {
        assert!(
            inside.contains(&p.at),
            "`{}` was laid at {:?}, outside the area {HOLE:?}: a solve in an area must leave the \
             rest of the map as it is",
            p.id,
            p.at
        );
    }
    for at in &inside {
        assert!(
            laid.contains(at),
            "the cell at {at:?} is in the area but nothing was laid there; laid: {laid:?}"
        );
    }
    assert_eq!(
        solved.free_cells, 9,
        "the area is nine cells, and all of them were free"
    );
    assert_eq!(
        (solved.width, solved.height),
        (5, 5),
        "the grid is the area and the ring around it it must join, not the whole 6 x 6 map"
    );
}

#[test]
fn the_area_joins_the_pieces_around_it() {
    let grammar = grammar::learn(&board(|_, _| false), 1.0).unwrap_or_else(|e| panic!("{e}"));
    let holed = board(in_hole);
    // Either colouring is legal inside the hole on its own, so a solve that ignored its neighbours
    // would lay the wrong one for about half these seeds.
    for seed in 0..16 {
        let solved = grammar::solve_in(&holed, &grammar, 1.0, HOLE, seed, ids())
            .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        for p in &solved.placements {
            let (x, z) = (
                (p.at.0 + 2.5).round() as usize,
                (p.at.1 + 2.5).round() as usize,
            );
            assert_eq!(
                p.descriptor,
                colour(x, z),
                "seed {seed}: the cell at {:?} got `{}`, which cannot stand beside the board's \
                 own pieces around the area",
                p.at,
                p.descriptor
            );
        }
    }
}

#[test]
fn a_neighbour_the_grammar_does_not_know_leaves_the_edge_open() {
    let grammar = grammar::learn(&board(|_, _| false), 1.0).unwrap_or_else(|e| panic!("{e}"));
    let mut holed = board(in_hole);
    // The ring cell west of the hole's middle row holds a piece the board never taught.
    let west = holed
        .placements
        .iter_mut()
        .find(|p| p.at == centre(0, 2))
        .unwrap_or_else(|| panic!("the board has a piece at cell (0, 2)"));
    west.descriptor = "stranger".to_owned();

    let solved = grammar::solve_in(&holed, &grammar, 1.0, HOLE, 3, ids())
        .unwrap_or_else(|e| panic!("an unknown neighbour must not refuse the area: {e}"));
    assert_eq!(
        solved.placements.len(),
        9,
        "every cell of the area is still laid"
    );
}

#[test]
fn a_corner_of_the_ring_touches_nothing_it_lays() {
    let grammar = grammar::learn(&board(|_, _| false), 1.0).unwrap_or_else(|e| panic!("{e}"));
    let mut holed = board(in_hole);
    // Cell (0, 0) is the ring's north-west corner. Turned to `b`, it stands beside two other `b`s,
    // a pair the board never taught, but it touches no cell of the area.
    let corner = holed
        .placements
        .iter_mut()
        .find(|p| p.at == centre(0, 0))
        .unwrap_or_else(|| panic!("the board has a piece at cell (0, 0)"));
    corner.descriptor = "b".to_owned();

    let solved = grammar::solve_in(&holed, &grammar, 1.0, HOLE, 5, ids()).unwrap_or_else(|e| {
        panic!("two old pieces at the ring's corner must not refuse the area between them: {e}")
    });
    assert_eq!(solved.placements.len(), 9, "every cell of the area is laid");
}

#[test]
fn an_area_off_the_map_is_refused_naming_it() {
    let grammar = grammar::learn(&board(|_, _| false), 1.0).unwrap_or_else(|e| panic!("{e}"));
    let off = Area {
        from: (10.0, 10.0),
        to: (12.0, 12.0),
    };
    let err = grammar::solve_in(&board(in_hole), &grammar, 1.0, off, 1, ids())
        .expect_err("an area with no cell of the map has nothing to lay");
    assert!(
        err.contains("holds no cell") && err.contains("(10.0, 10.0)"),
        "the refusal must say the area misses the map, naming it: {err}"
    );
}
