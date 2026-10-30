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

/// **Give a composition an anchored or a bounded envelope** (E-072 slice 3): the one field of a saved
/// group that a person and an agent could not change after it was kept. `bounded` without a `size`
/// fits the tile to the members, as the Tiles tab does (`build::fit_envelope`: whole cells across,
/// the project lattice's cell height up). The result goes through `composition::validate`, so a
/// bounded size that leaves a member outside it is refused and nothing is written.
///
/// Undo is the same operation with the previous envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeParams {
    /// The composition, by id.
    pub id: String,
    /// `anchored` or `bounded`.
    pub envelope: EnvelopeKind,
    /// `(width, height, depth)` in metres, for `bounded` only. Omitted: fitted to the members.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<(f32, f32, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeKind {
    Anchored,
    Bounded,
}

impl EnvelopeParams {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "envelope"],
            "properties": {
                "id": {"type": "string", "description": "The group, by id."},
                "envelope": {"type": "string", "enum": ["anchored", "bounded"], "description": "`bounded` fills a tile-shaped region and so can be a solver prototype; `anchored` claims no tile."},
                "size": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Bounded only: [width, height, depth] in metres. Omitted, the tile is fitted to the members (whole cells across, the lattice's cell height up)."}
            }
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<Self, Refusal> {
        let p: Self = serde_json::from_value(raw.clone()).map_err(|e| {
            Refusal::new(format!("this is not an envelope change: {e}. The fields are `id`, `envelope` and `size`."))
        })?;
        if p.envelope == EnvelopeKind::Anchored && p.size.is_some() {
            return Err(Refusal::new("an anchored group has no size; give `size` only with `bounded`."));
        }
        Ok(p)
    }
}

pub struct CompositionSetEnvelope;

impl CompositionSetEnvelope {
    fn go(ctx: &mut Ctx, params: &serde_json::Value, write: bool) -> Result<Outcome, Refusal> {
        use bevy_emerge_core::composition::Envelope;
        let p = EnvelopeParams::parse(params)?;
        let Some(project) = ctx.project.as_mut() else {
            return Err(Refusal::new("no project is open."));
        };
        let Some(prior) = project.compositions.compositions.iter().find(|c| c.id == p.id).cloned() else {
            let known: Vec<&str> = project.compositions.compositions.iter().map(|c| c.id.as_str()).collect();
            return Err(Refusal::new(format!("no group `{}`. The groups are: {}.", p.id, known.join(", "))));
        };
        let envelope = match p.envelope {
            EnvelopeKind::Anchored => Envelope::Anchored,
            EnvelopeKind::Bounded => {
                let fit = crate::build::fit_envelope(&prior.members, &project.library, project.lattice.cell_height);
                let size = p.size.unwrap_or(fit);
                // **A tile claims at least what its members reach**, or a stamp would seat other
                // tiles on top of them; the Tiles tab refits to this same floor.
                if size.0 + 1e-4 < fit.0 || size.2 + 1e-4 < fit.2 {
                    return Err(Refusal::new(format!(
                        "a bounded `{}` of {:.2} x {:.2} m leaves members outside it; its members need at least {:.2} x {:.2} m. Nothing was written.",
                        p.id, size.0, size.2, fit.0, fit.2
                    )));
                }
                Envelope::Bounded { size }
            }
        };
        let mut comp = prior.clone();
        comp.envelope = envelope;
        let mut proposed = project.compositions.clone();
        if let Some(slot) = proposed.compositions.iter_mut().find(|c| c.id == p.id) {
            *slot = comp.clone();
        }
        bevy_emerge_core::composition::validate(&proposed.compositions, &project.library).map_err(Refusal::new)?;
        if write {
            project.commit_composition(comp.clone()).map_err(Refusal::new)?;
        }
        let describe = |e: Envelope| match e {
            Envelope::Anchored => "anchored".to_owned(),
            Envelope::Bounded { size } => format!("bounded {:.2}x{:.2}x{:.2}", size.0, size.1, size.2),
        };
        let inverse = match prior.envelope {
            Envelope::Anchored => json!({"id": p.id, "envelope": "anchored"}),
            Envelope::Bounded { size } => json!({"id": p.id, "envelope": "bounded", "size": size}),
        };
        Ok(Outcome {
            summary: format!("group {} is now {} (was {})", p.id, describe(envelope), describe(prior.envelope)),
            affected: 1,
            saved: write,
            inverse: write.then_some(inverse),
            after: state_json(ctx, &p.id),
        })
    }
}

impl Operation for CompositionSetEnvelope {
    fn name(&self) -> &'static str {
        "composition_set_envelope"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn schema(&self) -> serde_json::Value {
        EnvelopeParams::schema()
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
        Self::go(ctx, params, true)
    }
}
