//! **The person's half of E-014: arm, drag a rectangle, type one line, Enter saves a capture.**
//!
//! A snip is a capture, not a registry operation: nothing here changes a map, so no operation is
//! registered for it. `Cmd+\` arms it. A press and a release over the window mark a rectangle, and
//! the release asks for a one-line note. Enter confirms and the window is captured; Escape cancels
//! and writes nothing.
//!
//! **What is written.** `<root>/target/loop/captures/snips/snip-<ms>.png`, the crop of the window
//! inside the rectangle, and `snip-<ms>.json`, its metadata (map, tool, camera, cursor, placements
//! inside the rectangle, the note). Both go through [`bevy_emerge_snip::write_snip`], the same code
//! `emerge_snip` uses, and `create_new` means an earlier snip is never replaced. Nothing is written
//! under `assets/emerge`, which games load.
//!
//! **A headless app cannot write the picture.** It has no render world, so no screenshot is ever
//! read back. A confirmed snip there is refused in `Status::problem`, with the reason, and no file
//! is created. The gesture is proved headless; the bytes on disk are proved by the crate's test and
//! by a live run.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::input::ButtonInput;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::{PrimaryWindow, Window};
use bevy_emerge_snip::{CameraView, PlacedIn, SnipRecord, write_snip};

use crate::chrome;
use crate::editor::{EditorState, Placement};
use crate::keys::{self, Action, Live, Phase};
use crate::project::{OpenMap, Project};
use crate::sequencer::CanRender;
use crate::view::{MainCamera, Pointer, cursor_ground};

/// Where snips are written, relative to the project root. Under `target/`, never `assets/emerge`.
pub const SNIP_DIR: &str = "target/loop/captures/snips";

/// The smallest drag that is a picture. Below this a release is a slip of the hand.
const MIN_SIDE_PX: f32 = 8.0;

/// The longest note the field keeps.
const NOTE_CAP: usize = 200;

/// A rectangle waiting for its note. `rect` is `[x0, y0, x1, y1]` in window logical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Pending {
    pub rect: [f32; 4],
    pub note: String,
}

/// Where a snip is, from arming to saving.
#[derive(Resource, Debug, Default, Clone)]
pub struct SnipEdit {
    /// Armed by `Cmd+\`, waiting for a drag.
    pub armed: bool,
    /// Where the drag began, while the button is down.
    pub from: Option<Vec2>,
    /// Released and waiting for its note.
    pub pending: Option<Pending>,
    /// Confirmed with Enter, waiting for [`snip_commit`] to take it in the same frame.
    pub confirmed: Option<Pending>,
}

impl SnipEdit {
    /// Does the snip own the pointer right now? Placement drivers yield to it, so a press that
    /// begins a drag cannot also place a piece.
    pub fn claims_pointer(&self) -> bool {
        self.armed || self.from.is_some() || self.pending.is_some()
    }

    fn disarm(&mut self) {
        self.armed = false;
        self.from = None;
    }
}

/// The rectangle with its corners in order, whichever way the drag went.
fn rect_between(a: Vec2, b: Vec2) -> [f32; 4] {
    [a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)]
}

/// `Cmd+\` arms the snip, and pressed again it disarms. Escape disarms while no note is open.
pub fn snip_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    live: Res<Live>,
    mut snip: ResMut<SnipEdit>,
    mut state: ResMut<EditorState>,
) {
    if keys::just_pressed(&keyboard, *live, Action::Snip) {
        if snip.armed || snip.from.is_some() {
            snip.disarm();
            state
                .status
                .note("snip cancelled, nothing was saved".to_owned());
        } else {
            snip.armed = true;
            state
                .status
                .note("snip armed: drag a rectangle over the editor".to_owned());
        }
        return;
    }
    if (snip.armed || snip.from.is_some()) && keyboard.just_pressed(KeyCode::Escape) {
        snip.disarm();
        state
            .status
            .note("snip cancelled, nothing was saved".to_owned());
    }
}

/// A press begins the rectangle and a release ends it. Releasing too small a rectangle is a
/// refusal, in the problem slot, and the snip stays armed so the person can drag again.
pub fn snip_pointer(
    pointer: Res<Pointer>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut snip: ResMut<SnipEdit>,
    mut state: ResMut<EditorState>,
) {
    if !snip.armed && snip.from.is_none() {
        return;
    }
    let Some(here) = pointer.0 else {
        return;
    };
    if snip.from.is_none() {
        if mouse.just_pressed(MouseButton::Left) {
            snip.from = Some(here);
        }
        return;
    }
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    let Some(from) = snip.from.take() else {
        return;
    };
    let rect = rect_between(from, here);
    if rect[2] - rect[0] < MIN_SIDE_PX || rect[3] - rect[1] < MIN_SIDE_PX {
        state.status.problem(
            "that area is too small to be a picture, so nothing was snipped. Drag a wider rectangle."
                .to_owned(),
        );
        return;
    }
    snip.armed = false;
    snip.pending = Some(Pending {
        rect,
        note: String::new(),
    });
}

/// While a note is open, every key belongs to it. Enter confirms, Escape cancels and writes
/// nothing, and the rest edit the line.
pub fn snip_note_keys(
    mut events: MessageReader<KeyboardInput>,
    mut snip: ResMut<SnipEdit>,
    mut state: ResMut<EditorState>,
) {
    if snip.pending.is_none() {
        // Drain before leaving, as `pin_reason_keys` does, so the key that opened the note is not
        // read as its first character next frame.
        events.clear();
        return;
    }
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Enter => {
                snip.confirmed = snip.pending.take();
                return;
            }
            Key::Escape => {
                snip.pending = None;
                state
                    .status
                    .note("snip cancelled, nothing was saved".to_owned());
                return;
            }
            Key::Backspace => {
                if let Some(p) = snip.pending.as_mut() {
                    p.note.pop();
                }
            }
            Key::Space => {
                if let Some(p) = snip.pending.as_mut()
                    && p.note.chars().count() < NOTE_CAP
                {
                    p.note.push(' ');
                }
            }
            Key::Character(c) => {
                if let Some(p) = snip.pending.as_mut() {
                    for ch in c.chars().filter(|ch| !ch.is_control()) {
                        if p.note.chars().count() < NOTE_CAP {
                            p.note.push(ch);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Takes a confirmed snip, gathers its metadata from the editor as it stands, and asks the
/// window for a picture. The file is written when the readback lands, by the observer.
///
/// A headless app has no render world ([`CanRender`] is false or absent), so the snip is refused
/// here, before anything is asked of the renderer.
#[allow(clippy::too_many_arguments)]
pub fn snip_commit(
    mut snip: ResMut<SnipEdit>,
    mut state: ResMut<EditorState>,
    project: Option<Res<Project>>,
    open: Option<Res<OpenMap>>,
    can_render: Option<Res<CanRender>>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<MainCamera>>>,
    pointer: Res<Pointer>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    placed: Query<(&Placement, &GlobalTransform)>,
    mut commands: Commands,
) {
    let Some(pending) = snip.confirmed.take() else {
        return;
    };
    let Some(project) = project.as_deref() else {
        state
            .status
            .problem("no project is open, so nothing was saved.".to_owned());
        return;
    };
    if !can_render.is_some_and(|c| c.0) {
        state.status.problem(
            "this editor has no render world, so no picture could be taken; nothing was saved."
                .to_owned(),
        );
        return;
    }
    let Some(window) = window else {
        state
            .status
            .problem("there is no window to take a picture of; nothing was saved.".to_owned());
        return;
    };
    let scale = window.resolution.scale_factor();
    let r = pending.rect;
    let physical = [
        (r[0] * scale).round() as i64,
        (r[1] * scale).round() as i64,
        (r[2] * scale).round() as i64,
        (r[3] * scale).round() as i64,
    ];

    let taken_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let stem = format!("snip-{taken_at_ms}");
    let map = open
        .as_deref()
        .map(|o| o.map.name.clone())
        .unwrap_or_default();

    let mut camera_view = None;
    let mut cursor_world = None;
    let mut placements = Vec::new();
    if let Some(camera) = camera.as_deref() {
        let (cam, cam_tf) = *camera;
        camera_view = Some(CameraView {
            translation: cam_tf.translation().to_array(),
            forward: cam_tf.forward().as_vec3().to_array(),
        });
        cursor_world = cursor_ground(pointer.0, cam, cam_tf).map(|v| v.to_array());
        for (placement, tf) in &placed {
            let Ok(at) = cam.world_to_viewport(cam_tf, tf.translation()) else {
                continue;
            };
            if at.x >= r[0] && at.x <= r[2] && at.y >= r[1] && at.y <= r[3] {
                placements.push(PlacedIn {
                    id: placement.0.clone(),
                    world: tf.translation().to_array(),
                });
            }
        }
    }

    let record = SnipRecord {
        stem: stem.clone(),
        taken_at_ms,
        map,
        active_tool: format!("{:?}", state.tool),
        note: pending.note,
        rect_window_px: r,
        scale_factor: scale,
        rect_picture_px: physical,
        size: [0, 0],
        picture: [0, 0],
        picture_file: String::new(),
        camera: camera_view,
        cursor_world,
        placements,
    };
    let dir = project.root.join(SNIP_DIR);
    let root: PathBuf = project.root.clone();

    commands.spawn(Screenshot::primary_window()).observe(
        move |shot: On<ScreenshotCaptured>, mut state: ResMut<EditorState>| {
            let rgba = match shot.image.clone().try_into_dynamic() {
                Ok(dynamic) => dynamic.to_rgba8(),
                Err(e) => {
                    state.status.problem(format!(
                        "the capture cannot become a picture ({e:?}); nothing was saved."
                    ));
                    return;
                }
            };
            let (w, h) = rgba.dimensions();
            match write_snip(
                &dir,
                &stem,
                (w, h, rgba.into_raw()),
                physical,
                record.clone(),
            ) {
                Ok(files) => {
                    let shown = files
                        .png
                        .strip_prefix(&root)
                        .unwrap_or(&files.png)
                        .display()
                        .to_string();
                    state.status.note(format!("snip saved to {shown}"));
                }
                Err(e) => state.status.problem(e),
            }
        },
    );
}

/// The one overlay entity the drag and the note are drawn with. Despawned and respawned each
/// frame, so it is gone before the frame a capture reads.
#[derive(Component)]
pub struct SnipOverlay;

/// Draws the rectangle while it is dragged, and the note while it is typed. Runs after the systems
/// that change the snip, so the frame a note is confirmed the overlay is already gone.
pub fn snip_overlay(
    mut commands: Commands,
    snip: Res<SnipEdit>,
    pointer: Res<Pointer>,
    old: Query<Entity, With<SnipOverlay>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let (rect, label) = match (&snip.pending, snip.from, pointer.0) {
        (Some(p), _, _) => (
            p.rect,
            format!("note: {}|  (Enter saves, Escape cancels)", p.note),
        ),
        (None, Some(from), Some(here)) => (
            rect_between(from, here),
            "drag over the area to snip".to_owned(),
        ),
        _ => return,
    };
    commands
        .spawn((
            SnipOverlay,
            Node {
                // PLACES-ITSELF-OK: the drag rectangle is placed at the pointer over the whole window, so it is not a panel and has no layout to join.
                position_type: PositionType::Absolute,
                left: Val::Px(rect[0]),
                top: Val::Px(rect[1]),
                width: Val::Px(rect[2] - rect[0]),
                height: Val::Px(rect[3] - rect[1]),
                border: UiRect::all(Val::Px(chrome::SNIP_EDGE_W)),
                ..default()
            },
            BorderColor::all(chrome::ACCENT),
            BackgroundColor(chrome::SNIP_WASH),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(label),
                chrome::font(chrome::text::LABEL),
                TextColor(chrome::TEXT),
                Node {
                    // PLACES-ITSELF-OK: a label riding the drag rectangle's top edge, not a panel; it is positioned against the rectangle it names.
                    position_type: PositionType::Absolute,
                    top: Val::Px(-22.0),
                    left: Val::Px(0.0),
                    ..default()
                },
            ));
        });
}

/// Registers the snip. Its own plugin so the editor's `Update` tuple stays under Bevy's cap.
pub struct SnipPlugin;

impl Plugin for SnipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnipEdit>()
            .add_systems(Update, snip_key.in_set(Phase::Act))
            .add_systems(Update, snip_pointer.in_set(Phase::Act).after(snip_key))
            .add_systems(Update, snip_note_keys.in_set(Phase::Text))
            .add_systems(
                Update,
                snip_commit.in_set(Phase::Text).after(snip_note_keys),
            )
            .add_systems(
                Update,
                snip_overlay
                    .after(snip_pointer)
                    .after(snip_commit)
                    .after(snip_note_keys),
            );
    }
}
