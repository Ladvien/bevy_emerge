//! **People have ties to each other, authored beside the map and carried into a run** (#104).

use bevy_emerge_mapper::ops::{self, Applied, Ctx};
use bevy_emerge_mapper::ops_people::people_of;
use serde_json::json;

use crate::fixtures;

fn open(tag: &str) -> Ctx {
    let root = fixtures::Fixture::new(tag).descriptor("site/wall", "alpha").build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..4 {
        app.update();
    }
    let project = app.world().resource::<bevy_emerge_mapper::project::Project>().clone();
    let open = app.world().resource::<bevy_emerge_mapper::project::OpenMap>().clone();
    Ctx { project: Some(project), open: Some(open), session: tag.to_owned(), ..Ctx::default() }
}

fn set(ctx: &mut Ctx, params: serde_json::Value) -> Result<(), String> {
    match ops::dispatch(ctx, "person_set", &params) {
        Ok(Applied::Done { .. }) => Ok(()),
        Ok(Applied::Refused(e)) | Err(e) => Err(e.0),
    }
}

#[test]
fn a_tie_is_saved_updated_ended_and_refused_when_it_names_nobody() {
    let mut ctx = open("people_ties");
    let project = ctx.project.clone().unwrap();
    set(&mut ctx, json!({"map": "m", "name": "Mara", "at": [1.0, 1.0]})).unwrap();
    set(&mut ctx, json!({"map": "m", "name": "Odo", "at": [3.0, 1.0]})).unwrap();

    set(&mut ctx, json!({"map": "m", "name": "Mara", "tie": {"with": "Odo", "affinity": 0.3}})).unwrap();
    let t = &people_of(&project, "m").people[0].ties;
    assert_eq!((t.len(), t[0].with.as_str(), t[0].affinity, t[0].trust), (1, "Odo", 0.3, 0.5));

    // Setting only trust keeps the affinity.
    set(&mut ctx, json!({"map": "m", "name": "Mara", "tie": {"with": "Odo", "trust": 0.9}})).unwrap();
    let t = &people_of(&project, "m").people[0].ties;
    assert_eq!((t[0].affinity, t[0].trust), (0.3, 0.9));

    let before = people_of(&project, "m");
    for bad in [
        json!({"with": "Nobody", "affinity": 0.1}),
        json!({"with": "Mara", "affinity": 0.1}),
        json!({"with": "Odo", "affinity": 2.0}),
    ] {
        assert!(set(&mut ctx, json!({"map": "m", "name": "Mara", "tie": bad})).is_err());
        assert_eq!(people_of(&project, "m"), before, "a refused tie writes nothing");
    }

    // Removing the person drops the ties that pointed at them.
    set(&mut ctx, json!({"map": "m", "name": "Odo", "remove": true})).unwrap();
    assert!(people_of(&project, "m").people[0].ties.is_empty());
}

#[cfg(feature = "bevy_society")]
#[test]
fn an_authored_tie_is_in_the_society_the_run_starts_from() {
    let mut ctx = open("people_ties_run");
    set(&mut ctx, json!({"map": "m", "name": "Mara", "at": [1.0, 1.0]})).unwrap();
    set(&mut ctx, json!({"map": "m", "name": "Odo", "at": [3.0, 1.0]})).unwrap();
    set(&mut ctx, json!({"map": "m", "name": "Mara", "tie": {"with": "Odo", "affinity": 0.3, "trust": 0.8}})).unwrap();
    let Applied::Done { outcome, .. } = ops::dispatch(&mut ctx, "people_run", &json!({"map": "m", "seed": 1, "days": 0.01})).unwrap() else {
        panic!("refused")
    };
    let after = outcome.after.unwrap();
    let mara = after["people"].as_array().unwrap().iter().find(|p| p["name"] == "Mara").unwrap();
    let bond = &mara["bonds"].as_array().unwrap().iter().find(|b| b[0] == "Odo").unwrap_or_else(|| panic!("no tie to Odo: {mara}"));
    assert!((bond[1].as_f64().unwrap() - 0.3).abs() < 0.2 && bond[2].as_f64().unwrap() > 0.5, "{bond}");
}

/// **Triggers are the project's Rhai, run by the crate, and the run says which fired** (#105).
#[cfg(feature = "bevy_society")]
mod triggers {
    use super::*;

    fn trigger_ctx(tag: &str) -> Ctx {
        let mut ctx = open(tag);
        let root = ctx.project.clone().unwrap().root;
        std::fs::create_dir_all(root.join("family")).unwrap();
        std::fs::write(root.join("family.ron"), "(version: 1, crates: [\"bevy_society\"])").unwrap();
        std::fs::write(root.join("family/bevy_society.ron"), "(factions: [(id: \"guild\")])").unwrap();
        ctx
    }

    fn put(ctx: &mut Ctx, value: &str) -> Result<(), String> {
        match ops::dispatch(ctx, "family_set", &json!({"crate": "bevy_society", "field": "triggers", "value": value})) {
            Ok(Applied::Done { .. }) => Ok(()),
            Ok(Applied::Refused(e)) | Err(e) => Err(e.0),
        }
    }

    const SEEN: &str = r#"[(id: "seen", on: Hour(7.0), then: "for p in ctx.persons() { fx.emit(\"seen\", #{ who: p }); }")]"#;

    #[test]
    fn a_trigger_with_a_syntax_error_is_refused_with_its_line_and_nothing_is_saved() {
        let mut ctx = trigger_ctx("trig_syntax");
        let file = ctx.project.clone().unwrap().root.join("family/bevy_society.ron");
        let before = std::fs::read_to_string(&file).unwrap();
        let e = put(&mut ctx, r#"[(id: "x", on: Tick, then: "let a = 1;\nlet b = (2;")]"#).unwrap_err();
        assert!(e.contains("trigger 'x'") && e.contains("line 2"), "{e}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    }

    #[test]
    fn a_trigger_with_an_area_reports_only_people_inside_it() {
        let mut ctx = trigger_ctx("trig_area");
        set(&mut ctx, json!({"map": "m", "name": "Mara", "at": [1.0, 1.0]})).unwrap();
        set(&mut ctx, json!({"map": "m", "name": "Far", "at": [200.0, 200.0]})).unwrap();
        put(&mut ctx, SEEN).unwrap();
        let run = |ctx: &mut Ctx, p: serde_json::Value| {
            let Applied::Done { outcome, .. } = ops::dispatch(ctx, "people_run", &p).unwrap() else { panic!("refused") };
            outcome.after.unwrap()["fired"].clone()
        };
        let all = run(&mut ctx, json!({"map": "m", "days": 1.0}));
        let names = |v: &serde_json::Value| -> Vec<String> { v.as_array().unwrap().iter().map(|f| f["who"].as_str().unwrap().to_owned()).collect() };
        let mut n = names(&all);
        n.sort();
        n.dedup();
        assert_eq!(n, ["Far", "Mara"], "{all}");
        let inside = run(&mut ctx, json!({"map": "m", "days": 1.0, "area": [-10.0, -10.0, 10.0, 10.0]}));
        let mut n = names(&inside);
        n.dedup();
        assert_eq!(n, ["Mara"], "{inside}");
    }
}
