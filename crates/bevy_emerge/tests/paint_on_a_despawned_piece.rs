//! **A painted piece despawned in the frame its materials arrive does not take the app down.**
//!
//! A glTF scene's materials land frames after the piece is spawned, and the editor despawns and
//! respawns pieces (a brush ghost follows the cursor) as fast as frames come. When the despawn and the
//! materials' arrival fall in the same frame, the paint system's material swap reaches an entity that
//! is gone; measured 2026-10-30 as an editor panic ("Entity despawned ... MeshMaterial3d<StandardMaterial>")
//! on a click during a capture sequence. The swap is a cosmetic follow-up, so a gone entity is skipped.

use bevy::prelude::*;
use bevy_emerge::{EmergePlugin, Paint};

#[derive(Component)]
struct DespawnThisFrame;

/// Despawns the marked roots through commands. Added before the plugin, so its command buffer is
/// applied ahead of the paint system's in the same frame: the paint system has already seen the
/// material arrive and queued its swap, and the swap then meets a despawned entity.
fn despawn_marked(mut commands: Commands, marked: Query<Entity, With<DespawnThisFrame>>) {
    for e in &marked {
        commands.entity(e).despawn();
    }
}

#[test]
fn a_piece_despawned_as_its_materials_arrive_is_skipped_not_a_panic() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<StandardMaterial>()
        .add_systems(Update, despawn_marked)
        .add_plugins(EmergePlugin);
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
