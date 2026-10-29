//! **A family settings file edited while the editor runs is reloaded within a second** (#96).

use bevy::prelude::*;
use bevy_emerge_mapper::editor::EditorState;
use bevy_emerge_mapper::family_host::{reload_settings, FamilySettings};

use crate::fixtures;

#[test]
fn an_edited_settings_file_reaches_the_editor_and_a_bad_one_is_a_problem() {
    let root = fixtures::Fixture::new("family_reload").descriptor("site/wall", "alpha").build("m");
    let file = root.join("family").join("bevy_elements.ron");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "(sun: (hour: 6.0))").unwrap();
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.init_resource::<EditorState>().init_resource::<FamilySettings>().add_systems(Update, reload_settings);
    app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(&file, "(sun: (hour: 18.5))").unwrap();
    let start = std::time::Instant::now();
    while start.elapsed().as_secs_f64() < 2.0 && app.world().resource::<FamilySettings>().text["bevy_elements"] != "(sun: (hour: 18.5))" {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(start.elapsed().as_secs_f64() < 1.6, "took {:?}", start.elapsed());
    assert_eq!(app.world().resource::<FamilySettings>().text["bevy_elements"], "(sun: (hour: 18.5))");

    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(&file, "(sun: (hour: ").unwrap();
    let start = std::time::Instant::now();
    while start.elapsed().as_secs_f64() < 3.0 && app.world().resource::<EditorState>().status.problems().is_empty() {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let probs = app.world().resource::<EditorState>().status.problems().iter().map(|p| p.text.clone()).collect::<Vec<_>>();
    assert!(probs.iter().any(|p| p.contains("bevy_elements") && p.contains("not valid RON")), "{probs:?}");
    assert_eq!(app.world().resource::<FamilySettings>().text["bevy_elements"], "(sun: (hour: 18.5))");
}
