//! **A guest `on` a host lands on the host's lid** — issue #82.
//!
//! `datum` adds the host's `align.y_offset` to its Y, and that offset is what seats the mesh's base on
//! the ground. The lid is therefore `placed_height` above the *ground*, and lifting a guest by
//! `placed_height` from the shifted origin counted the offset twice: a guest on `crate_wooden`
//! (`y_offset` 0.052) hovered 5.2 cm high and z-fought the lid.

use bevy_grimoire_core::descriptor::{Align, Descriptor, Extent, Mount, Offers};
use bevy_grimoire_core::library::{LIBRARY_VERSION, Library};
use bevy_grimoire_core::map::{Map, Placed};
use bevy_grimoire_core::stack::{resolve_y, surface_lift};

fn host(id: &str, height: f32, y_offset: Option<f32>, stretch: Option<f32>) -> Descriptor {
    Descriptor {
        id: id.into(),
        extent: Extent {
            footprint: Some((1.0, 1.0)),
            height: Some(height),
        },
        offers: Offers {
            surfaces: vec!["support".into()],
            ..Offers::default()
        },
        mount: Some(Mount::OnFloor),
        align: Align {
            y_offset,
            stretch_y: stretch,
            ..Align::default()
        },
        ..Descriptor::default()
    }
}

fn guest(y_offset: Option<f32>) -> Descriptor {
    Descriptor {
        id: "guest".into(),
        extent: Extent {
            footprint: Some((0.2, 0.2)),
            height: Some(0.1),
        },
        mount: Some(Mount::OnSurface {
            class: "support".into(),
        }),
        align: Align {
            y_offset,
            ..Align::default()
        },
        ..Descriptor::default()
    }
}

/// Resolve a guest on a host; returns (host Y, guest Y).
fn resolve(h: Descriptor, g: Descriptor) -> (f32, f32) {
    let lib = Library {
        version: LIBRARY_VERSION,
        note: None,
        descriptors: vec![h.clone(), g.clone()],
    };
    let at = |id: &str, d: &str, on: Option<&str>| Placed {
        id: id.into(),
        descriptor: d.into(),
        on: on.map(str::to_owned),
        ..Placed::default()
    };
    let map = Map {
        name: "lid".into(),
        placements: vec![at("h", &h.id, None), at("g", &g.id, Some("h"))],
        ..Map::default()
    };
    let ys = resolve_y(&map, &lib).unwrap_or_else(|e| panic!("{e}"));
    (ys[0], ys[1])
}

#[test]
fn a_host_with_an_offset_seats_its_guest_at_height_above_the_ground() {
    let (hy, gy) = resolve(host("h", 0.93, Some(0.052), None), guest(None));
    assert!((hy - 0.052).abs() < 1e-5, "host seated by its offset, got {hy}");
    assert!(
        (gy - 0.93).abs() < 1e-5,
        "guest must sit at the declared height above the ground, got {gy}"
    );
}

#[test]
fn a_host_with_no_offset_is_unchanged() {
    let (_, gy) = resolve(host("h", 0.9, None, None), guest(None));
    assert!((gy - 0.9).abs() < 1e-6, "got {gy}");
}

#[test]
fn a_stretched_host_still_scales_the_lift() {
    let (_, gy) = resolve(host("h", 1.0, Some(0.05), Some(1.2)), guest(None));
    assert!((gy - 1.2).abs() < 1e-5, "1.0 * 1.2 above the ground, got {gy}");
}

#[test]
fn a_guests_own_offset_is_kept() {
    let (_, gy) = resolve(host("h", 0.93, Some(0.052), None), guest(Some(-0.06)));
    assert!((gy - (0.93 - 0.06)).abs() < 1e-5, "got {gy}");
}

/// A host that declares a height taller than its mesh leaves its guest that far above the mesh. This is
/// the *known* outcome: a future `lid` field makes it a visible change, not a silent one.
#[test]
fn a_host_declaring_a_taller_height_than_its_mesh_leaves_the_guest_above_the_mesh_top() {
    let mesh_top = 0.893;
    let (_, gy) = resolve(host("h", mesh_top + 0.037, None, None), guest(None));
    assert!(
        (gy - mesh_top - 0.037).abs() < 1e-5,
        "guest rests at the declared height, got {gy}"
    );
}

#[test]
fn a_descriptor_reports_where_its_lid_is_above_its_origin() {
    assert_eq!(surface_lift(&host("h", 0.93, Some(0.052), None)), Some(0.93 - 0.052));
    assert_eq!(surface_lift(&Descriptor::default()), None);
}
