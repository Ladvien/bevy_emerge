//! **A dialogue the crate's `validate_script` refuses is not written** (#106): the saved conversation is the
//! crate's own `DialogueScript`, and `family_set` is how a person or an agent edits it.
#![cfg(feature = "bevy_speech_bubbles")]

use bevy_emerge_mapper::ops::{self, Applied, Ctx};
use serde_json::json;

use crate::fixtures;

const SCRIPT: &str = r#"(conversations: {"hello": (start: "a", nodes: {
    "a": Line(speaker: "Mara", text: "Well met.", next: Some("b")),
    "b": Choice(speaker: "Mara", prompt: "Stay?", options: [(text: "Yes", next: "c")]),
    "c": Line(speaker: "Mara", text: "Good.", next: None),
})})"#;

/// The `conversations` field's value with the start node replaced: a conversation is edited whole.
fn with_start(start: &str) -> String {
    let body = SCRIPT.trim_start_matches("(conversations: ").trim_end_matches(')');
    body.replace("start: \"a\"", &format!("start: \"{start}\""))
}

fn open(name: &str) -> (Ctx, std::path::PathBuf) {
    let root = fixtures::Fixture::new(name).descriptor("site/wall", "alpha").build("m");
    std::fs::create_dir_all(root.join("family")).unwrap();
    std::fs::write(root.join("family.ron"), "(version: 1, crates: [\"bevy_speech_bubbles\"])").unwrap();
    let file = root.join("family/bevy_speech_bubbles.ron");
    std::fs::write(&file, SCRIPT).unwrap();
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..4 {
        app.update();
    }
    let project = app.world().resource::<bevy_emerge_mapper::project::Project>().clone();
    (Ctx { project: Some(project), session: "test".to_owned(), ..Ctx::default() }, file)
}

#[test]
fn a_choice_pointing_nowhere_is_refused_with_the_crates_message_and_the_file_is_untouched() {
    let (mut ctx, file) = open("dialogue_bad");
    let err = ops::dispatch(
        &mut ctx,
        "family_set",
        &json!({"crate": "bevy_speech_bubbles", "field": "conversations", "value": with_start("nope")}),
    )
    .map(|_| ())
    .unwrap_err();
    assert!(format!("{err:?}").contains("start node 'nope' not found"), "{err:?}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), SCRIPT);
}

#[test]
fn a_valid_edit_is_written_and_reads_back_as_the_crates_own_script() {
    let (mut ctx, file) = open("dialogue_ok");
    let done = ops::dispatch(
        &mut ctx,
        "family_set",
        &json!({"crate": "bevy_speech_bubbles", "field": "conversations", "value": with_start("b")}),
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(matches!(done, Applied::Done { .. }));
    let script: bevy_speech_bubbles::DialogueScript = ron::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(script.conversation("hello").unwrap().start, "b");
}
