//! #99 acceptance, the engine-free half: a stroke changes only the cells under it and is one undo
//! step, a piece rests on a slope and rides up when the ground is raised, a 16-bit map round-trips.

use bevy_grimoire_core::heightfield::{Brush, Heightfield, HeightmapError, Stroke};

fn field() -> Heightfield {
    Heightfield::flat((0.0, 0.0), 1.0, 21, 21, 0.0)
}

fn raise(at: (f32, f32), radius: f32) -> Stroke {
    Stroke { brush: Brush::Raise, radius_m: radius, strength: 2.0, points: vec![at] }
}

#[test]
fn a_raise_stroke_changes_only_cells_within_its_radius() {
    let mut f = field();
    let before = f.clone();
    let changed = f.apply(&raise((10.0, 10.0), 3.0));
    assert!(!changed.0.is_empty());
    for (i, (b, a)) in before.heights.iter().zip(&f.heights).enumerate() {
        let (ix, iz) = ((i % 21) as f32, (i / 21) as f32);
        let d = ((ix - 10.0).powi(2) + (iz - 10.0).powi(2)).sqrt();
        if d > 3.0 {
            assert_eq!(b, a, "cell ({ix},{iz}) is outside the radius");
        }
    }
    assert!((f.height_at(10.0, 10.0) - 2.0).abs() < 1e-5, "the centre rises by the strength");
    assert!(changed.0.len() == f.heights.iter().filter(|h| **h != 0.0).count());
}

#[test]
fn a_stroke_is_one_undo_step_and_redoes() {
    let mut f = field();
    let original = f.clone();
    let changed = f.apply(&Stroke {
        brush: Brush::Raise,
        radius_m: 4.0,
        strength: 1.0,
        points: vec![(5.0, 5.0), (8.0, 5.0), (11.0, 5.0)],
    });
    let after = f.clone();
    f.revert(&changed);
    assert_eq!(f, original);
    f.reapply(&changed);
    assert_eq!(f, after);
}

#[test]
fn flatten_converges_on_its_height_and_smooth_lowers_a_spike() {
    let mut f = field();
    f.apply(&raise((10.0, 10.0), 5.0));
    f.apply(&Stroke { brush: Brush::Flatten(1.0), radius_m: 20.0, strength: 1.0, points: vec![(10.0, 10.0)] });
    assert!((f.height_at(10.0, 10.0) - 1.0).abs() < 1e-5);

    let mut g = field();
    g.heights[10 * 21 + 10] = 5.0;
    g.apply(&Stroke { brush: Brush::Smooth, radius_m: 3.0, strength: 1.0, points: vec![(10.0, 10.0)] });
    assert!(g.heights[10 * 21 + 10] < 5.0);
}

#[test]
fn noise_is_deterministic_per_seed() {
    let stroke = |seed| Stroke { brush: Brush::Noise { seed, scale_m: 4.0 }, radius_m: 8.0, strength: 1.0, points: vec![(10.0, 10.0)] };
    let (mut a, mut b, mut c) = (field(), field(), field());
    a.apply(&stroke(1));
    b.apply(&stroke(1));
    c.apply(&stroke(2));
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn a_piece_rests_on_the_slope_and_rides_up_when_the_ground_is_raised() {
    let mut f = field();
    // A ramp rising 0.5 m per metre along x.
    for iz in 0..21 {
        for ix in 0..21 {
            f.heights[iz * 21 + ix] = ix as f32 * 0.5;
        }
    }
    let footprint = [(4.0, 4.0), (6.0, 4.0), (6.0, 6.0), (4.0, 6.0)];
    let rest = f.rest_height(&footprint);
    assert!((rest - 3.0).abs() < 1e-5, "highest corner is at x=6 -> 3 m, got {rest}");
    f.apply(&raise((5.0, 5.0), 3.0));
    assert!(f.rest_height(&footprint) > rest + 0.5, "the piece rides up with the hill");
}

#[test]
fn a_16_bit_height_map_round_trips() {
    let samples: Vec<u16> = (0..16u32).map(|i| (i * 4369) as u16).collect();
    let f = Heightfield::from_u16((0.0, 0.0), 2.0, 4, 4, &samples, (-10.0, 30.0)).unwrap();
    assert_eq!(f.range(), (-10.0, 30.0));
    assert_eq!(f.to_u16((-10.0, 30.0)).unwrap(), samples);
}

#[test]
fn a_bad_height_map_is_an_error() {
    assert_eq!(
        Heightfield::from_u16((0.0, 0.0), 1.0, 3, 3, &[0; 8], (0.0, 1.0)),
        Err(HeightmapError::Shape { len: 8, width: 3, height: 3 })
    );
    assert_eq!(Heightfield::from_u16((0.0, 0.0), 1.0, 2, 2, &[0; 4], (1.0, 1.0)), Err(HeightmapError::Range));
}
