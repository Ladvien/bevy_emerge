//! **The Meshes pane says how sure the labels are, and who wrote them (cycle 8, items 3 and 17).**
//!
//! # What the bugs were
//!
//! 1. The labels row of a mesh's detail gave the model and the date (`gemma · 2026-10-20`) and
//!    nothing about how sure the model said it was. The Tiles proposal line printed the Rust
//!    variant (`High`).
//! 2. The row was painted unchecked whenever a model had written the record, so a piece a person
//!    had confirmed in full was still flagged *"a machine decided this and you have not"*, while
//!    its per-axis chips were correctly painted checked. The pane contradicted itself.
//!
//! # What is being proved
//!
//! - **The labels row reads the confidence as a word**, after the model name and the date.
//! - **The row is unchecked iff some axis is still a model's**, by the same predicate that
//!   `emerge_list_pieces` `by_model` asks.
//! - **The date is the model's labelling date** while a model axis is open, and the row says when a
//!   person touched it since. Once every axis is confirmed, the row says `checked <date>` instead.

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use serde_json::json;

mod fixtures;

/// **The editor, with the bridge open**, on a kit whose one piece was labelled by `gemma-x`.
fn editor(name: &str) -> App {
    open(fixtures::Fixture::new(name).labelled_descriptor("fence", "alpha", "gemma-x", "High"))
}

/// **The editor, with the bridge open**, on a kit whose one piece carries the labels `fixture` wrote.
fn open(fixture: fixtures::Fixture) -> App {
    let root = fixture.build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None)
        .unwrap_or_else(|e| panic!("{e}"));
    app.add_plugins(bevy_debugger_bevy::DebuggerPlugin::default());
    app.add_plugins(bevy_emerge_mapper::ops_brp::OpsBridgePlugin);
    for _ in 0..4 {
        app.update();
    }
    app
}

/// **Call one of the editor's BRP commands through the registered method**, as the MCP server does.
fn call(app: &mut App, method: &str, params: serde_json::Value) -> serde_json::Value {
    use bevy::remote::RemoteMethodSystemId;
    use bevy::remote::RemoteMethods;
    let id = match app.world().resource::<RemoteMethods>().get(method) {
        Some(RemoteMethodSystemId::Instant(id)) => *id,
        _ => panic!("no method `{method}` is registered"),
    };
    app.world_mut()
        .run_system_with(id, Some(params))
        .unwrap_or_else(|e| panic!("{method}: {e:?}"))
        .unwrap_or_else(|e| panic!("{method}: {e:?}"))
}

/// **Every drawn text and its box**, in physical pixels: (text, top y, height).
fn drawn_texts(app: &mut App) -> Vec<(String, f32, f32)> {
    for _ in 0..3 {
        app.update();
    }
    let world = app.world_mut();
    let mut q = world.query::<(&Text, &ComputedNode, &UiGlobalTransform, &bevy::ui::Node)>();
    q.iter(world)
        .filter(|(_, node, tf, style)| {
            style.display == bevy::ui::Display::Flex
                && node.size().x > 0.0
                && node.size().y > 0.0
                && tf.translation.y.is_finite()
        })
        .map(|(t, node, tf, _)| {
            (
                t.0.clone(),
                tf.translation.y - node.size().y / 2.0,
                node.size().y,
            )
        })
        .collect()
}

/// **Ask for the kit door, select `fence` as a click on its row does, and read the labels row.**
///
/// Returns the row's text and its ink, or `None` when no labels row is drawn.
fn labels_row(app: &mut App) -> Option<(String, Color)> {
    // Ask for the kit door, for real. It opens on the Meshes tab.
    call(
        app,
        "emerge/operation_apply",
        json!({"operation": "door_open", "parameters": {"door": "kit"}, "dry_run": false}),
    );
    for _ in 0..4 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<bevy_emerge_mapper::tiles::ImportState>()
        .selected_library_id = Some("fence".to_owned());
    drawn_texts(app);
    let world = app.world_mut();
    let mut q = world.query::<(&Text, &TextColor)>();
    q.iter(world)
        .find(|(t, _)| t.0.contains("gemma-x") || t.0.contains("Qwen3-VL") || t.0.starts_with("by hand"))
        .map(|(t, c)| (t.0.clone(), c.0))
}

#[test]
fn the_labels_row_reads_the_confidence_as_a_word() {
    let mut app = editor("labels_say_how_sure");
    let texts_before = drawn_texts(&mut app);
    assert!(texts_before.iter().all(|(t, _, _)| !t.contains("confidence")));
    let (text, ink) = labels_row(&mut app).expect("the labels row should be drawn");

    // The model name, the date, and the confidence as a word.
    let want = "gemma-x · 2026-10-20 · confidence high";
    assert_eq!(text, want, "the labels row should read {want:?}");

    // **A machine wrote it, and nobody has confirmed it.** The labels row wears the same colour
    // as a model's word elsewhere, not the author's own text colour.
    assert_eq!(
        ink,
        bevy_emerge_mapper::chrome::UNCHECKED,
        "the model-written labels row should be drawn in the unchecked colour"
    );
}

/// **A record a person has confirmed in full is not flagged.** Every axis is `Human`, so no model
/// axis is open, and the row must not paint it unchecked — the chips beside it already say *you did*.
#[test]
fn a_record_a_person_confirmed_in_full_is_not_drawn_unchecked() {
    let mut app = open(
        fixtures::Fixture::new("labels_all_confirmed").labelled_by(
            "fence",
            "alpha",
            "(kind: Some(Human), effects: Some(Human), look: Some(Human))",
            r#"model: Some("gemma-x"), at: "2026-10-21", confidence: Some(High)"#,
        ),
    );
    let (text, ink) = labels_row(&mut app).expect("the labels row should be drawn");
    assert_eq!(
        text, "gemma-x · checked 2026-10-21 · confidence high",
        "every axis is confirmed, so the row says when a person checked it, not the model's date"
    );
    assert_eq!(
        ink,
        bevy_emerge_mapper::chrome::TEXT,
        "a fully confirmed record must not wear the model's unchecked colour"
    );
}

/// **While a model axis is open, the row gives the model's date**, and says when a person touched
/// the record since, so the one date on the row cannot be mistaken for the other.
#[test]
fn an_open_model_axis_keeps_the_models_date_and_says_when_a_person_touched_it() {
    let mut app = open(fixtures::Fixture::new("labels_mixed").labelled_by(
        "fence",
        "alpha",
        "(kind: Some(Model), look: Some(Human))",
        r#"model: Some("gemma-x"), model_at: Some("2026-10-20"), at: "2026-10-21", confidence: Some(High)"#,
    ));
    let (text, ink) = labels_row(&mut app).expect("the labels row should be drawn");
    assert_eq!(text, "gemma-x · 2026-10-20 · touched 2026-10-21 · confidence high");
    assert_eq!(ink, bevy_emerge_mapper::chrome::UNCHECKED);
}

/// **The row names the model as a person reads it.** A model stored by its full path shows its file
/// stem; the path itself stays in the record, where `emerge/pieces` still reports it whole.
#[test]
fn the_labels_row_shows_the_models_file_stem_not_its_path() {
    let mut app = open(fixtures::Fixture::new("labels_model_path").labelled_by(
        "fence",
        "alpha",
        "(kind: Some(Model), effects: Some(Model), look: Some(Model))",
        r#"model: Some("/Users/me/models/Qwen3-VL-8B-Instruct-Q4_K_M.gguf"), at: "2026-10-20", confidence: Some(High)"#,
    ));
    let (text, _) = labels_row(&mut app).expect("the labels row should be drawn");
    assert_eq!(text, "Qwen3-VL-8B-Instruct-Q4_K_M · 2026-10-20 · confidence high");
}
