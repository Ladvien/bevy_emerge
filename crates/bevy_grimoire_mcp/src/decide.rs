//! **`grimoire_decide`'s logic, apart from the transport and the editor**, so a test drives it with a
//! recorded response and no model, no editor and no network.
//!
//! Two forms: a **named check** from `checks.ron` (the editor's built-ins, merged with the project's own
//! `assets/grimoire/checks.ron`, read on every call so an edit takes effect at once), and **ad-hoc typed
//! questions**. Images are files under the project; the one picture the server can take itself is `view`,
//! the editor's current picture.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy_grimoire_decide::{Check, Checks, Evidence, Question, Request, Transport};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The name of the picture the server takes itself.
pub const VIEW: &str = "view";

/// **The parameters of `grimoire_decide`.**
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecideParams {
    /// **A named check**, such as `floating`. Give this or `questions`, not both.
    #[serde(default)]
    pub check: Option<String>,
    /// **Ad-hoc typed questions**, by id: `{"type": "noul", "instructions": "..."}`, `{"type": "choice",
    /// "instructions": "...", "criteria": {"option": "what it means"}}` or `{"type": "score", "instructions":
    /// "...", "criteria": ["lowest level", "...", "highest level"]}`. One to 64.
    #[serde(default)]
    pub questions: Option<BTreeMap<String, Value>>,
    /// **What the ad-hoc questions are about**, in words.
    #[serde(default)]
    pub state: Option<String>,
    /// **Pictures, files under the project**, each with the name a check asks for (`before`, `after`,
    /// `closeup`, ...). Ad-hoc questions send them in this order. At most four.
    #[serde(default)]
    pub images: Vec<NamedImage>,
    /// **Whether to take the editor's current picture too**, named `view`. A named check that needs `view`
    /// and is not given it takes it anyway.
    #[serde(default)]
    pub view: bool,
    /// **The JSON facts**: the operations run, the pieces with names and positions, the camera.
    #[serde(default)]
    pub facts: Option<Value>,
}

/// **A picture on disk, and what it is called.**
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NamedImage {
    /// The name a check asks for.
    pub name: String,
    /// A PNG or JPEG file under the project, relative to it.
    pub path: String,
}

/// The editor's checks with the project's own over them.
pub fn load_checks(project: &Path) -> Result<Checks, String> {
    Checks::for_project(project)
}

/// **Whether the server must take the editor's picture before deciding.**
pub fn needs_view(params: &DecideParams, checks: &Checks) -> bool {
    if params.images.iter().any(|i| i.name == VIEW) {
        return false;
    }
    params.view
        || params
            .check
            .as_deref()
            .and_then(|n| checks.get(n))
            .is_some_and(|c| c.images.iter().any(|i| i == VIEW))
}

/// A file under `project`, as a data URL. Refuses a path that leaves the project.
fn image_data_url(project: &Path, rel: &str) -> Result<String, String> {
    use base64::Engine as _;
    let root = project.canonicalize().map_err(|e| format!("{}: {e}", project.display()))?;
    let full: PathBuf = root.join(rel);
    let full = full.canonicalize().map_err(|e| format!("image `{rel}`: {e}"))?;
    if !full.starts_with(&root) {
        return Err(format!("image `{rel}` is outside the project."));
    }
    let mime = match full.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => return Err(format!("image `{rel}` is not a .png or .jpg file.")),
    };
    let bytes = std::fs::read(&full).map_err(|e| format!("image `{rel}`: {e}"))?;
    Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// **Decide.** `view` is the editor's current picture as base64 PNG, when it was taken.
pub fn decide(
    params: &DecideParams,
    project: &Path,
    view: Option<&str>,
    transport: &dyn Transport,
) -> Result<Value, String> {
    let mut images: BTreeMap<String, String> = BTreeMap::new();
    let mut ordered: Vec<String> = Vec::new();
    for i in &params.images {
        let url = image_data_url(project, &i.path)?;
        ordered.push(url.clone());
        images.insert(i.name.clone(), url);
    }
    if let Some(png) = view {
        let url = format!("data:image/png;base64,{png}");
        if !images.contains_key(VIEW) {
            ordered.push(url.clone());
            images.insert(VIEW.to_owned(), url);
        }
    }
    match (&params.check, &params.questions) {
        (Some(_), Some(_)) => Err("give `check` or `questions`, not both.".into()),
        (None, None) => Err("give `check` (a named check) or `questions` (ad-hoc).".into()),
        (Some(name), None) => {
            let checks = load_checks(project)?;
            let check: &Check = checks.get(name).ok_or_else(|| {
                let known: Vec<&str> = checks.checks.iter().map(|c| c.name.as_str()).collect();
                format!("no check named `{name}`; the checks are: {}.", known.join(", "))
            })?;
            let evidence = Evidence { images, facts: params.facts.clone() };
            let finding = check.run(transport, &evidence).map_err(|e| e.to_string())?;
            Ok(json!({
                "check": name,
                "finding": finding,
                "message": finding.message(),
                "calibrated": check.calibration.is_some(),
                "advisory": true,
                "endpoint": transport.endpoint(),
            }))
        }
        (None, Some(questions)) => {
            let mut req = Request::new(params.state.clone().unwrap_or_default());
            for (id, q) in questions {
                let q: Question = serde_json::from_value(q.clone()).map_err(|e| format!("question `{id}`: {e}"))?;
                req.questions.insert(id.clone(), q);
            }
            req.images = ordered;
            let response = bevy_grimoire_decide::ask(transport, &req).map_err(|e| e.to_string())?;
            Ok(json!({"answers": response.answers, "endpoint": transport.endpoint()}))
        }
    }
}
