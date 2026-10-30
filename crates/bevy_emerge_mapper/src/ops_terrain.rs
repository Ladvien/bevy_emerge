//! **Sculpting the map's ground, as one registered operation** (#99): `terrain_stroke`.
//!
//! The heightfield is `<emerge_dir>/terrain/<map>.ron`, the engine-free `bevy_emerge_core::heightfield`
//! type as RON, saved beside the map. A stroke is a gesture: its points arrive whole and make one
//! change and one undo. The inverse is the same operation carrying the heights the stroke replaced
//! (`restore`), which only an undo may send.

use bevy_emerge_core::heightfield::{Heightfield, Stroke};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ops::{Ctx, Operation, Outcome, Refusal, Undo};

/// A flat field to start a map's terrain from, when it has none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub origin: (f32, f32),
    pub cell_m: f32,
    pub width: usize,
    pub depth: usize,
    pub height_m: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub map: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Stroke>,
    /// Make the field first, flat, when the map has none. Ignored when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create: Option<Create>,
    /// `(cell index, height)` pairs to put back: an undo's inverse. Not a thing a caller sends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore: Option<Vec<(usize, f32)>>,
}

impl Params {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["map"],
            "properties": {
                "map": {"type": "string", "description": "The map whose terrain is sculpted."},
                "stroke": {"type": "object", "description": "One gesture: {brush, radius_m, strength, points: [[x, z], ...]}. brush is \"Raise\", \"Lower\", \"Smooth\", {\"Flatten\": metres} or {\"Noise\": {seed, scale_m}}."},
                "create": {"type": "object", "description": "Start the map's terrain flat when it has none: {origin: [x, z], cell_m, width, depth, height_m}."},
                "restore": {"type": "array", "description": "Heights an undo puts back, as [cell index, metres] pairs. Only an undo sends this."}
            }
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
        let p: Params = serde_json::from_value(raw.clone()).map_err(|e| {
            Refusal::new(format!("this is not a terrain stroke: {e}. The fields are `map`, `stroke` and `create`, and nothing else."))
        })?;
        if p.stroke.is_none() && p.restore.is_none() {
            return Err(Refusal::new("a terrain stroke needs a `stroke`. Nothing was changed."));
        }
        Ok(p)
    }
}

fn path_of(ctx: &Ctx, map: &str) -> Result<std::path::PathBuf, Refusal> {
    let Some(project) = ctx.project.as_ref() else {
        return Err(Refusal::new("no project is open, so there is no terrain to sculpt."));
    };
    if map.is_empty() || map.contains(['/', '\\', '.']) {
        return Err(Refusal::new(format!("`{map}` is not a map name. Nothing was written.")));
    }
    Ok(project.emerge_dir.join("terrain").join(format!("{map}.ron")))
}

fn read(ctx: &Ctx, map: &str) -> Option<String> {
    std::fs::read_to_string(path_of(ctx, map).ok()?).ok()
}

fn to_text(field: &Heightfield) -> Result<String, Refusal> {
    ron::ser::to_string_pretty(field, ron::ser::PrettyConfig::default()).map_err(|e| Refusal::new(format!("the terrain could not be written: {e}")))
}

/// **The saved terrain of the open map**, or `None` for flat ground (no file, or one that is not a
/// heightfield). The map's name is its file name without `.map.ron`, the name `terrain_stroke` takes.
pub fn ground_of(project: &crate::project::Project, open: &crate::project::OpenMap) -> Option<Heightfield> {
    let name = open.map_path.file_name()?.to_str()?.strip_suffix(".map.ron")?;
    let text = std::fs::read_to_string(project.emerge_dir.join("terrain").join(format!("{name}.ron"))).ok()?;
    ron::from_str(&text).ok()
}

/// **Where every piece of the open map stands, on its terrain**: [`bevy_emerge_core::stack::resolve_y_on`]
/// over [`ground_of`], so sculpting lifts what stands on the ground.
pub fn heights_on_ground(project: &crate::project::Project, open: &crate::project::OpenMap) -> Result<Vec<f32>, String> {
    bevy_emerge_core::stack::resolve_y_on(&open.map, &project.library, ground_of(project, open).as_ref())
}

/// **Sculpt a map's heightfield with one stroke.**
pub struct TerrainStroke;

impl TerrainStroke {
    fn apply(ctx: &Ctx, params: &serde_json::Value, write: bool) -> Result<Outcome, Refusal> {
        let p = Params::parse(params)?;
        let path = path_of(ctx, &p.map)?;
        let mut field: Heightfield = match std::fs::read_to_string(&path) {
            Ok(text) => ron::from_str(&text).map_err(|e| Refusal::new(format!("{} is not a heightfield: {e}", path.display())))?,
            Err(_) => match &p.create {
                Some(c) if c.width >= 2 && c.depth >= 2 && c.cell_m > 0.0 => {
                    Heightfield::flat(c.origin, c.cell_m, c.width, c.depth, c.height_m)
                }
                Some(_) => return Err(Refusal::new("a terrain needs at least 2x2 cells of positive size.")),
                None => {
                    return Err(Refusal::new(format!(
                        "map `{}` has no terrain yet. Send `create` with the stroke to start one flat.",
                        p.map
                    )))
                }
            },
        };
        let (affected, inverse) = if let Some(cells) = &p.restore {
            let mut before = Vec::with_capacity(cells.len());
            for &(i, h) in cells {
                let slot = field.heights.get_mut(i).ok_or_else(|| Refusal::new(format!("cell {i} is outside the terrain.")))?;
                before.push((i, *slot));
                *slot = h;
            }
            (cells.len(), json!({"map": p.map, "restore": before}))
        } else {
            let stroke = p.stroke.as_ref().expect("parse needs a stroke or a restore");
            let changed = field.apply(stroke);
            let back: Vec<(usize, f32)> = changed.0.iter().map(|&(i, b, _)| (i, b)).collect();
            (changed.0.len(), json!({"map": p.map, "restore": back}))
        };
        let text = to_text(&field)?;
        if write {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| Refusal::new(format!("{} cannot be made: {e}", dir.display())))?;
            }
            std::fs::write(&path, &text)
                .map_err(|e| Refusal::new(format!("{} refused the change, so nothing was written: {e}", path.display())))?;
        }
        Ok(Outcome {
            summary: format!("terrain {}: {affected} cells changed", p.map),
            affected,
            saved: write,
            inverse: Some(inverse),
            after: Some(json!({"map": p.map, "text": text})),
        })
    }
}

impl Operation for TerrainStroke {
    fn name(&self) -> &'static str {
        "terrain_stroke"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        Params::parse(params).map(|p| p.restore.is_some()).unwrap_or(false)
    }
    fn schema(&self) -> serde_json::Value {
        Params::schema()
    }
    fn state_of(&self, ctx: &Ctx, after: &serde_json::Value) -> Option<serde_json::Value> {
        let map = after.get("map")?.as_str()?;
        Some(json!({"map": map, "text": read(ctx, map)?}))
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let o = Self::apply(ctx, params, false)?;
        Ok(json!({"map": Params::parse(params)?.map, "cells": o.affected}))
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Self::apply(ctx, params, true)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutBack {
    /// The terrain file as it was (`None`: there was none, so putting back deletes it).
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportParams {
    pub map: String,
    /// A 16-bit greyscale PNG height map, relative to the project root or absolute.
    #[serde(default)]
    pub file: Option<String>,
    /// World `(x, z)` of the map's first cell. Default `(0, 0)`.
    #[serde(default)]
    pub origin: Option<(f32, f32)>,
    /// Metres between cells.
    #[serde(default)]
    pub cell_m: Option<f32>,
    /// Metres at sample `0` and at sample `65535`.
    #[serde(default)]
    pub range_m: Option<(f32, f32)>,
    /// An undo's put-back. Not a thing a caller sends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put_back: Option<PutBack>,
}

impl ImportParams {
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["map"],
            "properties": {
                "map": {"type": "string", "description": "The map whose terrain is replaced by the height map."},
                "file": {"type": "string", "description": "A 16-bit greyscale PNG height map, relative to the project root or absolute."},
                "origin": {"type": "array", "description": "World [x, z] of the first cell. Default [0, 0]."},
                "cell_m": {"type": "number", "description": "Metres between cells."},
                "range_m": {"type": "array", "description": "[metres at sample 0, metres at sample 65535]."},
                "put_back": {"type": "object", "description": "An undo's put-back. Only an undo sends this."}
            }
        })
    }

    pub fn parse(raw: &serde_json::Value) -> Result<ImportParams, Refusal> {
        let p: ImportParams = serde_json::from_value(raw.clone()).map_err(|e| {
            Refusal::new(format!("this is not a terrain import: {e}. The fields are `map`, `file`, `origin`, `cell_m` and `range_m`."))
        })?;
        if p.put_back.is_none() && (p.file.is_none() || p.cell_m.is_none() || p.range_m.is_none()) {
            return Err(Refusal::new("a terrain import needs `file`, `cell_m` and `range_m`. Nothing was changed."));
        }
        Ok(p)
    }
}

/// **Replace a map's heightfield with an imported 16-bit height map.** Sample `0` is `range_m.0`, `65535` is `range_m.1`.
pub struct TerrainImport;

impl TerrainImport {
    fn apply(ctx: &Ctx, params: &serde_json::Value, write: bool) -> Result<Outcome, Refusal> {
        let p = ImportParams::parse(params)?;
        let path = path_of(ctx, &p.map)?;
        let before = std::fs::read_to_string(&path).ok();
        let inverse = json!({"map": p.map, "put_back": {"text": before}});
        let (text, affected) = if let Some(back) = &p.put_back {
            (back.text.clone(), 0)
        } else {
            let project = ctx.project.as_ref().expect("path_of needs a project");
            let file = project.root.join(p.file.as_deref().expect("parse needs a file"));
            let img = image::open(&file).map_err(|e| Refusal::new(format!("{} is not a readable image: {e}", file.display())))?;
            if !matches!(img, image::DynamicImage::ImageLuma16(_)) {
                return Err(Refusal::new(format!("{} is not a 16-bit greyscale PNG, so it is not a height map.", file.display())));
            }
            let luma = img.into_luma16();
            let (w, d) = (luma.width() as usize, luma.height() as usize);
            let field = Heightfield::from_u16(
                p.origin.unwrap_or((0.0, 0.0)),
                p.cell_m.expect("parse needs cell_m"),
                w,
                d,
                luma.as_raw(),
                p.range_m.expect("parse needs range_m"),
            )
            .map_err(|e| Refusal::new(format!("the height map cannot be used: {e:?}")))?;
            (Some(to_text(&field)?), w * d)
        };
        if write {
            match &text {
                Some(t) => {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir).map_err(|e| Refusal::new(format!("{} cannot be made: {e}", dir.display())))?;
                    }
                    std::fs::write(&path, t)
                        .map_err(|e| Refusal::new(format!("{} refused the change, so nothing was written: {e}", path.display())))?;
                }
                None => {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        Ok(Outcome {
            summary: format!("terrain {}: imported {affected} cells", p.map),
            affected,
            saved: write,
            inverse: Some(inverse),
            after: Some(json!({"map": p.map, "text": text})),
        })
    }
}

impl Operation for TerrainImport {
    fn name(&self) -> &'static str {
        "terrain_import"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        ImportParams::parse(params).map(|p| p.put_back.is_some()).unwrap_or(false)
    }
    fn schema(&self) -> serde_json::Value {
        ImportParams::schema()
    }
    fn state_of(&self, ctx: &Ctx, after: &serde_json::Value) -> Option<serde_json::Value> {
        let map = after.get("map")?.as_str()?;
        Some(json!({"map": map, "text": read(ctx, map)}))
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let o = Self::apply(ctx, params, false)?;
        Ok(json!({"map": ImportParams::parse(params)?.map, "cells": o.affected}))
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Self::apply(ctx, params, true)
    }
}
