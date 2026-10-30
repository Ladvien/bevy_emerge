//! **People on a map, as one registered operation** (#104): `person_set`.
//!
//! A person is a marker: a name, where they stand, a faction, and the buildings that are their home and
//! their workplace. Their behaviour is `bevy_society`'s, never the editor's; the marker is only what the
//! game needs to spawn them (`Home(entity)`, a faction membership). The file is
//! `<emerge_dir>/people/<map>.ron`, beside the map, so the game reads it and the editor reloads it.
//!
//! A home or workplace names a placement that is on the map, and **a placement that someone lives or works
//! in cannot be removed** (`tenant_of`, called by `placement_remove`): the refusal names the person.
//! With the `bevy_society` feature a faction must be one the project's own society definitions declare.
//!
//! The inverse is the same operation carrying the file as it was (`put_back`), which only an undo may send.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ops::{Ctx, Operation, Outcome, Refusal, Undo};

/// One person on the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    pub name: String,
    /// World `(x, z)` where they start the day.
    pub at: (f32, f32),
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    /// The id of the placement they live in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,
    /// The id of the placement they work in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work: Option<String>,
}

/// A map's people: `<emerge_dir>/people/<map>.ron`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct People {
    pub people: Vec<Person>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutBack {
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub map: String,
    /// The person's name. A new name adds a person (and needs `at`); a standing one is changed.
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub at: Option<(f32, f32)>,
    /// A faction id; `""` leaves the faction.
    #[serde(default)]
    pub faction: Option<String>,
    /// A placement id; `""` clears it.
    #[serde(default)]
    pub home: Option<String>,
    #[serde(default)]
    pub work: Option<String>,
    /// Take the person off the map.
    #[serde(default)]
    pub remove: bool,
    #[serde(default)]
    pub put_back: Option<PutBack>,
}

fn path_of(project: &crate::project::Project, map: &str) -> Result<std::path::PathBuf, Refusal> {
    if map.is_empty() || map.contains(['/', '\\', '.']) {
        return Err(Refusal::new(format!("`{map}` is not a map name. Nothing was written.")));
    }
    Ok(project.emerge_dir.join("people").join(format!("{map}.ron")))
}

/// **The saved people of a map**, nobody when there is no file.
pub fn people_of(project: &crate::project::Project, map: &str) -> People {
    path_of(project, map)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| ron::from_str(&t).ok())
        .unwrap_or_default()
}

/// **Who lives or works in one of `ids`**: `(person, "home" | "work", placement)`, the first found.
pub fn tenant_of(project: &crate::project::Project, map: &str, ids: &[String]) -> Option<(String, &'static str, String)> {
    people_of(project, map).people.into_iter().find_map(|p| {
        if let Some(h) = p.home.as_ref().filter(|h| ids.contains(h)) {
            return Some((p.name.clone(), "home", h.clone()));
        }
        p.work.as_ref().filter(|w| ids.contains(w)).map(|w| (p.name.clone(), "work", w.clone()))
    })
}

#[cfg(feature = "bevy_society")]
fn check_faction(ctx: &Ctx, faction: &str) -> Result<(), Refusal> {
    let Ok(path) = crate::ops_family::settings_path(ctx, "bevy_society") else { return Ok(()) };
    let Ok(text) = std::fs::read_to_string(&path) else { return Ok(()) };
    let Ok(file) = ron::from_str::<bevy_society::DefsFile>(&text) else { return Ok(()) };
    if file.factions.iter().any(|f| f.id == faction) {
        return Ok(());
    }
    let known: Vec<&str> = file.factions.iter().map(|f| f.id.as_str()).collect();
    Err(Refusal::new(format!("the society definitions declare no faction `{faction}` (they declare: {}). Nothing was changed.", known.join(", "))))
}

#[cfg(not(feature = "bevy_society"))]
fn check_faction(_ctx: &Ctx, _faction: &str) -> Result<(), Refusal> {
    Ok(())
}

fn building(ctx: &Ctx, what: &str, id: &str) -> Result<(), Refusal> {
    let Some(open) = ctx.open.as_ref() else {
        return Err(Refusal::new("no map is open, so there is no building to name."));
    };
    if open.map.placements.iter().any(|p| p.id == id) {
        Ok(())
    } else {
        Err(Refusal::new(format!("no placement is called `{id}` on this map, so it cannot be a {what}. Nothing was changed.")))
    }
}

/// **Author a person.**
pub struct PersonSet;

impl PersonSet {
    fn apply(ctx: &Ctx, raw: &serde_json::Value, write: bool) -> Result<Outcome, Refusal> {
        let p: Params = serde_json::from_value(raw.clone()).map_err(|e| {
            Refusal::new(format!("this is not a person: {e}. The fields are `map`, `name`, `at`, `faction`, `home`, `work` and `remove`."))
        })?;
        let Some(project) = ctx.project.as_ref() else {
            return Err(Refusal::new("no project is open, so there is nobody to place."));
        };
        let path = path_of(project, &p.map)?;
        let before = std::fs::read_to_string(&path).ok();
        let inverse = json!({"map": p.map, "put_back": {"text": before}});

        let (text, summary) = if let Some(back) = &p.put_back {
            (back.text.clone(), format!("people {}: put back", p.map))
        } else {
            let Some(name) = p.name.as_deref().filter(|n| !n.trim().is_empty()) else {
                return Err(Refusal::new("a person needs a `name`. Nothing was changed."));
            };
            let mut people = people_of(project, &p.map);
            let at_row = people.people.iter().position(|q| q.name == name);
            let summary = if p.remove {
                let Some(i) = at_row else {
                    return Err(Refusal::new(format!("nobody on `{}` is called `{name}`. Nothing was changed.", p.map)));
                };
                people.people.remove(i);
                format!("{name} left {}", p.map)
            } else {
                let mut person = match at_row {
                    Some(i) => people.people[i].clone(),
                    None => match p.at {
                        Some(at) => Person { name: name.to_owned(), at, faction: None, home: None, work: None },
                        None => return Err(Refusal::new(format!("`{name}` is new, so they need an `at` (x, z). Nothing was changed."))),
                    },
                };
                if let Some(at) = p.at {
                    person.at = at;
                }
                if let Some(f) = &p.faction {
                    if f.is_empty() {
                        person.faction = None;
                    } else {
                        check_faction(ctx, f)?;
                        person.faction = Some(f.clone());
                    }
                }
                for (slot, what, value) in [(&mut person.home, "home", &p.home), (&mut person.work, "workplace", &p.work)] {
                    match value.as_deref() {
                        Some("") => *slot = None,
                        Some(id) => {
                            building(ctx, what, id)?;
                            *slot = Some(id.to_owned());
                        }
                        None => {}
                    }
                }
                let summary = format!("{} {name}", if at_row.is_some() { "changed" } else { "added" });
                match at_row {
                    Some(i) => people.people[i] = person,
                    None => people.people.push(person),
                }
                summary
            };
            let text = ron::ser::to_string_pretty(&people, ron::ser::PrettyConfig::default())
                .map_err(|e| Refusal::new(format!("the people could not be written: {e}")))?;
            (Some(text), summary)
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
        Ok(Outcome { summary, affected: 1, saved: write, inverse: Some(inverse), after: Some(json!({"map": p.map, "text": text})) })
    }
}

impl Operation for PersonSet {
    fn name(&self) -> &'static str {
        "person_set"
    }
    fn undo(&self) -> Undo {
        Undo::Inverse
    }
    fn only_as_undo(&self, params: &serde_json::Value) -> bool {
        params.get("put_back").is_some()
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["map"],
            "properties": {
                "map": {"type": "string", "description": "The map the person lives on."},
                "name": {"type": "string", "description": "The person's name. A new name adds a person and needs `at`; a standing one is changed."},
                "at": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "World [x, z] where they start the day."},
                "faction": {"type": "string", "description": "A faction the project's society definitions declare; \"\" leaves it."},
                "home": {"type": "string", "description": "The id of the placement they live in; \"\" clears it. That placement cannot be removed while they live there."},
                "work": {"type": "string", "description": "The id of the placement they work in; \"\" clears it."},
                "remove": {"type": "boolean", "description": "Take the named person off the map."},
                "put_back": {"type": "object", "description": "An undo's put-back. Only an undo sends this."}
            }
        })
    }
    fn state_of(&self, ctx: &Ctx, after: &serde_json::Value) -> Option<serde_json::Value> {
        let map = after.get("map")?.as_str()?;
        let path = path_of(ctx.project.as_ref()?, map).ok()?;
        Some(json!({"map": map, "text": std::fs::read_to_string(path).ok()}))
    }
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
        let o = Self::apply(ctx, params, false)?;
        Ok(json!({"change": o.summary}))
    }
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
        Self::apply(ctx, params, true)
    }
}
