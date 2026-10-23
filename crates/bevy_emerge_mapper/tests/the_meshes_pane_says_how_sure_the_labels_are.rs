//! **The Meshes pane says how sure the labels are, and who wrote them (cycle 8, item 3).**
//!
//! # What the bug was
//!
//! The labels row of a mesh's detail gave the model and the date (`gemma · 2026-10-20`) and
//! nothing about how sure the model said it was. The confidence it had stored was on the record
//! and on screen nowhere, so the author could not tell a confident label from a guess. The Tiles
//! proposal line had the same gap in another form: it printed the Rust variant (`High`).
//!
//! # What is being proved
//!
//! 1. **The labels row reads the confidence as a word**, after the model name and the date:
//!    `gemma-x · 2026-10-20 · confidence high`, drawn in the real layout of the Meshes tab.
//! 2. **A model-written labels row is drawn in the unchecked colour**, the one the proposal and
//!    the per-axis chips already use for a model's word (`chrome::UNCHECKED`).

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use serde_json::json;

mod fixtures;

/// **The editor, with the bridge open**, on a kit whose one piece was labelled by `gemma-x`.
fn editor(name: &str) -> App {
    let root = fixtures::Fixture::new(name)
        .labelled_descriptor("fence", "alpha", "gemma-x", "High")
        .build("m");
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

#[test]
fn the_labels_row_reads_the_confidence_as_a_word() {
    let mut app = editor("labels_say_how_sure");
    // Ask for the kit door, for real. It opens on the Meshes tab.
    call(
        &mut app,
        "emerge/operation_apply",
        json!({"operation": "door_open", "parameters": {"door": "kit"}, "dry_run": false}),
    );
    for _ in 0..4 {
        app.update();
    }
    // Select the piece in the library list, as a click on its row does.
    app.world_mut()
        .resource_mut::<bevy_emerge_mapper::tiles::ImportState>()
        .selected_library_id = Some("fence".to_owned());
    let texts = drawn_texts(&mut app);
    let want = "gemma-x · 2026-10-20 · confidence high";
    assert!(
        texts.iter().any(|(t, _, _)| t == want),
        "the labels row should read {want:?}; the labels texts drawn are {:?}",
        texts
            .iter()
            .filter(|(t, _, _)| t.contains("gemma") || t.contains("by hand"))
            .collect::<Vec<_>>()
    );

    // **A machine wrote it, and nobody has confirmed it.** The labels row wears the same colour
    // as a model's word elsewhere, not the author's own text colour.
    let ink = {
        let world = app.world_mut();
        let mut q = world.query::<(&Text, &TextColor)>();
        q.iter(world).find(|(t, _)| t.0 == want).map(|(_, c)| c.0)
    };
    assert_eq!(
        ink,
        Some(bevy_emerge_mapper::chrome::UNCHECKED),
        "the model-written labels row should be drawn in the unchecked colour"
    );
}
