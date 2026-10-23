//! **A capture sequence asks for one frame at a time, and a frame that never lands ends the run.**
//!
//! The regression test for B-038: `emerge_capture_sequence` at 15 fps dropped frames with
//! "Duplicate render target" and never reported done.
//!
//! # What is being proved
//!
//! **Bevy takes at most one screenshot per render target per frame.** A second screenshot
//! requested for the same target in the same frame is despawned without its observer firing, so
//! its PNG is never written. The sequencer's frames all go to the same surface image, so a
//! sequencer that spawns every frame that has come due in one update loses every frame after the
//! first. The run then waits for a `written` count that can never be reached, and never finishes.
//!
//! **So the sequencer holds one capture in flight.** A due frame waits until the previous PNG has
//! landed. And a capture that never lands is a refusal with a reason after a bounded wait, not a
//! run that waits forever.
//!
//! # How the test runs without a GPU
//!
//! The harness is headless, so no screenshot can ever be read back. That is the point: nothing
//! lands, so any screenshot spawned and left in the world is one that is *in flight*. The test
//! lets the editor's `Surface` and the `CanRender` flag stand for a real renderer, and counts the
//! `Screenshot` entities the sequencer has spawned. The clock is set by
//! `TimeUpdateStrategy::ManualDuration`, so a big jump in time makes many frames due in one update,
//! which is the condition that broke.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::Screenshot;
use bevy::time::TimeUpdateStrategy;
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::sequencer::{CanRender, SequenceParams, Sequencer};
use bevy_emerge_mapper::surface::Surface;

mod fixtures;

/// **A project with one map and one piece.**
fn project(name: &str) -> std::path::PathBuf {
    fixtures::Fixture::new(name)
        .descriptor("site/wall", "alpha")
        .place("site/wall", (0.0, 0.0))
        .build("m")
}

/// **An editor that behaves as if it can render**, with its clock under the test's control.
///
/// **The surface is a default handle** — nothing renders it headless, so no readback ever lands,
/// and every screenshot the sequencer spawns stays in the world where the test can count it.
fn rendering_editor(name: &str, step: Duration) -> App {
    let root = project(name);
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    // **`Time<Virtual>` clamps each step to its `max_delta` (250 ms by default)**, and the sequencer
    // reads that clock. Without this line a "one second" step advances the clock a quarter second.
    app.world_mut()
        .resource_mut::<bevy::time::Time<bevy::time::Virtual>>()
        .set_max_delta(step);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
    app.insert_resource(CanRender(true));
    app.insert_resource(Surface {
        image: Handle::default(),
    });
    app
}

/// **How many screenshots are in flight**, which is how many the sequencer has spawned.
fn in_flight(app: &mut App) -> usize {
    app.world_mut()
        .query::<&Screenshot>()
        .iter(app.world())
        .count()
}

fn params(label: &str, seconds: f32, fps: u32) -> SequenceParams {
    SequenceParams {
        label: label.to_owned(),
        out_dir: std::env::temp_dir()
            .join(format!(
                "emerge-one-at-a-time-{}-{label}",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned(),
        seconds,
        fps,
        operation: None,
    }
}

/// **A jump of a second makes fifteen frames due at once.** The sequencer must still ask for one.
#[test]
fn a_burst_of_due_frames_is_asked_for_one_at_a_time() {
    let mut app = rendering_editor("one_at_a_time", Duration::from_secs(1));
    for _ in 0..2 {
        app.update();
    }
    {
        let mut sequencer = app.world_mut().resource_mut::<Sequencer>();
        sequencer
            .start(&params("burst", 2.0, 15), 0.0)
            .expect("the sequence starts");
    }
    // **Exactly one, not at most one.** `<= 1` also passes when nothing is spawned at all, so the
    // first update must have put frame 0 in flight, and it must stay there: four seconds is under
    // the five-second landing timeout, so no frame lands and none is replaced.
    for _ in 0..4 {
        app.update();
        let n = in_flight(&mut app);
        assert_eq!(
            n, 1,
            "{n} screenshots are in flight, and the sequence must hold exactly one: Bevy keeps one \
             per render target per frame, so a second is despawned without a PNG (\"Duplicate \
             render target\")"
        );
    }
}

/// **A frame whose PNG never lands ends the run with a reason**, rather than waiting forever.
#[test]
fn a_frame_that_never_lands_ends_the_run_with_a_reason() {
    let mut app = rendering_editor("never_lands", Duration::from_secs(1));
    for _ in 0..2 {
        app.update();
    }
    let id = {
        let mut sequencer = app.world_mut().resource_mut::<Sequencer>();
        sequencer
            .start(&params("never", 0.2, 5), 0.0)
            .expect("the sequence starts")
    };
    let mut status = serde_json::Value::Null;
    for _ in 0..12 {
        app.update();
        status = app.world().resource::<Sequencer>().status(id);
        if status["status"] != "running" {
            break;
        }
    }
    assert_eq!(
        status["status"], "failed",
        "the run never finished: {status}"
    );
    let reason = status["error"].as_str().unwrap_or_default();
    assert!(
        reason.contains("did not land"),
        "the run failed without saying why the frame is missing: {status}"
    );
}
