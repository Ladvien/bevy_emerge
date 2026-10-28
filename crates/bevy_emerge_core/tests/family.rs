const KNOWN: &[&str] = &["bevy_elements", "bevy_flora", "nope_not_listed"];
use bevy_emerge_core::family::{path_for_project, Family};

#[test]
fn a_project_without_a_family_file_uses_no_crate() {
    let d = std::env::temp_dir().join(format!("emerge_family_none_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    assert_eq!(Family::load(&d).unwrap(), Family::default());
    assert!(path_for_project(&d).ends_with("family.ron"));
}

#[test]
fn a_listed_crate_without_its_feature_is_a_problem_not_a_silent_skip() {
    let f = Family::from_ron("(version: 1, crates: [\"bevy_elements\", \"bevy_flora\"])").unwrap();
    let problems = f.problems(KNOWN, &["bevy_elements"]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("bevy_flora") && problems[0].contains("--features bevy_flora"), "{problems:?}");
    assert_eq!(f.active(&["bevy_elements"]), vec!["bevy_elements"]);
    assert!(f.problems(KNOWN, &["bevy_elements", "bevy_flora"]).is_empty());
}

#[test]
fn an_unknown_name_and_a_repeat_are_refused_or_reported() {
    let f = Family::from_ron("(version: 1, crates: [\"nope\"])").unwrap();
    assert!(f.problems(&["bevy_elements"], &["nope"])[0].contains("not a family crate"));
    assert!(Family::from_ron("(version: 1, crates: [\"isomesh\", \"isomesh\"])").unwrap_err().contains("twice"));
    assert!(Family::from_ron("(version: 2, crates: [])").is_err());
}
