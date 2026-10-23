//! **The INSPECTOR's status slot is one labelled row, and a hint does not move the panel.**
//!
//! B-012. The receipt sits in UNDER's value column with no label of its own, so "map is 300 x 44 x
//! 1120 m, and the file was written" reads as the value of UNDER, the row above it.
//!
//! B-013. The hint row (`Field::Hint`) is zero tall while it is empty, so a hint appearing (a
//! surface or mounted piece armed over floor it cannot stand on) pushed EDGES, MAP SIZE and the
//! lists below it down. The fix folds the hint into the receipt's reserved slot, so the panel keeps
//! one height whatever the status says.
//!
//! **The real geometry, from the real layout**: `ComputedNode` and `UiGlobalTransform` in a headless
//! app, the receipt produced by the size field itself, and the hint set on the editor's state.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy_emerge_mapper::editor::EditorState;

mod fixtures;

fn editor(tag: &str) -> App {
    let root = fixtures::Fixture::new(tag)
        .sized_descriptor("fence", "alpha", 2.064, 0.12)
        .build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None)
        .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..6 {
        app.update();
    }
    app
}

fn tap(app: &mut App, logical: Key, code: KeyCode) {
    for state in [
        bevy::input::ButtonState::Pressed,
        bevy::input::ButtonState::Released,
    ] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: logical.clone(),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
}

/// **Resize one axis through the size field**, as a person does: open it, type, Enter.
fn resize(app: &mut App, axis: char, metres: &str) {
    bevy_emerge_mapper::editor::size_field_for_test(app.world_mut(), axis);
    app.update();
    for c in metres.chars() {
        let code = match c {
            '0' => KeyCode::Digit0,
            '1' => KeyCode::Digit1,
            '2' => KeyCode::Digit2,
            '3' => KeyCode::Digit3,
            '4' => KeyCode::Digit4,
            '5' => KeyCode::Digit5,
            '6' => KeyCode::Digit6,
            '7' => KeyCode::Digit7,
            '8' => KeyCode::Digit8,
            '9' => KeyCode::Digit9,
            _ => panic!("not a digit: {c}"),
        };
        tap(app, Key::Character(c.to_string().into()), code);
    }
    tap(app, Key::Enter, KeyCode::Enter);
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

fn top_of(texts: &[(String, f32, f32)], want: &str) -> f32 {
    texts
        .iter()
        .find(|(t, _, _)| t == want)
        .map(|(_, top, _)| *top)
        .unwrap_or_else(|| panic!("no drawn text reads {want:?}; drawn: {texts:?}"))
}

/// **A hint leaves MAP SIZE where it was.**
///
/// Failed before the fix: with the hint set, the hint row grew from zero and MAP SIZE moved down by
/// the hint's height.
#[test]
fn a_hint_does_not_push_map_size_down() {
    let mut app = editor("hint-holds-still");
    let before = drawn_texts(&mut app);
    let heading = top_of(&before, "MAP SIZE  (m)");

    let hint = "a surface piece cannot stand on this floor";
    app.world_mut().resource_mut::<EditorState>().hint = hint.to_owned();
    let after = drawn_texts(&mut app);

    assert!(
        after.iter().any(|(t, _, _)| t.contains("cannot stand on this floor")),
        "the hint is not drawn, so this test is not about a hint that is there; drawn: {after:?}"
    );
    let moved = top_of(&after, "MAP SIZE  (m)") - heading;
    assert!(
        moved.abs() < 0.5,
        "the hint pushed MAP SIZE down by {moved:.1} px; a hint that comes and goes with the cursor \
         must not move the panel under the author's pointer"
    );
}

/// **The receipt has a label, so it does not read as the value of the row above it.**
///
/// Failed before the fix: the receipt's row had an empty label, and no text read `LAST` at its top.
#[test]
fn the_receipt_row_is_labelled_so_it_is_not_taken_for_under() {
    let mut app = editor("receipt-is-labelled");
    resize(&mut app, 'y', "44");
    let after = drawn_texts(&mut app);

    let receipt = after
        .iter()
        .find(|(t, _, _)| t.contains("and the file was written"))
        .unwrap_or_else(|| panic!("the resize's receipt is not drawn; drawn: {after:?}"));
    let label = top_of(&after, "LAST");
    assert!(
        (label - receipt.1).abs() < 0.5,
        "the LAST label sits at {label:.1} px but the receipt at {:.1} px; the receipt has no \
         label on its own row",
        receipt.1,
    );
}
