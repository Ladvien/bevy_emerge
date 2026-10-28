//! **Notes for agents: the data, its file beside the map, and the rules that keep it honest.**
//!
//! A person marks tiles or placed meshes with a note; an agent lists the open ones, does the work
//! through ordinary operations and resolves the note. This module is the engine-free half of that:
//! what a note is, where it is written, and the four changes a note can undergo. The operations
//! that make those changes undoable and reachable from the UI and MCP are built on top of it.
//!
//! # Where it lives
//!
//! One file per map, **beside the map**: `maps/town.map.ron` has `maps/town.notes.ron`. It is
//! written through [`crate::ron_surgery::save_atomic`] like the map is, and its extension is `ron`,
//! so a [`crate::live::FileWatch`] over the project already sees it change and a running game or
//! editor reloads it with the rest.
//!
//! # What a note keeps
//!
//! An id (`n1`, `n2`, … never reused: a resolved note stays on record, so its id is spoken for),
//! its targets (placement ids and cells), its text, and its state. Resolving records the ids of the
//! operations the agent ran and one line of what changed, so a resolved note answers "what was done
//! about this".

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file format's version, written first so a later change can be migrated.
pub const NOTES_VERSION: u32 = 1;

/// What a note is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// A placed mesh, by [`crate::map::Placed::id`].
    Placement(String),
    /// A tile, by grid cell.
    Cell(i32, i32),
}

/// Where a note stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Open,
    /// Worked. `ops` names the operations the agent ran; `summary` is one line of what changed.
    Resolved { ops: Vec<String>, summary: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub targets: Vec<Target>,
    pub text: String,
    pub state: State,
}

impl Note {
    pub fn is_open(&self) -> bool {
        self.state == State::Open
    }
}

/// Every note of one map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Notes {
    pub version: u32,
    pub notes: Vec<Note>,
}

impl Default for Notes {
    fn default() -> Self {
        Notes { version: NOTES_VERSION, notes: Vec::new() }
    }
}

/// The notes file that belongs to a map file: `town.map.ron` → `town.notes.ron`, in the same
/// directory. A map path that does not end `.map.ron` gets `.notes.ron` appended to its stem.
pub fn path_for_map(map_path: &Path) -> PathBuf {
    let name = map_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = name
        .strip_suffix(".map.ron")
        .or_else(|| name.strip_suffix(".ron"))
        .unwrap_or(&name);
    map_path.with_file_name(format!("{stem}.notes.ron"))
}

impl Notes {
    pub fn from_ron(text: &str) -> Result<Notes, String> {
        let notes: Notes = ron::from_str(text).map_err(|e| format!("notes: parse: {e}"))?;
        if notes.version != NOTES_VERSION {
            return Err(format!(
                "notes: version {} is not the {NOTES_VERSION} this build reads",
                notes.version
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for n in &notes.notes {
            if !seen.insert(n.id.as_str()) {
                return Err(format!("notes: id {} appears twice", n.id));
            }
        }
        Ok(notes)
    }

    pub fn to_ron(&self) -> Result<String, String> {
        let mut text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("notes: serialize: {e}"))?;
        if !text.ends_with('\n') {
            text.push('\n');
        }
        Ok(text)
    }

    /// Reads the notes file; a file that is not there is no notes, not an error.
    pub fn load(path: &Path) -> Result<Notes, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Notes::from_ron(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Notes::default()),
            Err(e) => Err(format!("notes: {}: {e}", path.display())),
        }
    }

    /// Writes the notes file atomically. No notes and no file writes nothing, so a map that never
    /// had a note never grows one.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if self.notes.is_empty() && !path.exists() {
            return Ok(());
        }
        crate::ron_surgery::save_atomic(path, &self.to_ron()?)
    }

    pub fn get(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    /// The notes an agent still has to work, in the order they were left.
    pub fn open(&self) -> impl Iterator<Item = &Note> {
        self.notes.iter().filter(|n| n.is_open())
    }

    pub fn resolved(&self) -> impl Iterator<Item = &Note> {
        self.notes.iter().filter(|n| !n.is_open())
    }

    fn next_id(&self) -> String {
        let top = self
            .notes
            .iter()
            .filter_map(|n| n.id.strip_prefix('n')?.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        format!("n{}", top + 1)
    }

    fn get_mut(&mut self, id: &str) -> Result<&mut Note, String> {
        self.notes.iter_mut().find(|n| n.id == id).ok_or_else(|| format!("notes: no note {id}"))
    }

    /// Leaves a note. A note needs a target and some text.
    pub fn add(&mut self, targets: Vec<Target>, text: &str) -> Result<String, String> {
        if targets.is_empty() {
            return Err("notes: a note needs at least one target".to_owned());
        }
        if text.trim().is_empty() {
            return Err("notes: a note needs some text".to_owned());
        }
        let id = self.next_id();
        self.notes.push(Note { id: id.clone(), targets, text: text.to_owned(), state: State::Open });
        Ok(id)
    }

    /// Changes an open note's text. A resolved note is a record and is not edited.
    pub fn edit(&mut self, id: &str, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("notes: a note needs some text".to_owned());
        }
        let note = self.get_mut(id)?;
        if !note.is_open() {
            return Err(format!("notes: {id} is resolved; reopen it to edit"));
        }
        note.text = text.to_owned();
        Ok(())
    }

    /// Resolves an open note, naming the operations that did the work and what they changed.
    pub fn resolve(&mut self, id: &str, ops: Vec<String>, summary: &str) -> Result<(), String> {
        let note = self.get_mut(id)?;
        if !note.is_open() {
            return Err(format!("notes: {id} is already resolved"));
        }
        note.state = State::Resolved { ops, summary: summary.to_owned() };
        Ok(())
    }

    /// Puts a resolved note back to open (the undo of a resolve).
    pub fn reopen(&mut self, id: &str) -> Result<(), String> {
        let note = self.get_mut(id)?;
        if note.is_open() {
            return Err(format!("notes: {id} is already open"));
        }
        note.state = State::Open;
        Ok(())
    }

    /// Placement targets that no longer exist in the map, as `(note id, placement id)`. A note
    /// outlives the placement it marked (an agent may delete it while working the note), so this
    /// is a report, never a load error.
    pub fn dangling(&self, map: &crate::map::Map) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for n in &self.notes {
            for t in &n.targets {
                if let Target::Placement(p) = t
                    && !map.placements.iter().any(|x| &x.id == p)
                {
                    out.push((n.id.clone(), p.clone()));
                }
            }
        }
        out
    }
}
