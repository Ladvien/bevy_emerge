//! **An agent defines a composition from placements on the map, as one registered operation**
//! (E-072): `composition_define`.
//!
//! The Map tab's `Shift+B` box and `keep_as_group` already turn a set of pieces into a reusable
//! group; this is the same conversion with the set named by placement ids instead of drawn. It goes
//! through [`crate::editor::set_of_placements`], [`crate::editor::composition_from_set`] and
//! [`crate::project::Project::commit_composition`], so the group an agent defines is the group a
//! person would have kept, with the same refusals (unmeasured piece, support left outside, hung
//! from the ceiling) and the same replace rule: an existing id is redefined only when `replace` is
//! true, and never over a stamp override it would strand.
//!
//! Undo puts `compositions.ron` back: the previous definition, or removal when the id was new.

use serde::{Deserialize, Serialize};
use serde_json::json;

use bevy_emerge_core::composition::Composition;

use crate::ops::{Ctx, Operation, Outcome, Refusal, Undo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// The group's name; forced to snake_case, as the Compose tab forces it.
    pub id: String,
    /// Ids of the open map's placements that become its members.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub from: Vec<String>,
    /// Redefine a composition that already has this id. Every stamp of it then expands to the new body.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replace: bool,
    /// Undo only: put this definition back whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore: Option<Composition>,
    /// Undo only: take the composition this operation made out again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub remove: bool,
}

impl Params {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "from"],
            "properties": {
                "id": {"type": "string", "description": "The group's name, snake_case. It is the id stamps refer to."},
                "from": {"type": "array", "items": {"type": "string"}, "minItems": 1, "description": "Ids of placements on the open map that become the group's members. A piece resting on another in the list stays on it; one resting on something outside the list is refused."},
                "replace": {"type": "boolean", "description": "Redefine a group that already has this id. Without it an existing id is refused and nothing is written."}
            }
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
        serde_json::from_value(raw.clone()).map_err(|e| {
            Refusal::new(format!("this is not a composition definition: {e}. The fields are `id`, `from` and `replace`."))
        })
    }

    fn undo_only(&self) -> bool {
        self.restore.is_some() || self.remove
    }
}

/// **Make a composition from the open map's placements.**
pub struct CompositionDefine;

fn state_json(ctx: &Ctx, id: &str) -> Option<serde_json::Value> {
    let project = ctx.project.as_ref()?;
    let def = project.compositions.compositions.iter().find(|c| c.id == id);
    Some(json!({"id": id, "def": def}))
}

impl CompositionDefine {
    fn go(ctx: &mut Ctx, params: &serde_json::Value, write: bool) -> Result<Option<Outcome>, Refusal> {
        let p = Params::parse(params)?;
        let id = bevy_emerge_core::naming::to_snake_case(&p.id);
        let (Some(project), Some(open)) = (ctx.project.as_mut(), ctx.open.as_ref()) else {
            return Err(Refusal::new("no map is open, so there are no placements to make a group from."));
        };
        let prior = project.compositions.compositions.iter().find(|c| c.id == id).cloned();

        let (summary, members) = if p.undo_only() {
            if !p.from.is_empty() || p.restore.is_some() && p.remove {
                return Err(Refusal::new("an undo either restores a definition or removes one, and names no placements."));
            }
            if let Some(c) = &p.restore {
                if c.id != id {
                    return Err(Refusal::new(format!("the definition to restore is `{}`, not `{id}`.", c.id)));
                }
                if write {
                    project.commit_composition(c.clone()).map_err(Refusal::new)?;
                }
                (format!("put group {id} back as it was"), c.members.len())
            } else {
                if write {
                    project.remove_composition(&id).map_err(Refusal::new)?;
                }
                (format!("took group {id} back out"), 0)
            }
        } else {
            let set = crate::editor::set_of_placements(project, &open.map, &p.from).map_err(Refusal::new)?;
            if prior.is_some() && !p.replace {
                return Err(Refusal::new(format!(
                    "a group named `{id}` already exists. Pass `replace: true` to redefine it, or pick another name. Nothing was written."
                )));
            }
            if write {
                crate::editor::keep_as_group(project, open, &set, &id, true).map_err(Refusal::new)?;
            } else {
                let comp = crate::editor::composition_from_set(&set, &id, &project.library, &project.compositions)
                    .map_err(Refusal::new)?;
                return Ok(Some(Outcome {
                    summary: format!("would define group {id} from {} piece(s)", comp.members.len()),
                    affected: comp.members.len(),
                    saved: false,
                    inverse: None,
                    after: None,
                }));
            }
            let n = set.pieces.len();
            (format!("defined group {id} from {n} piece(s)"), n)
        };

        if !write {
            return Ok(None);
        }
        let inverse = match prior {
            Some(c) => json!({"id": id, "restore": c}),
            None => json!({"id": id, "remove": true}),
        };
        let after = state_json(ctx, &id);
        Ok(Some(Outcome { summary, affected: members, saved: true, inverse: Some(inverse), after }))
    }
}

impl Operation for CompositionDefine {
    fn name(&self) -> &'static str {
        "composition_define"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        Params::parse(params).is_ok_and(|p| p.undo_only())
    }
    fn schema(&self) -> serde_json::Value {
        Params::schema()
    }
    fn state_of(&self, ctx: &Ctx, after: &serde_json::Value) -> Option<serde_json::Value> {
        state_json(ctx, after.get("id")?.as_str()?)
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let mut scratch = Ctx { project: ctx.project.clone(), open: ctx.open.clone(), ..Ctx::default() };
        Self::go(&mut scratch, params, false)?;
        Ok(params.clone())
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Ok(Self::go(ctx, params, true)?.expect("a write returns its outcome"))
    }
}
