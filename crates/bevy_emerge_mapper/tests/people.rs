//! **A person is a marker with a home and a workplace, saved beside the map, and a building someone lives in stays** (#104).

use bevy_emerge_mapper::ops::{self, Applied, Ctx};
use bevy_emerge_mapper::ops_people::people_of;
use bevy_emerge_mapper::project::Project;
use serde_json::json;

use crate::fixtures;

fn open(tag: &str) -> (Ctx, Project) {
    let root = fixtures::Fixture::new(tag).descriptor("site/wall", "alpha").build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..4 {
        app.update();
    }
    let project = app.world().resource::<Project>().clone();
    let open = app.world().resource::<bevy_emerge_mapper::project::OpenMap>().clone();
    (Ctx { project: Some(project.clone()), open: Some(open), session: tag.to_owned(), ..Ctx::default() }, project)
}

fn place(ctx: &mut Ctx, at: [f32; 2]) -> String {
    let Applied::Done { outcome, .. } = ops::dispatch(ctx, "placement_add", &json!({"piece": "site/wall", "at": at})).unwrap() else {
        panic!("placement refused")
    };
    ctx.open.as_ref().unwrap().map.placements.last().unwrap_or_else(|| panic!("{}", outcome.summary)).id.clone()
}

#[test]
fn a_person_keeps_their_home_and_work_and_the_building_stays_until_they_move() {
    let (mut ctx, project) = open("people_home");
    let (house, shop) = (place(&mut ctx, [0.5, 0.5]), place(&mut ctx, [4.5, 0.5]));
    let Applied::Done { .. } = ops::dispatch(
        &mut ctx,
        "person_set",
        &json!({"map": "m", "name": "Mara", "at": [1.0, 1.0], "home": house, "work": shop, "faction": "guild"}),
    )
    .unwrap() else {
        panic!("refused")
    };
    let p = &people_of(&project, "m").people[0];
    assert_eq!((p.name.as_str(), p.home.as_deref(), p.work.as_deref()), ("Mara", Some(house.as_str()), Some(shop.as_str())));

    let why = ops::dispatch(&mut ctx, "placement_remove", &json!({"id": house})).map(|a| matches!(a, Applied::Done { .. }));
    let msg = match why {
        Err(e) => e.0,
        Ok(false) => "refused".into(),
        Ok(true) => panic!("a home was removed from under its owner"),
    };
    assert!(msg.contains("Mara") && msg.contains("home"), "the refusal names the person: {msg}");
    assert!(ctx.open.as_ref().unwrap().map.placements.iter().any(|q| q.id == house), "the building is still on the map");

    ops::dispatch(&mut ctx, "person_set", &json!({"map": "m", "name": "Mara", "home": ""})).unwrap();
    ops::dispatch(&mut ctx, "placement_remove", &json!({"id": house})).unwrap();

    // Naming a building that is not there is refused, and nothing is written.
    let before = people_of(&project, "m");
    assert!(ops::dispatch(&mut ctx, "person_set", &json!({"map": "m", "name": "Mara", "work": "nowhere"})).is_err());
    assert_eq!(people_of(&project, "m"), before);
}

#[test]
fn one_undo_takes_a_person_back() {
    let (mut ctx, project) = open("people_undo");
    ops::dispatch(&mut ctx, "person_set", &json!({"map": "m", "name": "Ola", "at": [2.0, 2.0]})).unwrap();
    let Applied::Done { operation_id, .. } = ops::dispatch(&mut ctx, "person_set", &json!({"map": "m", "name": "Ben", "at": [3.0, 2.0]})).unwrap() else {
        panic!("refused")
    };
    assert_eq!(people_of(&project, "m").people.len(), 2);
    ops::undo(&mut ctx, Some(&operation_id)).unwrap();
    let left = people_of(&project, "m").people;
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].name, "Ola");
}
