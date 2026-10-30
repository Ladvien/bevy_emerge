//! **The preview's sky is on the surface while dressed, and comes off whole when the file goes**
//! (#193, #194).
#![cfg(feature = "bevy_elements")]
//!
//! Measured on sib (Vulkan, Xvfb) with the editor live over BRP: with `family/bevy_elements.ron`
//! the viewport held only the ground colour, and turning the interface camera off showed the
//! dressed world (sky, grid, map box) on the surface underneath. Painting the ground camera red
//! turned the whole viewport red, so the interface was writing the ground camera's frame over the
//! world: the interface camera shares the ground camera's main texture, and `MsaaWriteback::Auto`
//! writes a lower camera's frame back into its MSAA target, so its transparent clear never
//! reached the composite. Setting the interface's `msaa_writeback` to `Off` live brought the world
//! back. No GPU is available to a test, so this asserts the camera state that frame depends on.
//!
//! Taking the file away left the world camera `Hdr` (a required component of `ElementsCamera`
//! stays when it goes) and every part `bevy_elements` put on it, so the undressed interface wiped
//! the world again; removing `Hdr` while the crate's atmosphere was still on the camera quit the
//! editor on a validation error ("Render pipeline targets are incompatible with render pass").
//! So the crate is asked to take its own dressing back first (`ElementsCamera::bare`), and only
//! then do the marker and `Hdr` go, with the editor's own exposure and fill light back.

use bevy::camera::{Exposure, Hdr, MsaaWriteback};
use bevy::prelude::*;
use bevy_elements::ElementsCamera;
use bevy_emerge_mapper::environment::dress_camera;
use bevy_emerge_mapper::family_host::FamilySettings;
use bevy_emerge_mapper::surface::SurfaceCamera;
use bevy_emerge_mapper::view::{MainCamera, editor_ambient};

fn app() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.init_resource::<FamilySettings>().add_systems(Update, dress_camera);
    let world = app.world_mut().spawn((MainCamera, Camera::default(), editor_ambient())).id();
    let ui = app.world_mut().spawn((SurfaceCamera, Camera::default())).id();
    (app, world, ui)
}

fn set_file(app: &mut App, present: bool) {
    let mut settings = app.world_mut().resource_mut::<FamilySettings>();
    if present {
        settings.text.insert("bevy_elements".to_owned(), String::new());
    } else {
        settings.text.clear();
    }
}

#[test]
fn a_dressed_interface_starts_from_its_own_clear_not_the_ground_written_back() {
    let (mut app, _, ui) = app();
    app.update();
    assert_eq!(
        app.world().get::<Camera>(ui).unwrap().msaa_writeback,
        MsaaWriteback::Auto,
        "no family file: the editor's own composition"
    );

    set_file(&mut app, true);
    app.update();
    assert_eq!(
        app.world().get::<Camera>(ui).unwrap().msaa_writeback,
        MsaaWriteback::Off,
        "dressed, the interface must not load the ground camera's frame back over the world"
    );

    set_file(&mut app, false);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world().get::<Camera>(ui).unwrap().msaa_writeback,
        MsaaWriteback::Auto,
        "undressed, the interface composes over the shared texture again"
    );
}

#[test]
fn taking_the_file_away_lets_the_crate_undress_before_the_camera_leaves_hdr() {
    let (mut app, world, _) = app();
    set_file(&mut app, true);
    app.update();
    assert!(app.world().get::<Hdr>(world).is_some(), "dressed: ElementsCamera brings Hdr");
    // What the crate drives while dressed: the exposure from the sun, the fill from the sky.
    app.world_mut().entity_mut(world).insert((Exposure { ev100: 15.0 }, AmbientLight { brightness: 0.0, ..default() }));

    set_file(&mut app, false);
    app.update();
    assert_eq!(
        app.world().get::<ElementsCamera>(world),
        Some(&ElementsCamera::bare()),
        "the crate is told to take its sky, fog and MSAA back first"
    );
    assert!(
        app.world().get::<Hdr>(world).is_some(),
        "Hdr stays while the crate's HDR-only passes may still be on the camera"
    );

    for _ in 0..3 {
        app.update();
    }
    let w = app.world().entity(world);
    assert!(!w.contains::<ElementsCamera>(), "undressed: the marker is gone");
    assert!(!w.contains::<Hdr>(), "undressed: the world is on the shared texture again, as before dressing");
    assert_eq!(w.get::<Exposure>().unwrap().ev100, Exposure::default().ev100, "the editor's own exposure");
    assert_eq!(
        w.get::<AmbientLight>().unwrap().brightness,
        editor_ambient().brightness,
        "the editor's own fill light"
    );

    set_file(&mut app, true);
    app.update();
    assert_eq!(
        app.world().get::<ElementsCamera>(world),
        Some(&ElementsCamera::default()),
        "the file back: dressed again"
    );
}
