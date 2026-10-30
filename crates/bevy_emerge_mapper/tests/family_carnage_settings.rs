//! **`bevy_carnage` is a hosted family crate, and its own `GorePolicy` is the saved violence dials** (#108, #96).
#![cfg(feature = "bevy_carnage")]

use bevy_emerge_mapper::ops::{self, Applied, Ctx};
use serde_json::json;

use crate::fixtures;

#[test]
fn the_violence_dials_are_set_through_family_set_and_read_back_as_the_crates_own_type() {
    let root = fixtures::Fixture::new("carnage_dials").descriptor("site/wall", "alpha").build("m");
    std::fs::create_dir_all(root.join("family")).unwrap();
    std::fs::write(root.join("family.ron"), "(version: 1, crates: [\"bevy_carnage\"])").unwrap();
    let file = root.join("family/bevy_carnage.ron");
    let policy = bevy_carnage::GorePolicy::default();
    std::fs::write(&file, ron::ser::to_string_pretty(&policy, ron::ser::PrettyConfig::default()).unwrap()).unwrap();
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..4 {
        app.update();
    }
    let project = app.world().resource::<bevy_emerge_mapper::project::Project>().clone();
    assert!(bevy_emerge_mapper::family_host::problems_with(&root, &bevy_emerge_mapper::family_host::built()).is_empty());
    let mut ctx = Ctx { project: Some(project), session: "test".to_owned(), ..Ctx::default() };
    let done = ops::dispatch(&mut ctx, "family_set", &json!({"crate": "bevy_carnage", "field": "viscera", "value": "false"})).unwrap();
    assert!(matches!(done, Applied::Done { .. }));
    let read: bevy_carnage::GorePolicy = ron::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(!read.viscera);
    assert_eq!(read.dismemberment, policy.dismemberment, "only the named dial changes");
}
