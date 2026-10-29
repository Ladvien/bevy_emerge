//! The loop's checker: named checks on captures, over `bevy_emerge_decide`. Driven by `tools/decide/check.py`.
//!
//!   check run [--checks FILE] [--only a,b] [--bar 0.9] [--replay DIR] [--title T] [--dir D] [--image name=path]... [--facts FILE]
//!   check calibrate --checks FILE --check NAME --samples FILE [--date D]
//!
//! `run` exits 1 only when a check calibrated at or above `--bar` finds a failure, 2 when a check could
//! not be asked, else 0. `calibrate` asks the live endpoint (`EMERGE_DECIDE_*`) and prints the calibration.
use base64::Engine as _;
use bevy_emerge_decide::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

fn data_url(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mime = if path.extension().is_some_and(|e| e == "jpg" || e == "jpeg") { "jpeg" } else { "png" };
    Ok(format!("data:image/{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

fn load_checks(file: Option<&str>) -> Result<Checks, String> {
    match file {
        None => Ok(Checks::builtin()),
        Some(f) => {
            let text = std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?;
            Checks::from_ron(&text).map_err(|e| format!("{f}: {e}"))
        }
    }
}

fn with_title(mut c: Check, title: Option<&str>) -> Check {
    if let Some(t) = title {
        c.question = c.question.replace("{title}", t);
    }
    c
}

fn opts(args: &[String]) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let k = a.strip_prefix("--").ok_or_else(|| format!("unexpected argument `{a}`"))?;
        m.entry(k.into()).or_default().push(it.next().ok_or_else(|| format!("--{k} needs a value"))?.clone());
    }
    Ok(m)
}

fn one<'a>(o: &'a BTreeMap<String, Vec<String>>, k: &str) -> Option<&'a str> {
    o.get(k).and_then(|v| v.last()).map(String::as_str)
}

fn run(o: &BTreeMap<String, Vec<String>>) -> Result<ExitCode, String> {
    let checks = load_checks(one(o, "checks"))?;
    let bar: f64 = one(o, "bar").map_or(Ok(0.9), str::parse).map_err(|e| format!("--bar: {e}"))?;
    let mut ev = Evidence::default();
    if let Some(dir) = one(o, "dir") {
        for e in std::fs::read_dir(dir).map_err(|e| format!("{dir}: {e}"))? {
            let p = e.map_err(|e| e.to_string())?.path();
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "png" | "jpg" | "jpeg") {
                ev.images.insert(p.file_stem().unwrap().to_string_lossy().into(), data_url(&p)?);
            }
        }
        let facts = Path::new(dir).join("facts.json");
        if facts.exists() {
            ev.facts = Some(read_json(&facts)?);
        }
    }
    for spec in o.get("image").into_iter().flatten() {
        let (name, path) = spec.split_once('=').ok_or_else(|| format!("--image {spec}: want name=path"))?;
        ev.images.insert(name.into(), data_url(Path::new(path))?);
    }
    if let Some(f) = one(o, "facts") {
        ev.facts = Some(read_json(Path::new(f))?);
    }
    let only: Vec<&str> = one(o, "only").map(|s| s.split(',').collect()).unwrap_or_default();
    let title = one(o, "title");
    let selected: Vec<Check> = checks
        .checks
        .iter()
        .filter(|c| only.is_empty() || only.contains(&c.name.as_str()))
        .cloned()
        .map(|c| with_title(c, title))
        .collect();
    if let Some(n) = only.iter().find(|n| checks.get(n).is_none()) {
        return Err(format!("no check named `{n}`"));
    }
    let transport: Box<dyn Transport> = match one(o, "replay") {
        Some(d) => Box::new(Replay::load(Path::new(d)).map_err(|e| e.to_string())?),
        None => Box::new(Http::new(Config::from_env())),
    };
    let report = run_checks(&selected, transport.as_ref(), &ev, bar);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
    Ok(if report.blocks() {
        ExitCode::from(1)
    } else if report.has_errors() {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

fn read_json(p: &Path) -> Result<serde_json::Value, String> {
    let t = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(&t).map_err(|e| format!("{}: {e}", p.display()))
}

#[derive(serde::Deserialize)]
struct SampleIn {
    images: BTreeMap<String, String>,
    #[serde(default)]
    facts: Option<serde_json::Value>,
    #[serde(default)]
    title: Option<String>,
    /// True when the check should answer yes.
    truth: bool,
}

fn calibrate_cmd(o: &BTreeMap<String, Vec<String>>) -> Result<ExitCode, String> {
    let checks = load_checks(one(o, "checks"))?;
    let name = one(o, "check").ok_or("--check NAME is required")?;
    let check = checks.get(name).ok_or_else(|| format!("no check named `{name}`"))?;
    let samples_path = one(o, "samples").ok_or("--samples FILE is required")?;
    let samples: Vec<SampleIn> = serde_json::from_value(read_json(Path::new(samples_path))?).map_err(|e| format!("{samples_path}: {e}"))?;
    let config = Config::from_env();
    let model = config.model.clone().unwrap_or_else(|| config.endpoint.clone());
    let http = Http::new(config);
    let mut labelled = Vec::new();
    for (i, s) in samples.iter().enumerate() {
        let mut ev = Evidence { images: BTreeMap::new(), facts: s.facts.clone() };
        for (n, p) in &s.images {
            ev.images.insert(n.clone(), data_url(Path::new(p))?);
        }
        let f = with_title(check.clone(), s.title.as_deref()).run(&http, &ev).map_err(|e| format!("sample {i}: {e}"))?;
        eprintln!("sample {i}: p_yes {:.3} truth {}", f.p_yes, s.truth);
        labelled.push(Sample { p_yes: f.p_yes, truth: s.truth });
    }
    let date = one(o, "date").unwrap_or("unknown");
    let cal = calibrate(name, &model, date, &labelled).map_err(|e| e.to_string())?;
    println!("{}", ron::ser::to_string_pretty(&cal, ron::ser::PrettyConfig::default()).map_err(|e| e.to_string())?);
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("run") => opts(&args[1..]).and_then(|o| run(&o)),
        Some("calibrate") => opts(&args[1..]).and_then(|o| calibrate_cmd(&o)),
        _ => Err("usage: check run|calibrate ... (see the header of examples/check.rs)".into()),
    };
    result.unwrap_or_else(|e| {
        eprintln!("check: {e}");
        ExitCode::from(2)
    })
}
