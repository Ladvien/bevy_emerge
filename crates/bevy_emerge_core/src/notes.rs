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
    /// Written by a person and not yet sent: no agent sees it, the sign stands on the map.
    Draft,
    Open,
    /// Worked. `ops` names the operations the agent ran; `summary` is one line of what changed.
    Resolved { ops: Vec<String>, summary: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub targets: Vec<Target>,
    pub text: String,
    pub state: State,
    /// Where the person drew it and what stood there; `None` for a note made from targets alone.
    #[serde(default)]
    pub site: Option<Site>,
}

/// The place a note was written about, as it was when written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    /// Inclusive cell rectangle `(min_x, min_z, max_x, max_z)`.
    pub cells: (i32, i32, i32, i32),
    /// World bounds `(min_x, min_z, max_x, max_z)` in metres.
    pub world: (f32, f32, f32, f32),
    /// Ids of the placements inside the area when the note was written.
    pub pieces: Vec<String>,
    /// Cells around the area an agent may also touch.
    pub margin: i32,
    /// Camera when written: `(x, y, z, yaw, pitch)`.
    pub camera: (f32, f32, f32, f32, f32),
}

/// An agent works a note within one cell of its area.
pub const DEFAULT_MARGIN: i32 = 1;

impl Site {
    /// The cell rectangle an agent may work: the area grown by the margin.
    pub fn reach(&self) -> (i32, i32, i32, i32) {
        let (a, b, c, d) = self.cells;
        (a - self.margin, b - self.margin, c + self.margin, d + self.margin)
    }
}

impl Note {
    pub fn is_open(&self) -> bool {
        self.state == State::Open
    }

    pub fn is_draft(&self) -> bool {
        self.state == State::Draft
    }

    /// A sign stands over it: a draft or an open note, not a resolved one.
    pub fn stands(&self) -> bool {
        !matches!(self.state, State::Resolved { .. })
    }
}

/// Every note of one map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        self.notes.iter().filter(|n| !n.stands())
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
        self.notes.push(Note { id: id.clone(), targets, text: text.to_owned(),
            state: State::Open,
            site: None,
        });
        Ok(id)
    }

    /// Leaves a draft note on a cell rectangle, recording what the map holds there. The targets
    /// are the placements inside the area (as [`Site::pieces`]) or, when it holds none, its cells.
    pub fn add_area(
        &mut self,
        map: &crate::map::Map,
        cells: (i32, i32, i32, i32),
        cell_size: f32,
        camera: (f32, f32, f32, f32, f32),
        text: &str,
    ) -> Result<String, String> {
        let (x0, z0) = (cells.0.min(cells.2), cells.1.min(cells.3));
        let (x1, z1) = (cells.0.max(cells.2), cells.1.max(cells.3));
        let world = (x0 as f32 * cell_size, z0 as f32 * cell_size, (x1 + 1) as f32 * cell_size, (z1 + 1) as f32 * cell_size);
        let pieces: Vec<String> = map
            .placements
            .iter()
            .filter(|p| {
                let (x, z) = (p.at.0, p.at.1);
                x >= world.0 && x < world.2 && z >= world.1 && z < world.3
            })
            .map(|p| p.id.clone())
            .collect();
        let targets = if pieces.is_empty() {
            let mut t = Vec::new();
            for x in x0..=x1 {
                for z in z0..=z1 {
                    t.push(Target::Cell(x, z));
                }
            }
            t
        } else {
            pieces.iter().cloned().map(Target::Placement).collect()
        };
        let site = Site { cells: (x0, z0, x1, z1), world, pieces, margin: DEFAULT_MARGIN, camera };
        let id = self.add(targets, text)?;
        let note = self.get_mut(&id)?;
        note.state = State::Draft;
        note.site = Some(site);
        Ok(id)
    }

    /// Leaves a draft note on pieces a person Shift-clicked, recording the cells they cover.
    pub fn add_pieces(
        &mut self,
        map: &crate::map::Map,
        ids: &[String],
        cell_size: f32,
        camera: (f32, f32, f32, f32, f32),
        text: &str,
    ) -> Result<String, String> {
        let mut found = Vec::new();
        for id in ids {
            let p = map
                .placements
                .iter()
                .find(|p| &p.id == id)
                .ok_or_else(|| format!("notes: no placement {id}"))?;
            found.push(p);
        }
        if found.is_empty() {
            return Err("notes: a note needs at least one target".to_owned());
        }
        let cell = |v: f32| (v / cell_size).floor() as i32;
        let (mut x0, mut z0, mut x1, mut z1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for p in &found {
            let (x, z) = (cell(p.at.0), cell(p.at.1));
            (x0, z0, x1, z1) = (x0.min(x), z0.min(z), x1.max(x), z1.max(z));
        }
        let world = (x0 as f32 * cell_size, z0 as f32 * cell_size, (x1 + 1) as f32 * cell_size, (z1 + 1) as f32 * cell_size);
        let pieces: Vec<String> = ids.to_vec();
        let targets = pieces.iter().cloned().map(Target::Placement).collect();
        let id = self.add(targets, text)?;
        let note = self.get_mut(&id)?;
        note.state = State::Draft;
        note.site = Some(Site { cells: (x0, z0, x1, z1), world, pieces, margin: DEFAULT_MARGIN, camera });
        Ok(id)
    }

    /// Takes a note off the map. Its id stays spoken for only while other notes still hold a
    /// higher one, so removing the newest note frees its id.
    pub fn remove(&mut self, id: &str) -> Result<Note, String> {
        let at = self.notes.iter().position(|n| n.id == id).ok_or_else(|| format!("notes: no note {id}"))?;
        Ok(self.notes.remove(at))
    }

    /// Changes a draft or open note's text. A resolved note is a record and is not edited.
    pub fn edit(&mut self, id: &str, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("notes: a note needs some text".to_owned());
        }
        let note = self.get_mut(id)?;
        if !note.stands() {
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
        if note.stands() {
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
