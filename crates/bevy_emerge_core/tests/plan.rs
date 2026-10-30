//! **An agent's plan is data beside the map** (E-069): a checklist with a status per item, kept in the
//! `.plan.ron` next to the `.map.ron`, the way a note is kept in `.notes.ron`.

use bevy_emerge_core::plan::{path_for_map, Plan, PlanItem, Status};
use std::path::Path;

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("emerge_plan_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn item(text: &str, status: Status) -> PlanItem {
    PlanItem { text: text.to_owned(), status }
}

#[test]
fn the_file_sits_beside_the_map() {
    assert_eq!(path_for_map(Path::new("a/maps/town.map.ron")), Path::new("a/maps/town.plan.ron"));
    assert_eq!(path_for_map(Path::new("a/maps/odd.ron")), Path::new("a/maps/odd.plan.ron"));
}

#[test]
fn a_plan_survives_a_restart_with_every_status() {
    let dir = scratch("roundtrip");
    let path = path_for_map(&dir.join("town.map.ron"));
    let plan = Plan::new(vec![
        item("lay the road", Status::Done),
        item("place the lamps", Status::Doing),
        item("add a fence", Status::Pending),
        item("plant trees", Status::Skipped),
    ])
    .unwrap();
    plan.save(&path).unwrap();
    assert_eq!(Plan::load(&path).unwrap(), plan);
}

#[test]
fn a_map_with_no_plan_file_has_an_empty_plan_and_an_empty_plan_writes_nothing() {
    let dir = scratch("absent");
    let path = path_for_map(&dir.join("town.map.ron"));
    assert_eq!(Plan::load(&path).unwrap(), Plan::default());
    Plan::default().save(&path).unwrap();
    assert!(!path.exists(), "a map that never had a plan never grows one");
}

#[test]
fn an_item_without_text_is_refused() {
    let e = Plan::new(vec![item("fine", Status::Pending), item("   ", Status::Pending)]).unwrap_err();
    assert!(e.contains("item 2"), "names the item: {e}");
}

#[test]
fn a_status_defaults_to_pending_and_names_are_lowercase_in_json() {
    let items: Vec<PlanItem> = serde_json::from_str(r#"[{"text": "a"}, {"text": "b", "status": "doing"}]"#).unwrap();
    assert_eq!(items[0].status, Status::Pending);
    assert_eq!(items[1].status, Status::Doing);
    assert!(serde_json::from_str::<Vec<PlanItem>>(r#"[{"text": "a", "status": "blocked"}]"#).is_err());
}

#[test]
fn a_file_from_another_version_is_refused() {
    let e = Plan::from_ron("(version: 9, items: [])").unwrap_err();
    assert!(e.contains("version 9"), "{e}");
}
