//! **The game's answer to a reload**, written where the editor can read it.
//!
//! A game that reloads its emerge project when the editor writes it either takes the new map or
//! refuses it (a descriptor id that does not resolve, a plan that cannot be built). Without a trace
//! the editor, and an agent driving it, keeps editing a map the game stopped following. The game
//! writes a [`Verdict`] to `<emerge_dir>/.live/game.ron`; the dot-directory is skipped by
//! [`super::FileWatch`], so this write never triggers a reload.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::ron_surgery::save_atomic;

/// The schema this build reads and writes.
pub const VERDICT_VERSION: u32 = 1;

/// **What the game said about its latest reload.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub version: u32,
    /// Milliseconds since the Unix epoch when the game decided.
    pub at_unix_ms: u64,
    /// True when the reload landed.
    pub ok: bool,
    /// What the game said: the diff on success, the refusal's reason otherwise.
    pub message: String,
}

impl Verdict {
    /// A verdict stamped now.
    pub fn now(ok: bool, message: impl Into<String>) -> Verdict {
        let at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        Verdict { version: VERDICT_VERSION, at_unix_ms, ok, message: message.into() }
    }

    /// `<emerge_dir>/.live/game.ron`.
    pub fn path(emerge_dir: &Path) -> PathBuf {
        emerge_dir.join(".live").join("game.ron")
    }

    /// Writes the verdict atomically, creating `.live/`.
    pub fn write(&self, emerge_dir: &Path) -> Result<(), String> {
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("live verdict: {e}"))?;
        save_atomic(&Self::path(emerge_dir), &text)
    }

    /// An absent file is `Ok(None)`; a file that does not parse, or comes from a newer version,
    /// is an `Err` naming its path.
    pub fn read(emerge_dir: &Path) -> Result<Option<Verdict>, String> {
        let path = Self::path(emerge_dir);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let v: Verdict =
            ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if v.version > VERDICT_VERSION {
            return Err(format!(
                "{}: version {} but this build reads {VERDICT_VERSION} - refusing a verdict \
                 written by a newer tool",
                path.display(),
                v.version
            ));
        }
        Ok(Some(v))
    }
}
