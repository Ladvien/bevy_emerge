//! **A block offers options on request, each one different from the others** (#166, research lens
//! #10, narrowed: "show me three courthouses").
//!
//! `rules::options` asks the same block rules for several layouts and keeps only those that differ
//! from every option already kept in at least `min_differ` cells. Each option carries the seed that
//! made it, and `solve_block` with that seed rebuilds exactly that block, so choosing an option is
//! choosing its seed. Fewer options than asked is an answer: `too_close` and `failed` say where the
//! seeds went.

use bevy_emerge_core::descriptor::Descriptor;
use bevy_emerge_core::library::Library;
use bevy_emerge_core::map::{Map, Placed};
use bevy_emerge_core::rules::{
    Alphabet, BlockRules, Footprint, Options, PARTS, RULES_VERSION, Rect, Rules, Weight, learn,
    options, part_index, solve_block, validate,
};

const SIZE: usize = 15;

fn rules() -> BlockRules {
    BlockRules {
        example: "block".to_owned(),
        regions: vec![],
        cell: 2.0,
        size: (SIZE as u32, SIZE as u32),
        border: Some("t/street".to_owned()),
        no_empty: true,
        weights: vec![Weight { prefix: "t/house_".to_owned(), times: 100.0 }],
        footprints: vec![Footprint {
            prefix: "t/house_".to_owned(),
            sizes: vec![(2, 2), (2, 3), (3, 3)],
            yaws: vec![0.0, 180.0],
            front: Some("t/frontage".to_owned()),
            gap: 1,
            min_count: 0,
        }],
        attempts: 64,
        seed: 7,
    }
}

/// The library the example is drawn from; `learn` reads ids off the map, so this documents them.
fn library() -> Library {
    let mut descriptors: Vec<Descriptor> = ["t/street", "t/frontage", "t/yard"]
        .iter()
        .map(|id| Descriptor { id: (*id).to_owned(), ..Descriptor::default() })
        .collect();
    descriptors.extend(
        PARTS
            .iter()
            .map(|p| Descriptor { id: format!("t/house_{p}"), ..Descriptor::default() }),
    );
    Library { descriptors, ..Library::default() }
}

/// A street ring, a frontage ring inside it, yard, and whole houses of every allowed size on the
/// north row (turned 180) and the south row (turned 0). Cells are 2 m; the block is 15 by 15.
fn example() -> Map {
    let mut map = Map {
        name: "block".to_owned(),
        bounds: (SIZE as f32 * 2.0, 4.0, SIZE as f32 * 2.0),
        ..Map::default()
    };
    let mut cells: Vec<(String, f32)> = (0..SIZE * SIZE)
        .map(|i| {
            let (c, r) = (i % SIZE, i / SIZE);
            let ring = r == 0 || c == 0 || r == SIZE - 1 || c == SIZE - 1;
            let inner = r == 1 || c == 1 || r == SIZE - 2 || c == SIZE - 2;
            let d = if ring {
                "t/street"
            } else if inner {
                "t/frontage"
            } else {
                "t/yard"
            };
            (d.to_owned(), 0.0)
        })
        .collect();
    for (c0, r0, w, d, yaw) in [
        (3usize, 2usize, 2usize, 2usize, 180.0f32),
        (6, 2, 3, 3, 180.0),
        (10, 2, 2, 3, 180.0),
        (3, 11, 2, 2, 0.0),
        (7, 10, 3, 3, 0.0),
    ] {
        let rect = Rect { footprint: 0, c0, r0, w, d, yaw };
        for v in 0..d {
            for u in 0..w {
                let (c, r) = rect.cell(u, v);
                cells[r * SIZE + c] = (format!("t/house_{}", PARTS[part_index(u, v, w, d)]), yaw);
            }
        }
    }
    let half = SIZE as f32;
    for (i, (descriptor, yaw)) in cells.iter().enumerate() {
        let (c, r) = (i % SIZE, i / SIZE);
        map.placements.push(Placed {
            id: format!("p{i}"),
            descriptor: descriptor.clone(),
            at: (-half + (c as f32 + 0.5) * 2.0, -half + (r as f32 + 0.5) * 2.0),
            yaw: *yaw,
            ..Placed::default()
        });
    }
    map
}

fn alphabet() -> Alphabet {
    let rules = rules();
    Rules { version: RULES_VERSION, blocks: vec![rules.clone()] }
        .validate_against(&library())
        .unwrap_or_else(|e| panic!("{e}"));
    learn(&rules, &example()).unwrap_or_else(|e| panic!("{e}"))
}

fn differ(a: &[usize], b: &[usize]) -> usize {
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

fn ask(a: &Alphabet, pins: &[(usize, usize)], count: usize, min_differ: usize) -> Options {
    let rules = rules();
    options(a, &rules, pins, rules.seed, count, min_differ).unwrap_or_else(|e| panic!("{e}"))
}

/// **Acceptance 1.** Three options from the house block: every pair differs in at least
/// `min_differ` cells, each passes `validate`, each one's seed rebuilds it, and each says how far
/// its nearest other option is.
#[test]
fn three_options_differ_pairwise_and_each_seed_rebuilds_its_block() {
    let a = alphabet();
    let rules = rules();
    // Free layouts of this block differ in about 70 of its 169 inner cells, so 75 turns some
    // seeds away: the filter is exercised, not just passed.
    let min_differ = 75;
    let got = ask(&a, &[], 3, min_differ);
    assert_eq!(got.options.len(), 3, "three distinct layouts exist for this block: {got:?}");

    for (i, x) in got.options.iter().enumerate() {
        for y in &got.options[i + 1..] {
            let d = differ(&x.block.grid, &y.block.grid);
            assert!(
                d >= min_differ,
                "seeds {} and {} differ in {d} cells, under the {min_differ} asked",
                x.seed,
                y.seed
            );
        }
        assert_eq!(
            validate(&a, &rules, &x.block.grid).as_ref(),
            Some(&x.block.rects),
            "option {i} (seed {}) passes the block's own rules and names its footprints",
            x.seed
        );
        let rebuilt = solve_block(&a, &rules, &[], x.seed).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(rebuilt, x.block, "solve_block with option {i}'s seed {} rebuilds it", x.seed);

        let nearest = got
            .options
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, y)| differ(&x.block.grid, &y.block.grid))
            .min();
        assert_eq!(x.nearest, nearest, "option {i} names how far its nearest other option is");
    }
    let seeds: Vec<u64> = got.options.iter().map(|o| o.seed).collect();
    assert!(
        seeds.windows(2).all(|w| w[0] < w[1]),
        "options come in the order their seeds were tried: {seeds:?}"
    );
    assert_eq!(seeds[0], rules.seed, "seed 7's layout passes, so it is the first option");
    assert!(
        got.tried == 3 + got.too_close + got.failed,
        "every seed tried is kept, too close or failed: {got:?}"
    );
    assert!(got.too_close > 0, "some seeds' layouts were too close to a kept one: {got:?}");
}

/// **Acceptance 2.** The same arguments give the same options in the same order.
#[test]
fn options_are_deterministic() {
    let a = alphabet();
    let first = ask(&a, &[], 3, 75);
    let again = ask(&a, &[], 3, 75);
    assert_eq!(first, again);
}

/// **Acceptance 3.** With every free cell pinned there is one layout; asking for three gives it
/// once, and every other seed is reported as too close, none as failed.
#[test]
fn a_fully_pinned_block_offers_one_option_and_says_the_rest_were_too_close() {
    let a = alphabet();
    let rules = rules();
    let map = example();
    let pins: Vec<(usize, usize)> = map
        .placements
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let (c, r) = (i % SIZE, i / SIZE);
            !(r == 0 || c == 0 || r == SIZE - 1 || c == SIZE - 1)
        })
        .map(|(i, p)| {
            let proto = a
                .index_of(&p.descriptor, p.yaw)
                .unwrap_or_else(|| panic!("the example has {} at {}", p.descriptor, p.yaw));
            (i, proto)
        })
        .collect();
    let got = ask(&a, &pins, 3, 1);
    assert_eq!(got.options.len(), 1, "one layout is possible: {got:?}");
    assert_eq!(got.options[0].nearest, None, "a lone option has no nearest other");
    assert_eq!(
        solve_block(&a, &rules, &pins, got.options[0].seed).unwrap_or_else(|e| panic!("{e}")),
        got.options[0].block,
        "the option's seed rebuilds it under the same pins"
    );
    assert_eq!(got.failed, 0, "every seed collapses to the pinned layout");
    assert_eq!(
        got.tried,
        rules.attempts as usize * 3,
        "the whole budget is spent looking for a second layout"
    );
    assert_eq!(got.too_close, got.tried - 1, "every other seed repeated the first layout");

    // `min_differ` 0 still never keeps the same layout twice.
    let zero = ask(&a, &pins, 3, 0);
    assert_eq!(zero.options.len(), 1, "an identical layout is never a second option: {zero:?}");
}

/// **Acceptance 4.** No options asked, no seed spent; `min_differ` 1 keeps any new layout.
#[test]
fn zero_options_spend_nothing_and_min_differ_one_keeps_any_new_layout() {
    let a = alphabet();
    let none = ask(&a, &[], 0, 20);
    assert!(none.options.is_empty());
    assert_eq!((none.tried, none.too_close, none.failed), (0, 0, 0));

    let any = ask(&a, &[], 3, 1);
    assert_eq!(any.options.len(), 3, "{any:?}");
    assert_eq!(
        any.too_close, 0,
        "free layouts differ in tens of cells, so min_differ 1 turns none away: {any:?}"
    );
    for (i, x) in any.options.iter().enumerate() {
        for y in &any.options[i + 1..] {
            assert_ne!(x.block.grid, y.block.grid, "no two options are the same layout");
        }
    }
}
