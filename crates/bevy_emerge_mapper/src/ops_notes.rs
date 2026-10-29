//! **Notes for agents, as registered operations** (#89): `note_add`, `note_edit`, `note_remove`.
//!
//! A note is data beside the map it marks (`bevy_emerge_core::notes`, the `.notes.ron` next to the
//! `.map.ron`). These three operations are the only way the editor changes it, so a person's click
//! and an agent's `apply` are one change with one log line and one undo.
//!
//! Each operation's inverse is the same operation sent a parameter only an undo may send
//! (`take_back`, `put_back`), the way `placement_remove` puts a piece back. A plan is the commit run
//! on a copy that is never written, so a preview cannot disagree with what a dispatch does.

use bevy_emerge_core::notes::{self, Note, Notes};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ops::{Ctx, Operation, Outcome, Refusal, Undo};

/// **A note put back where it stood**, for the undo of a removal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutBack {
    pub index: usize,
    pub note: Note,
}

fn notes_path(ctx: &Ctx) -> Result<std::path::PathBuf, Refusal> {
    let Some(open) = ctx.open.as_ref() else {
        return Err(Refusal::new("no map is open, so there is nowhere to leave a note."));
    };
    Ok(notes::path_for_map(&open.map_path))
}

/// **The notes of the open map and the map's name**, which is the state an undo checks.
fn state_of_open(ctx: &Ctx) -> Option<serde_json::Value> {
    let open = ctx.open.as_ref()?;
    let notes = Notes::load(&notes::path_for_map(&open.map_path)).ok()?;
    Some(json!({"map": open.map.name, "notes": serde_json::to_value(&notes).ok()?}))
}

/// What one edit to the notes gives back: the receipt line, the inverse, and how many notes it touched.
struct Change {
    summary: String,
    inverse: serde_json::Value,
}

/// **Run one change on the open map's notes.** `write` false is the plan: the same code on the same
/// notes, and nothing is saved.
fn run(
    ctx: &mut Ctx,
    write: bool,
    change: impl FnOnce(&mut Notes, &bevy_emerge_core::map::Map) -> Result<Change, Refusal>,
) -> Result<Option<Outcome>, Refusal> {
    let path = notes_path(ctx)?;
    let mut notes = Notes::load(&path).map_err(Refusal::new)?;
    let open = ctx.open.as_ref().expect("notes_path checked the map");
    let done = change(&mut notes, &open.map)?;
    if !write {
        return Ok(None);
    }
    notes.save(&path).map_err(|e| Refusal::new(format!("the notes file refused the change, so nothing was written: {e}")))?;
    Ok(Some(Outcome {
        summary: done.summary,
        affected: 1,
        saved: true,
        inverse: Some(done.inverse),
        after: state_of_open(ctx),
    }))
}

/// `by_add` says which operation is putting it back, because an inverse is dispatched under the
/// operation that logged it: `note_add` is undone with `take_back`, `note_remove` with `id`.
fn put_back(notes: &mut Notes, pb: &PutBack, by_add: bool) -> Result<Change, Refusal> {
    if notes.get(&pb.note.id).is_some() {
        return Err(Refusal::new(format!("note {} is already on the map, so it was not put back.", pb.note.id)));
    }
    let at = pb.index.min(notes.notes.len());
    let id = pb.note.id.clone();
    notes.notes.insert(at, pb.note.clone());
    let inverse = if by_add { json!({"take_back": id}) } else { json!({"id": id}) };
    Ok(Change { summary: format!("put back note {id}"), inverse })
}

fn take_out(notes: &mut Notes, id: &str, what: &str) -> Result<Change, Refusal> {
    let index = notes
        .notes
        .iter()
        .position(|n| n.id == id)
        .ok_or_else(|| Refusal::new(format!("no note {id} on this map, so nothing was {what}.")))?;
    let note = notes.remove(id).map_err(Refusal::new)?;
    Ok(Change { summary: format!("{what} note {id}"), inverse: json!({"put_back": PutBack { index, note }}) })
}

fn bad_params(what: &str, fields: &str, e: serde_json::Error) -> Refusal {
    Refusal::new(format!("these are not {what}: {e}. The fields are {fields}, and nothing else."))
}

// ---------------------------------------------------------------- note_add

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddParams {
    /// Inclusive cell rectangle `[min_x, min_z, max_x, max_z]` the note is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cells: Option<[i32; 4]>,
    /// Placement ids the note is about, instead of a rectangle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pieces: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// The camera pose when written: `[x, y, z, yaw, pitch]`. Zeros when the writer has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<[f32; 5]>,
    /// Undo only: take a note this operation added back off the map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take_back: Option<String>,
    /// Undo only: put a note this operation's undo took off back where it stood.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put_back: Option<PutBack>,
}

impl AddParams {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["text"],
            "properties": {
                "cells": {
                    "type": "array", "items": {"type": "integer"}, "minItems": 4, "maxItems": 4,
                    "description": "The cell rectangle the note is about, inclusive: min_x, min_z, max_x, max_z. Give this or `pieces`."
                },
                "pieces": {
                    "type": "array", "items": {"type": "string"},
                    "description": "Ids of the placements the note is about. Give this or `cells`."
                },
                "text": {"type": "string", "description": "The instruction, for an agent, about that place."},
                "camera": {
                    "type": "array", "items": {"type": "number"}, "minItems": 5, "maxItems": 5,
                    "description": "The camera pose when written: x, y, z, yaw, pitch. Optional."
                }
            }
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<AddParams, Refusal> {
        serde_json::from_value(raw.clone())
            .map_err(|e| bad_params("a note", "`cells` or `pieces`, `text` and `camera`", e))
    }

    fn undo_only(&self) -> bool {
        self.take_back.is_some() || self.put_back.is_some()
    }
}

/// **Leave a draft note on a cell rectangle or on pieces.**
pub struct NoteAdd;

impl NoteAdd {
    fn apply(ctx: &mut Ctx, params: &serde_json::Value, write: bool) -> Result<Option<Outcome>, Refusal> {
        let p = AddParams::parse(params)?;
        run(ctx, write, |notes, map| {
            if let Some(id) = &p.take_back {
                return take_out(notes, id, "took back");
            }
            if let Some(pb) = &p.put_back {
                return put_back(notes, pb, true);
            }
            let camera = p.camera.map_or((0.0, 0.0, 0.0, 0.0, 0.0), |c| (c[0], c[1], c[2], c[3], c[4]));
            let cell = bevy_emerge_core::grid::TILE;
            let id = match (&p.cells, &p.pieces) {
                (Some(c), None) => notes.add_area(map, (c[0], c[1], c[2], c[3]), cell, camera, &p.text),
                (None, Some(ids)) => notes.add_pieces(map, ids, cell, camera, &p.text),
                _ => return Err(Refusal::new("a note is about `cells` or about `pieces`, and not both. Nothing was added.")),
            }
            .map_err(Refusal::new)?;
            Ok(Change { summary: format!("left note {id}"), inverse: json!({"take_back": id}) })
        })
    }
}

impl Operation for NoteAdd {
    fn name(&self) -> &'static str {
        "note_add"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        AddParams::parse(params).is_ok_and(|p| p.undo_only())
    }
    fn schema(&self) -> serde_json::Value {
        AddParams::schema()
    }
    fn state(&self, ctx: &Ctx) -> Option<serde_json::Value> {
        state_of_open(ctx)
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let mut scratch = Ctx { open: ctx.open.clone(), ..Ctx::default() };
        Self::apply(&mut scratch, params, false)?;
        Ok(params.clone())
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Ok(Self::apply(ctx, params, true)?.expect("a write returns its outcome"))
    }
}

// --------------------------------------------------------------- note_edit

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditParams {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub text: String,
    /// Undo only: the note as it stood before the edit, put back whole so its history is not
    /// lengthened by the undo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore: Option<Note>,
}

impl EditParams {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "text"],
            "properties": {
                "id": {"type": "string", "description": "The note's id."},
                "text": {"type": "string", "description": "The new text. A resolved note is a record and is not edited."}
            }
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<EditParams, Refusal> {
        let p: EditParams = serde_json::from_value(raw.clone()).map_err(|e| bad_params("a note edit", "`id` and `text`", e))?;
        if p.restore.is_none() && (p.id.is_empty() || p.text.is_empty()) {
            return Err(Refusal::new("a note edit needs the note's `id` and its new `text`. Nothing was edited."));
        }
        Ok(p)
    }
}

/// **Change a draft or open note's text.**
pub struct NoteEdit;

impl NoteEdit {
    fn apply(ctx: &mut Ctx, params: &serde_json::Value, write: bool) -> Result<Option<Outcome>, Refusal> {
        let p = EditParams::parse(params)?;
        run(ctx, write, |notes, _| {
            if let Some(old) = &p.restore {
                let at = notes
                    .notes
                    .iter()
                    .position(|n| n.id == old.id)
                    .ok_or_else(|| Refusal::new(format!("note {} is gone, so the edit was not taken back.", old.id)))?;
                notes.notes[at] = old.clone();
                return Ok(Change { summary: format!("took back the edit of note {}", old.id), inverse: json!({}) });
            }
            let before = notes.get(&p.id).ok_or_else(|| Refusal::new(format!("no note {} on this map, so nothing was edited.", p.id)))?.clone();
            notes.edit(&p.id, &p.text).map_err(Refusal::new)?;
            Ok(Change { summary: format!("edited note {}", p.id), inverse: json!({"restore": before}) })
        })
    }
}

impl Operation for NoteEdit {
    fn name(&self) -> &'static str {
        "note_edit"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        EditParams::parse(params).is_ok_and(|p| p.restore.is_some())
    }
    fn schema(&self) -> serde_json::Value {
        EditParams::schema()
    }
    fn state(&self, ctx: &Ctx) -> Option<serde_json::Value> {
        state_of_open(ctx)
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let mut scratch = Ctx { open: ctx.open.clone(), ..Ctx::default() };
        Self::apply(&mut scratch, params, false)?;
        Ok(params.clone())
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Ok(Self::apply(ctx, params, true)?.expect("a write returns its outcome"))
    }
}

// ------------------------------------------------------------- note_remove

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Undo only: the note a removal took, and the place it held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put_back: Option<PutBack>,
}

impl RemoveParams {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["id"],
            "properties": {"id": {"type": "string", "description": "The note's id. Its sign comes off the map."}}
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<RemoveParams, Refusal> {
        let p: RemoveParams = serde_json::from_value(raw.clone())
            .map_err(|e| bad_params("a note removal", "`id`", e))?;
        if p.id.is_some() == p.put_back.is_some() {
            return Err(Refusal::new("a removal names one note by `id`. Nothing was removed."));
        }
        Ok(p)
    }
}

/// **Take a note off the map.**
pub struct NoteRemove;

impl NoteRemove {
    fn apply(ctx: &mut Ctx, params: &serde_json::Value, write: bool) -> Result<Option<Outcome>, Refusal> {
        let p = RemoveParams::parse(params)?;
        run(ctx, write, |notes, _| match (&p.id, &p.put_back) {
            (Some(id), _) => take_out(notes, id, "removed"),
            (None, Some(pb)) => put_back(notes, pb, false),
            (None, None) => unreachable!("parse refused it"),
        })
    }
}

impl Operation for NoteRemove {
    fn name(&self) -> &'static str {
        "note_remove"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        RemoveParams::parse(params).is_ok_and(|p| p.put_back.is_some())
    }
    fn schema(&self) -> serde_json::Value {
        RemoveParams::schema()
    }
    fn state(&self, ctx: &Ctx) -> Option<serde_json::Value> {
        state_of_open(ctx)
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let mut scratch = Ctx { open: ctx.open.clone(), ..Ctx::default() };
        Self::apply(&mut scratch, params, false)?;
        Ok(params.clone())
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Ok(Self::apply(ctx, params, true)?.expect("a write returns its outcome"))
    }
}

// --------------------------------------------------------------- note_send

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Undo only: the notes as they stood before the send, put back whole, history included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore: Option<Vec<Note>>,
}

impl SendParams {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {"id": {"type": "string", "description": "The draft to send. Omit it to send every draft on the map."}}
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<SendParams, Refusal> {
        serde_json::from_value(raw.clone()).map_err(|e| bad_params("a note send", "`id`, or nothing to send every draft", e))
    }
}

/// **Send one draft, or every draft, so an agent may claim them.** One undo step returns them to drafts.
pub struct NoteSend;

impl NoteSend {
    fn apply(ctx: &mut Ctx, params: &serde_json::Value, write: bool) -> Result<Option<Outcome>, Refusal> {
        let p = SendParams::parse(params)?;
        run(ctx, write, |notes, _| {
            if let Some(before) = &p.restore {
                for old in before {
                    let at = notes
                        .notes
                        .iter()
                        .position(|n| n.id == old.id)
                        .ok_or_else(|| Refusal::new(format!("note {} is gone, so the send was not taken back.", old.id)))?;
                    notes.notes[at] = old.clone();
                }
                return Ok(Change { summary: "took back a send".to_owned(), inverse: json!({}) });
            }
            let sent = match &p.id {
                Some(id) => {
                    let before = notes.get(id).ok_or_else(|| Refusal::new(format!("no note {id} on this map, so nothing was sent.")))?.clone();
                    notes.send(id).map_err(Refusal::new)?;
                    (vec![before], vec![id.clone()])
                }
                None => {
                    let before: Vec<Note> = notes.notes.iter().filter(|n| n.is_draft()).cloned().collect();
                    let ids = notes.send_all().map_err(Refusal::new)?;
                    (before, ids)
                }
            };
            Ok(Change {
                summary: format!("sent {}", sent.1.join(", ")),
                inverse: json!({"restore": sent.0}),
            })
        })
    }
}

impl Operation for NoteSend {
    fn name(&self) -> &'static str {
        "note_send"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        SendParams::parse(params).is_ok_and(|p| p.restore.is_some())
    }
    fn schema(&self) -> serde_json::Value {
        SendParams::schema()
    }
    fn state(&self, ctx: &Ctx) -> Option<serde_json::Value> {
        state_of_open(ctx)
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let mut scratch = Ctx { open: ctx.open.clone(), ..Ctx::default() };
        Self::apply(&mut scratch, params, false)?;
        Ok(params.clone())
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Ok(Self::apply(ctx, params, true)?.expect("a write returns its outcome"))
    }
}
