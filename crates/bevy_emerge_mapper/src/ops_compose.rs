//! `composition_set`: an agent (or the Keep-as-group door) defines, replaces or removes a composition,
//! its envelope and its members included.

use bevy_emerge_core::composition::Composition;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ops::{Ctx, Operation, Outcome, Refusal, Undo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// The whole composition to define. An id already in the project is replaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition: Option<Composition>,
    /// The id an undo takes out of the project. Only an undo may set this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<String>,
    /// The composition an undo puts back. Only an undo may set this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore: Option<Composition>,
}

impl Params {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "composition": {
                    "description": "A whole composition as compositions.ron writes it: `id`, `envelope` (`Anchored` or `Bounded(size: (w, h, d))` in the RON form's JSON equivalent), `members`, optional `locations` and `note`. The id is a project id; one already defined is replaced, and a map's stamps of it then expand to the new definition. The whole set is validated first, so a member the library lacks or a nested group that does not resolve refuses and writes nothing.",
                    "type": "object"
                },
                "remove": {"description": "The id an undo takes out. Only an undo may set this.", "type": "string"},
                "restore": {"description": "The composition an undo puts back. Only an undo may set this.", "type": "object"}
            }
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
        let p: Params = serde_json::from_value(raw.clone()).map_err(|e| {
            Refusal::new(format!(
                "these are not the parameters for a composition: {e}. Give `composition`; `remove` and `restore` are undo forms."
            ))
        })?;
        let n = usize::from(p.composition.is_some())
            + usize::from(p.remove.is_some())
            + usize::from(p.restore.is_some());
        if n != 1 {
            return Err(Refusal::new(
                "give exactly one of `composition`, `remove` or `restore`. Nothing was changed.",
            ));
        }
        Ok(p)
    }
}

pub struct CompositionSet;

impl CompositionSet {
    /// Run against a scratch clone when `write` is false, so a preview asks what the write asks.
    fn apply(ctx: &mut Ctx, params: &serde_json::Value, write: bool) -> Result<Outcome, Refusal> {
        let p = Params::parse(params)?;
        let Some(project) = ctx.project.as_mut() else {
            return Err(Refusal::new("no project is open, so there are no compositions to change."));
        };
        if let Some(id) = &p.remove {
            let gone = if write {
                project.remove_composition(id)
            } else {
                project.plan_remove_composition(id).map(|(_, gone)| gone)
            }
            .map_err(Refusal::new)?;
            return Ok(Outcome {
                summary: format!("took composition `{id}` out of the project"),
                affected: 1,
                saved: write,
                inverse: Some(json!({"restore": gone})),
                after: Some(json!({"id": id})),
            });
        }
        let (comp, verb) = match (p.composition, p.restore) {
            (Some(c), _) => (c, "defined"),
            (_, Some(c)) => (c, "put back"),
            _ => unreachable!("parse demands exactly one"),
        };
        let id = comp.id.clone();
        let before = project.compositions.compositions.iter().find(|c| c.id == id).cloned();
        if write {
            project.commit_composition(comp)
        } else {
            project.plan_composition(comp).map(|_| ())
        }
        .map_err(Refusal::new)?;
        let inverse = match before {
            Some(prev) => json!({"restore": prev}),
            None => json!({"remove": id}),
        };
        Ok(Outcome {
            summary: format!("{verb} composition `{id}`"),
            affected: 1,
            saved: write,
            inverse: Some(inverse),
            after: Some(json!({"id": id})),
        })
    }
}

impl Operation for CompositionSet {
    fn name(&self) -> &'static str {
        "composition_set"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        Params::parse(params).map(|p| p.remove.is_some() || p.restore.is_some()).unwrap_or(false)
    }
    fn schema(&self) -> serde_json::Value {
        Params::schema()
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let mut c = Ctx { project: ctx.project.clone(), ..Ctx::default() };
        let o = Self::apply(&mut c, params, false)?;
        Ok(json!({"would": o.summary}))
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Self::apply(ctx, params, true)
    }
}
