//! **An effect an agent writes is settled the way a kind write settles it.** Cycle 9 gap 6.
//!
//! `prop` implies `powered` in this vocabulary. Writing `kind` settled the effects a kind implies;
//! writing `effects` did not, so the registry answered with a list the library's layered read then
//! changed underneath. Now a derived effect that the piece's kind does not imply is refused, naming
//! the kind, and leaving an implied effect out puts it back. The axis stays stamped `agent`.

use bevy::prelude::*;
use bevy_emerge_core::descriptor::{Axis, By};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::project::Project;
use bevy_emerge_mapper::tiles::Mode;

mod fixtures;

fn editor_at(name: &str) -> App {
    let root = fixtures::Fixture::new(name)
        .kind_implies("powered")
        .descriptor("floor", "alpha")
        .descriptor("bench", "alpha")
        .build("m");
    let mut app =
        harness::build_headless_at(&root, "m", None, Mode::Meshes).unwrap_or_else(|e| panic!("{e}"));
    app.add_plugins(bevy_debugger_bevy::DebuggerPlugin::default());
    app.add_plugins(bevy_emerge_mapper::ops_brp::OpsBridgePlugin);
    for _ in 0..8 {
        app.update();
    }
    app
}

/// **Through the registered method**, as an agent's request goes.
fn call(app: &mut App, method: &str, params: serde_json::Value) -> serde_json::Value {
    use bevy::remote::{RemoteMethodSystemId, RemoteMethods};
    let Some(RemoteMethodSystemId::Instant(id)) =
        app.world().resource::<RemoteMethods>().get(method).cloned()
    else {
        panic!("no method `{method}` is registered, so an agent cannot reach it");
    };
    app.world_mut()
        .run_system_with(id, Some(params))
        .unwrap_or_else(|e| panic!("{method}: {e:?}"))
        .unwrap_or_else(|e| panic!("{method}: {e:?}"))
}

fn label_set(app: &mut App, id: &str, axis: &str, tokens: &[&str]) -> serde_json::Value {
    call(
        app,
        "emerge/operation_apply",
        serde_json::json!({
            "operation": "label_set",
            "parameters": {"id": id, "axis": axis, "tokens": tokens},
            "dry_run": false,
        }),
    )
}

/// The piece as the editor holds it in memory — the live resource, not a copy.
fn live_piece(app: &App, id: &str) -> bevy_emerge_core::descriptor::Descriptor {
    app.world()
        .resource::<Project>()
        .measured
        .descriptors
        .iter()
        .find(|d| d.id == id)
        .unwrap_or_else(|| panic!("`{id}` is not in the live measured library"))
        .clone()
}

#[test]
fn an_effect_the_kind_does_not_imply_is_refused_and_the_piece_is_unchanged() {
    let mut app = editor_at("effects-refused");
    let cleared = label_set(&mut app, "bench", "kind", &[]);
    assert_eq!(cleared["outcome"], "applied", "{cleared}");
    let before = live_piece(&app, "bench").effects;
    let answer = label_set(&mut app, "bench", "effects", &["powered"]);
    assert_eq!(answer["outcome"], "refused", "{answer}");
    let why = answer["refusal"].as_str().unwrap_or_default();
    assert!(why.contains("powered"), "the refusal names the effect: {why}");
    assert!(
        why.contains("does not imply"),
        "the refusal says the kind does not imply it: {why}"
    );
    assert_eq!(live_piece(&app, "bench").effects, before, "a refused write changes nothing");
}

#[test]
fn leaving_out_an_effect_the_kind_implies_puts_it_back_and_stays_agent() {
    let mut app = editor_at("effects-settled");
    let kind = label_set(&mut app, "bench", "kind", &["prop"]);
    assert_eq!(kind["outcome"], "applied", "{kind}");
    assert!(
        live_piece(&app, "bench").effects.iter().any(|e| e == "powered"),
        "a kind write settles the effect it implies"
    );

    let answer = label_set(&mut app, "bench", "effects", &[]);
    assert_eq!(answer["outcome"], "applied", "{answer}");
    let piece = live_piece(&app, "bench");
    assert!(
        piece.effects.iter().any(|e| e == "powered"),
        "an effects write settles the implied effect back in: {:?}",
        piece.effects
    );
    let by = piece.labels.and_then(|l| l.by.get(Axis::Effects));
    assert_eq!(by, Some(By::Agent), "the axis stays stamped agent");
}
