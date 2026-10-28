//! `family.ron`: which of the owner's family crates a project uses.
//!
//! The editor hosts each family crate behind a cargo feature named after it. A project names the crates
//! it uses here, and the editor shows only those panels. Listing a crate the running build was not
//! compiled with is a problem the author is told about ([`Family::problems`]), never a silent no-op.
//! Engine-free: the build's feature set is passed in by the caller.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FAMILY_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    pub version: u32,
    pub crates: Vec<String>,
}

impl Default for Family {
    fn default() -> Self {
        Family { version: FAMILY_VERSION, crates: Vec::new() }
    }
}

/// `<project>/family.ron`.
pub fn path_for_project(root: &Path) -> PathBuf {
    root.join("family.ron")
}

impl Family {
    pub fn from_ron(text: &str) -> Result<Family, String> {
        let f: Family = ron::from_str(text).map_err(|e| format!("family: parse: {e}"))?;
        if f.version != FAMILY_VERSION {
            return Err(format!("family: version {} is not the {FAMILY_VERSION} this build reads", f.version));
        }
        let mut seen = std::collections::BTreeSet::new();
        for c in &f.crates {
            if !seen.insert(c.as_str()) {
                return Err(format!("family: {c} is listed twice"));
            }
        }
        Ok(f)
    }

    /// Reads the project's `family.ron`; a project without one uses no family crate.
    pub fn load(root: &Path) -> Result<Family, String> {
        let path = path_for_project(root);
        match std::fs::read_to_string(&path) {
            Ok(text) => Family::from_ron(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Family::default()),
            Err(e) => Err(format!("family: {}: {e}", path.display())),
        }
    }

    /// One message per listed crate the editor cannot host: an unknown name, or a known crate whose
    /// feature this build lacks. `known` is every crate the host can host, `built` the subset compiled in. Both come from the
    /// editor: this crate is engine-free and names no engine crate.
    pub fn problems(&self, known: &[&str], built: &[&str]) -> Vec<String> {
        self.crates
            .iter()
            .filter_map(|c| {
                if !known.contains(&c.as_str()) {
                    Some(format!("family.ron lists {c}, which is not a family crate the editor knows"))
                } else if !built.contains(&c.as_str()) {
                    Some(format!(
                        "family.ron lists {c}, but this editor was built without the `{c}` feature; rebuild with `--features {c}`"
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    /// The listed crates this build can host, in file order: the panels to show.
    pub fn active<'a>(&'a self, built: &[&str]) -> Vec<&'a str> {
        self.crates.iter().map(String::as_str).filter(|c| built.contains(c)).collect()
    }
}
