//! **`composition_set` defines, replaces and removes a composition, and is one undo** (E-072).
//! An agent can author the middle of the chain (mesh, piece, composition, layout).

use bevy_emerge_mapper::ops::{self, Applied, Ctx};
use serde_json::json;

mod fixtures;

fn project(tag: &str) -> (Ctx, std::path::PathBuf) {
    let root = fixtures::Fixture::new(tag).descriptor("site/wall", "alpha").build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..4 {
        app.update();
    }
    let project = app.world().resource::<bevy_emerge_mapper::project::Project>().clone();
    let file = project.project_dir.join("compositions.ron");
    (Ctx { project: Some(project), session: "test".to_owned(), ..Ctx::default() }, file)
}

fn group(id: &str, piece: &str, envelope: serde_json::Value) -> serde_json::Value {
    json!({
        "id": id,
        "envelope": envelope,
        "members": [{
            "id": "wall",
            "body": {"Descriptor": {"id": piece}},
            "at": [0.0, 0.0],
        }],
    })
}

fn refused(r: Result<Applied, ops::Refusal>) -> String {
    match r {
        Err(e) | Ok(Applied::Refused(e)) => e.0,
        Ok(_) => panic!("the change was accepted"),
    }
}

fn ids(ctx: &Ctx) -> Vec<String> {
    ctx.project.as_ref().unwrap().compositions.compositions.iter().map(|c| c.id.clone()).collect()
}

#[test]
fn a_composition_is_defined_replaced_and_each_step_is_one_undo() {
    let (mut ctx, file) = project("set");
    let before = std::fs::read_to_string(&file).ok();
    let a = group("site/hut", "site/wall", json!("Anchored"));
    assert!(matches!(ops::dispatch(&mut ctx, "composition_set", &json!({"composition": a})).unwrap(), Applied::Done { .. }));
    assert_eq!(ids(&ctx), ["site/hut"]);
    let first = std::fs::read_to_string(&file).unwrap();
    assert!(first.contains("site/hut"));

    let b = group("site/hut", "site/wall", json!({"Bounded": {"size": [2.0, 3.0, 2.0]}}));
    ops::dispatch(&mut ctx, "composition_set", &json!({"composition": b})).unwrap();
    assert!(std::fs::read_to_string(&file).unwrap().contains("Bounded"));

    ops::undo(&mut ctx, None).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), first, "undo of a replace puts the old definition back");
    ops::undo(&mut ctx, None).unwrap();
    assert!(ids(&ctx).is_empty(), "undo of a definition removes it");
    let _ = before;
}

#[test]
fn a_refusal_writes_nothing_and_the_undo_forms_are_not_an_agents() {
    let (mut ctx, file) = project("refuse");
    let none = std::fs::read_to_string(&file).ok();
    let e = refused(ops::dispatch(
        &mut ctx,
        "composition_set",
        &json!({"composition": group("site/ghost", "site/nothing", json!("Anchored"))}),
    ));
    assert!(e.contains("nothing"), "{e}");
    assert_eq!(std::fs::read_to_string(&file).ok(), none);
    assert!(ids(&ctx).is_empty());

    let e = refused(ops::dispatch(&mut ctx, "composition_set", &json!({"remove": "site/hut"})));
    assert!(!e.is_empty());
    let e = refused(ops::dispatch(&mut ctx, "composition_set", &json!({})));
    assert!(e.contains("exactly one"), "{e}");
}

#[test]
fn a_preview_names_the_change_and_writes_nothing() {
    let (ctx, file) = project("plan");
    let none = std::fs::read_to_string(&file).ok();
    let op = ops::find("composition_set").unwrap();
    let plan = op.plan(&ctx, &json!({"composition": group("site/hut", "site/wall", json!("Anchored"))})).unwrap();
    assert!(plan["would"].as_str().unwrap().contains("site/hut"));
    assert_eq!(std::fs::read_to_string(&file).ok(), none);
}
