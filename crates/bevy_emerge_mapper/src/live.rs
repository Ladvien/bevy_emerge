//! **`--live`: the editor writes every committed change through to disk.**
//!
//! A game that watches its emerge project (`bevy_emerge_core::live::FileWatch`) reloads when a file
//! changes, so the only thing between an edit and the running game is the editor's save. The
//! registered operations already save as they commit; the keys that are not yet operations (lift,
//! tip, move, straighten, the editor's own undo) change the map in memory and leave it unsaved until
//! `Cmd+S`. With `--live`, every frame that ends with the map dirty saves it, after every edit of the
//! frame has run.
//!
//! **No second write path.** The write is `OpenMap::save`, the same validation and the same atomic
//! write the save key uses, and the editor's own file watcher reads its own write back as the map it
//! already holds (`watch::adopt_map` calls `mark_saved`). A map that fails to save stays dirty, so the
//! next valid state is written; the refusal is logged once, not every frame.

use bevy::prelude::*;

use crate::project::{OpenMap, Project};

/// The write-through. Inert unless [`LiveWrite`] is inserted.
pub struct LivePlugin;

/// **Present when the editor runs with `--live`.** `main` inserts it; a test inserts it by hand.
#[derive(Resource)]
pub struct LiveWrite;

impl Plugin for LivePlugin {
    fn build(&self, app: &mut App) {
        // `PostUpdate`, so every `Update` dispatch and undo of the frame has landed first.
        app.add_systems(
            PostUpdate,
            write_through.run_if(resource_exists::<LiveWrite>),
        );
    }
}

/// Save the open map whenever it is dirty, and say what happened.
fn write_through(
    open: Option<ResMut<OpenMap>>,
    project: Option<Res<Project>>,
    mut last_refusal: Local<Option<String>>,
) {
    let (Some(mut open), Some(project)) = (open, project) else {
        return;
    };
    // Read through the immutable view first, so a clean frame does not mark the map changed.
    if !open.as_ref().dirty() {
        return;
    }
    match open.save(&project) {
        Ok(()) => {
            info!("live: wrote {}", open.map_path.display());
            *last_refusal = None;
        }
        Err(e) => {
            if last_refusal.as_deref() != Some(e.as_str()) {
                warn!("live: `{}` not written: {e}", open.map.name);
                *last_refusal = Some(e);
            }
        }
    }
}
