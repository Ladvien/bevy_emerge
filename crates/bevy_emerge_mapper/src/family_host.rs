//! **The editor as host of the owner's family crates** (#96).
//!
//! Each family crate is an optional cargo feature of this crate, named after it. A project's
//! `family.ron` (`bevy_emerge_core::family`) lists the ones it uses. A listed crate this build was
//! compiled without is a problem the author is told about, never a silent no-op.

use bevy::prelude::*;
use bevy_emerge_core::family::{Family, FamilyWatch, SettingsChange};
use std::path::PathBuf;

use crate::editor::EditorState;
use crate::project::Project;

/// Every crate this editor can host, whether or not this build has it. Add a name here in the
/// commit that adds the cargo feature.
pub const KNOWN: &[&str] = &["bevy_carnage", "bevy_elements", "bevy_flora", "bevy_gooey", "bevy_lights", "bevy_orca", "bevy_society", "bevy_speech_bubbles", "bevy_stigmergy"];

/// The family crates this build was compiled with: one entry per enabled cargo feature.
pub fn built() -> Vec<&'static str> {
    let mut on = Vec::new();
    if cfg!(feature = "bevy_carnage") {
        on.push("bevy_carnage");
    }
    if cfg!(feature = "bevy_elements") {
        on.push("bevy_elements");
    }
    if cfg!(feature = "bevy_flora") {
        on.push("bevy_flora");
    }
    if cfg!(feature = "bevy_gooey") {
        on.push("bevy_gooey");
    }
    if cfg!(feature = "bevy_lights") {
        on.push("bevy_lights");
    }
    if cfg!(feature = "bevy_orca") {
        on.push("bevy_orca");
    }
    if cfg!(feature = "bevy_society") {
        on.push("bevy_society");
    }
    if cfg!(feature = "bevy_speech_bubbles") {
        on.push("bevy_speech_bubbles");
    }
    if cfg!(feature = "bevy_stigmergy") {
        on.push("bevy_stigmergy");
    }
    on
}

/// **What is wrong with the project's `family.ron` for this build**: an unreadable file, an unknown
/// crate, or a crate whose feature is off. Empty when the project lists nothing or has no file.
pub fn problems_with(root: &std::path::Path, built: &[&str]) -> Vec<String> {
    match Family::load(root) {
        Ok(family) => family.problems(KNOWN, built),
        Err(e) => vec![e],
    }
}

/// Reports the project's family problems once per opened project root, on the status line where the
/// author looks.
pub fn report_problems(
    project: Option<Res<Project>>,
    mut state: ResMut<EditorState>,
    mut told: Local<Option<PathBuf>>,
) {
    let Some(project) = project else { return };
    if told.as_deref() == Some(project.root.as_path()) {
        return;
    }
    *told = Some(project.root.clone());
    for p in problems_with(&project.root, &built()) {
        state.status.problem(p);
    }
}

/// **Hot reload of family settings** (#96): a settings file that changes on disk, from `family_set`
/// or from a person's editor, is read once it settles and reported on the status line. The new text
/// is kept in [`FamilySettings`] for the crate's own plugin to apply; a file that does not parse is a
/// problem and the previous settings stay.
#[derive(Resource, Default, Debug)]
pub struct FamilySettings {
    /// Crate name → the last good settings text.
    pub text: std::collections::BTreeMap<String, String>,
}

/// The set [`reload_settings`] runs in, which a crate's apply system orders after. A set rather than
/// the system itself, because ordering against a system type panics once a schedule holds two of it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reloaded;

pub fn reload_settings(
    project: Option<Res<Project>>,
    mut settings: ResMut<FamilySettings>,
    mut state: ResMut<EditorState>,
    mut watch: Local<Option<(PathBuf, FamilyWatch, std::time::Instant)>>,
) {
    let Some(project) = project else { return };
    if watch.as_ref().is_none_or(|(root, ..)| *root != project.root) {
        let fresh = FamilyWatch::new(&project.root);
        settings.text = fresh.current().clone();
        *watch = Some((project.root.clone(), fresh, std::time::Instant::now()));
    }
    let Some((_, w, started)) = watch.as_mut() else { return };
    for change in w.poll(started.elapsed().as_secs_f64()) {
        match change {
            SettingsChange::Reloaded { krate, text } => {
                state.status.note(format!("{krate} settings reloaded"));
                settings.text.insert(krate, text);
            }
            SettingsChange::Invalid { krate, error } => {
                state.status.problem(format!("{krate} settings are not valid RON, so the previous ones stay: {error}"));
            }
            SettingsChange::Removed { krate } => {
                state.status.note(format!("{krate} settings removed; its defaults apply"));
                settings.text.remove(&krate);
            }
        }
    }
}
