//! **A family crate's settings, as one registered operation** (#96): `family_set`.
//!
//! The settings are the crate's own serde types written as RON in `<project>/family/<crate>.ron`.
//! The editor holds no schema for them: this operation rewrites one dot-pathed field's value text
//! (`bevy_emerge_core::family::set_field`) and keeps the rest of the file as the author left it. Its
//! inverse is the same operation with the old value, so a person's Enter and an agent's `apply` are
//! one change with one log line and one undo.

use bevy_emerge_core::family;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ops::{Ctx, Operation, Outcome, Refusal, Undo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// The family crate whose settings file is written, as named in `family.ron`.
    #[serde(rename = "crate")]
    pub krate: String,
    /// Dot-separated field names inside the settings, `sun.hour`.
    pub field: String,
    /// One RON value: `6.5`, `"dusk"`, `(1.0, 2.0)`.
    pub value: String,
}

impl Params {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["crate", "field", "value"],
            "properties": {
                "crate": {"type": "string", "description": "The family crate whose settings are written, as listed in family.ron."},
                "field": {"type": "string", "description": "Dot-separated field names inside its settings file, e.g. sun.hour."},
                "value": {"type": "string", "description": "The new value as one RON value, e.g. 6.5 or \"dusk\"."}
            }
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
        serde_json::from_value(raw.clone()).map_err(|e| {
            Refusal::new(format!("these are not a family setting: {e}. The fields are `crate`, `field` and `value`, and nothing else."))
        })
    }
}

fn settings_path(ctx: &Ctx, krate: &str) -> Result<std::path::PathBuf, Refusal> {
    let Some(root) = ctx.root_or_project() else {
        return Err(Refusal::new("no project is open, so there are no family settings to change."));
    };
    // A crate name is one path segment: it is a file name, never a route out of `family/`.
    if krate.is_empty() || krate.contains(['/', '\\', '.']) {
        return Err(Refusal::new(format!("`{krate}` is not a family crate name. Nothing was written.")));
    }
    let listed = family::Family::load(&root).map_err(Refusal::new)?;
    if !listed.crates.iter().any(|c| c == krate) {
        return Err(Refusal::new(format!("{krate} is not listed in family.ron, so its settings were not changed.")));
    }
    Ok(root.join("family").join(format!("{krate}.ron")))
}

fn read(ctx: &Ctx, krate: &str) -> Option<String> {
    std::fs::read_to_string(settings_path(ctx, krate).ok()?).ok()
}

/// **Set one field of a family crate's settings file.**
pub struct FamilySet;

impl FamilySet {
    fn apply(ctx: &Ctx, params: &serde_json::Value, write: bool) -> Result<Option<Outcome>, Refusal> {
        let p = Params::parse(params)?;
        let path = settings_path(ctx, &p.krate)?;
        let text = std::fs::read_to_string(&path)
            .map_err(|e| Refusal::new(format!("{} cannot be read, so nothing was changed: {e}", path.display())))?;
        let (new, old) = family::set_field(&text, &p.field, &p.value).map_err(Refusal::new)?;
        if !write {
            return Ok(None);
        }
        std::fs::write(&path, &new)
            .map_err(|e| Refusal::new(format!("{} refused the change, so nothing was written: {e}", path.display())))?;
        Ok(Some(Outcome {
            summary: format!("{}: {} = {}", p.krate, p.field, p.value.trim()),
            affected: 1,
            saved: true,
            inverse: Some(json!({"crate": p.krate, "field": p.field, "value": old})),
            after: Some(json!({"crate": p.krate, "file": new})),
        }))
    }
}

impl Operation for FamilySet {
    fn name(&self) -> &'static str {
        "family_set"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn schema(&self) -> serde_json::Value {
        Params::schema()
    }
    fn state_of(&self, ctx: &Ctx, after: &serde_json::Value) -> Option<serde_json::Value> {
        let krate = after.get("crate")?.as_str()?;
        Some(json!({"crate": krate, "file": read(ctx, krate)?}))
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        Self::apply(ctx, params, false)?;
        Ok(params.clone())
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Ok(Self::apply(ctx, params, true)?.expect("a write returns its outcome"))
    }
}
