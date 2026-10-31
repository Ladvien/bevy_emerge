//! **Deleting `family/bevy_elements.ron` must give the editor camera back as the editor made it** (#193).
#![cfg(feature = "bevy_elements")]
//!
//! `ElementsCamera` requires `Hdr`, and the crate never removes it: `dress_camera` removed only
//! `ElementsCamera`, so the camera went on rendering into its own HDR texture while
//! `composite_surface(false, ..)` put the surface back to the plain, `blend_state: None` write the
//! editor draws with. That is the state #118 describes (the interface camera's write replaces the
//! world) and why the world never came back without a restart. The editor's camera has no `Hdr` of
//! its own (`view::setup`), so undressing takes `Hdr` off. No GPU is available to a test, so this
//! asserts the component the frame depends on, not the frame.

use bevy::camera::{Camera, Hdr};
use bevy::prelude::*;
use bevy_elements::ElementsCamera;
use bevy_grimoire_mapper::environment::dress_camera;
use bevy_grimoire_mapper::family_host::FamilySettings;
use bevy_grimoire_mapper::surface::SurfaceCamera;
use bevy_grimoire_mapper::view::MainCamera;

#[test]
fn the_file_going_takes_the_hdr_it_brought_off_the_camera() {
    let mut app = App::new();
    app.init_resource::<FamilySettings>().add_systems(Update, dress_camera);
    let world = app.world_mut().spawn((MainCamera, Camera::default())).id();
    app.world_mut().spawn((SurfaceCamera, Camera::default()));

    app.update();
    assert!(app.world().get::<Hdr>(world).is_none(), "no family file: the editor's own camera");

    app.world_mut()
        .resource_mut::<FamilySettings>()
        .text
        .insert("bevy_elements".to_owned(), String::new());
    app.update();
    assert!(app.world().get::<ElementsCamera>(world).is_some(), "the file dresses the camera");
    assert!(app.world().get::<Hdr>(world).is_some(), "premise: dressing brings Hdr with it");

    app.world_mut().resource_mut::<FamilySettings>().text.clear();
    app.update();
    assert!(app.world().get::<ElementsCamera>(world).is_none(), "the file is gone: undressed");
    assert!(
        app.world().get::<Hdr>(world).is_none(),
        "the camera went on drawing into an HDR texture the plain surface composition does not read"
    );
}
