//! **A painted piece despawned in the frame its materials arrive does not take the app down.**
//!
//! A glTF scene's materials land frames after the piece is spawned, and the editor despawns and
//! respawns pieces (a brush ghost follows the cursor) as fast as frames come. When the despawn and the
//! materials' arrival fall in the same frame, the paint system's material swap reaches an entity that
//! is gone; measured 2026-10-30 as an editor panic ("Entity despawned ... MeshMaterial3d<StandardMaterial>")
//! on a click during a capture sequence (#140). The swap is a cosmetic follow-up, so a gone entity is
//! skipped.

use bevy::prelude::*;
use bevy_grimoire::{GrimoirePlugin, Paint};

#[derive(Component)]
struct DespawnThisFrame;

/// Despawns the marked roots through commands, in the same schedule as the paint system, so both see
/// the piece and both queue a command against it.
fn despawn_marked(mut commands: Commands, marked: Query<Entity, With<DespawnThisFrame>>) {
    for e in &marked {
        commands.entity(e).despawn();
    }
}

/// The app, with the despawner registered before or after the plugin: which buffer the schedule
/// applies first is not something a game controls, so both orders must hold.
fn app(despawner_first: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<StandardMaterial>();
    if despawner_first {
        app.add_systems(Update, despawn_marked).add_plugins(GrimoirePlugin);
    } else {
        app.add_plugins(GrimoirePlugin).add_systems(Update, despawn_marked);
    }
    app
}

fn despawned_as_materials_arrive(despawner_first: bool) {
    let mut app = app(despawner_first);
    let material = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
    let root = app.world_mut().spawn((Paint(2), Transform::default(), DespawnThisFrame)).id();
    let child = app.world_mut().spawn((MeshMaterial3d(material), ChildOf(root))).id();

    app.update();

    assert!(app.world().get_entity(root).is_err(), "the root was despawned this frame");
    assert!(app.world().get_entity(child).is_err(), "and its child with it");
    // A piece that stays is still painted: the swap is skipped only for the gone entity.
    let material = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
    let kept = app.world_mut().spawn((Paint(2), Transform::default())).id();
    let leaf = app.world_mut().spawn((MeshMaterial3d(material.clone()), ChildOf(kept))).id();
    app.update();
    let now = app.world().get::<MeshMaterial3d<StandardMaterial>>(leaf).unwrap().0.id();
    assert_ne!(now, material.id(), "a painted piece that stays gets its biased twin");
}

#[test]
fn a_piece_despawned_as_its_materials_arrive_is_skipped_not_a_panic_despawner_first() {
    despawned_as_materials_arrive(true);
}

#[test]
fn a_piece_despawned_as_its_materials_arrive_is_skipped_not_a_panic_paint_first() {
    despawned_as_materials_arrive(false);
}
