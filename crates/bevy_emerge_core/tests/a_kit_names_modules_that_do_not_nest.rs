//! **Kit standards: the sub-kit modules that do not nest** (#168, lens #4).
//!
//! Shipped modular kits put every sub-kit on one grid: a sub-kit's module divides the base module
//! or is a whole multiple of it, so runs of each meet on a grid line every module. `standards::check`
//! holds each piece to its own rule's `multiple_of`, so a 2 m wall kit and a 1.5 m fence kit both
//! pass while the two share a grid line only every 6 m. `standards::unnested` names those pairs,
//! with where they meet again.

use bevy_emerge_core::standards::{Dim, MEET_WITHIN, Rule, STANDARDS_VERSION, Standards, Unnested, unnested};

fn module(name: &str, dim: Dim, m: f32) -> Rule {
    Rule {
        name: name.to_owned(),
        kind: None,
        dim,
        min: None,
        max: None,
        multiple_of: Some(m),
    }
}

fn of(rules: Vec<Rule>) -> Standards {
    Standards {
        version: STANDARDS_VERSION,
        rules,
    }
}

fn names(found: &[Unnested]) -> Vec<[&str; 2]> {
    found
        .iter()
        .map(|u| [u.rules[0].as_str(), u.rules[1].as_str()])
        .collect()
}

#[test]
fn two_sub_kit_modules_that_do_not_nest_are_named_with_where_they_meet() {
    let s = of(vec![
        module("wall module", Dim::Width, 2.0),
        module("fence module", Dim::Width, 1.5),
    ]);
    let found = unnested(&s);
    assert_eq!(found.len(), 1, "{found:?}");
    let u = &found[0];
    assert_eq!(u.rules, ["wall module".to_owned(), "fence module".to_owned()]);
    assert_eq!(u.dims, [Dim::Width, Dim::Width]);
    assert_eq!(u.modules, [2.0, 1.5]);
    // Three walls and four fences: 6 m is the first run both land on.
    let meet = u.meet.expect("2 and 1.5 meet");
    assert!((meet - 6.0).abs() < 1e-4, "{meet}");
}

#[test]
fn a_module_that_divides_another_or_is_a_whole_multiple_of_it_nests() {
    let s = of(vec![
        module("floor module", Dim::Width, 2.0),
        module("trim module", Dim::Width, 0.5),
        module("hall module", Dim::Depth, 4.0),
        module("same module", Dim::Width, 2.0),
    ]);
    assert_eq!(unnested(&s), Vec::new());
}

#[test]
fn a_quarter_turn_puts_a_width_along_a_depth_so_the_two_are_compared() {
    let s = of(vec![
        module("wall module", Dim::Width, 2.0),
        module("rail module", Dim::Depth, 1.5),
    ]);
    let found = unnested(&s);
    assert_eq!(names(&found), vec![["wall module", "rail module"]]);
    assert_eq!(found[0].dims, [Dim::Width, Dim::Depth]);
}

#[test]
fn height_is_its_own_axis_and_never_pairs_with_a_footprint() {
    // 2.4 m is no multiple of a 2 m footprint module, and nobody stacks a width on a height.
    let across = of(vec![
        module("wall module", Dim::Width, 2.0),
        module("storey", Dim::Height, 2.4),
    ]);
    assert_eq!(unnested(&across), Vec::new());

    // A 0.25 m stair rise does not climb a 2.4 m storey in whole steps: they meet at 12 m.
    let up = of(vec![
        module("storey", Dim::Height, 2.4),
        module("stair rise", Dim::Height, 0.25),
    ]);
    let found = unnested(&up);
    assert_eq!(names(&found), vec![["storey", "stair rise"]]);
    let meet = found[0].meet.expect("2.4 and 0.25 meet");
    assert!((meet - 12.0).abs() < 1e-3, "{meet}");
}

#[test]
fn modules_within_the_multiple_tolerance_nest() {
    // Exported meshes are rarely exact; the same centimetre `check` allows is allowed here.
    let s = of(vec![
        module("wall module", Dim::Width, 2.0),
        module("wall module as exported", Dim::Width, 2.005),
        module("double module", Dim::Depth, 4.008),
    ]);
    assert_eq!(unnested(&s), Vec::new());
}

#[test]
fn modules_that_never_meet_within_the_limit_say_so() {
    // A 1.215 m module against 2 m comes nearest a shared line at 96 m, and even there it is 1.5 cm
    // off, past the 1 cm tolerance.
    let s = of(vec![
        module("wall module", Dim::Width, 2.0),
        module("odd module", Dim::Width, 1.215),
    ]);
    let found = unnested(&s);
    assert_eq!(names(&found), vec![["wall module", "odd module"]]);
    assert_eq!(found[0].meet, None, "not within {MEET_WITHIN} of the larger module");
}

#[test]
fn rules_without_a_module_are_not_compared_and_pairs_come_in_file_order() {
    let mut passage = module("door passage", Dim::Width, 1.0);
    passage.multiple_of = None;
    passage.min = Some(1.0);
    let s = of(vec![
        module("a", Dim::Width, 2.0),
        passage,
        module("b", Dim::Width, 1.5),
        module("c", Dim::Depth, 1.3),
    ]);
    assert_eq!(
        names(&unnested(&s)),
        vec![["a", "b"], ["a", "c"], ["b", "c"]],
        "every unnested pair once, earlier rule first, in file order"
    );
}
