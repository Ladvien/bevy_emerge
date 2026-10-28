//! **The editor as host of the owner's family crates** (#96).
//!
//! Each family crate is an optional cargo feature of this crate, named after it. A project's
//! `family.ron` (`bevy_emerge_core::family`) lists the ones it uses. A listed crate this build was
//! compiled without is a problem the author is told about, never a silent no-op.

use bevy::prelude::*;
use bevy_emerge_core::family::Family;
use std::path::PathBuf;

use crate::editor::EditorState;
use crate::project::Project;

/// Every crate this editor can host, whether or not this build has it. Add a name here in the
/// commit that adds the cargo feature.
pub const KNOWN: &[&str] = &["bevy_elements"];

/// The family crates this build was compiled with: one entry per enabled cargo feature.
pub fn built() -> Vec<&'static str> {
    let mut on = Vec::new();
    if cfg!(feature = "bevy_elements") {
        on.push("bevy_elements");
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
