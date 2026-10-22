//! **Capturing a sequence** — the editor takes the frames, so frame 0 is the state before.
//!
//! # Why this is on the editor's side and not the MCP server's
//!
//! **The brief's rule is "frame 0 is the state before it"**, and only the editor can promise
//! that. The screenshot reads the *rendered* frame, not the ECS state — so an operation applied
//! in the same frame a capture is spawned would show in frame 0, and the sequence would begin
//! with the thing it was supposed to show the absence of.
//!
//! **So the editor takes the frames**, times them itself, and applies the operation **after
//! frame 0's observer has fired**. The MCP server asks for a sequence and polls for its result.
//!
//! # What it writes
//!
//! One PNG per frame, named `f{i:03}_t{t:.2}s.png`, **and never over an existing file** — a
//! capture that silently replaced the last run's frames would make two runs indistinguishable.
//! Beside them, a `sequence.json` with the state at each frame, so a frame is never a picture
//! without a state dump.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::surface::Surface;

/// **How a capture sequence is asked for.**
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceParams {
    /// **The sequence's name** — a slash-separated snake_case id. It names the directory the
    /// frames land in, and it is refused if it is not an id.
    pub label: String,
    /// **Where the frames go.** An absolute path; the editor writes there and nowhere else.
    pub out_dir: String,
    /// **How long the sequence runs**, in seconds.
    #[serde(default = "two_seconds")]
    pub seconds: f32,
    /// **How many frames a second.** Four, or two for a long slow operation.
    #[serde(default = "four_fps")]
    pub fps: u32,
    /// **The operation to apply, once frame 0 is in.** Optional: without it the sequence is the
    /// settle alone, which is what a reviewer wants when they are checking that nothing moved.
    #[serde(default)]
    pub operation: Option<Operation>,
}

/// **The operation a sequence applies.**
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// **The operation's name**, as `emerge/operation_list` gives it.
    pub name: String,
    /// **Its parameters.**
    #[serde(default)]
    pub parameters: Value,
}

/// **Two seconds, which is one contact sheet at four frames a second.**
fn two_seconds() -> f32 {
    2.0
}

/// **Four frames a second, the brief's default.**
fn four_fps() -> u32 {
    4
}

/// **Whether this editor can write a frame at all.**
///
/// **Settled once, at startup.** A headless editor has no render world, so a screenshot never
/// produces a readback, and a sequence there can promise a *schedule* but not a *picture*. The
/// run says so rather than finishing as though the pictures were there.
#[derive(Clone, Copy, Debug, Resource)]
pub struct CanRender(pub bool);

/// **What a sequence is doing right now.**
#[derive(Resource, Default)]
pub struct Sequencer {
    /// **The run in flight**, if there is one.
    run: Option<Run>,
    /// **The runs that have finished**, newest last, so a caller can ask about one by id.
    finished: Vec<Finished>,
}

/// **A run in flight.**
struct Run {
    id: u64,
    label: String,
    out_dir: std::path::PathBuf,
    /// **When it started**, in seconds since the editor started.
    started: f32,
    seconds: f32,
    fps: u32,
    /// **The next frame's index.**
    next: u32,
    /// **The operation, applied once frame 0 is in.**
    operation: Option<Operation>,
    /// **Whether the operation has been applied yet.**
    applied: bool,
    /// **When it was applied**, in seconds from the start.
    applied_at: Option<f32>,
    /// **What it said.**
    operation_result: Option<Value>,
    /// **The frames asked for, and when.** The manifest's list.
    asked: Vec<Frame>,
    /// **The frames whose PNG has landed.** The run is finished when this has caught up.
    written: Vec<String>,
}

/// **One frame, written.**
#[derive(Debug, Clone, Serialize)]
pub struct Frame {
    /// **Its index.**
    pub index: u32,
    /// **When it was scheduled, in seconds from the start.**
    pub t_s: f32,
    /// **When it was actually taken**, in seconds from the start. The two differ: the editor
    /// renders at whatever rate it renders at, and the schedule is the *intent*.
    pub actual_t_s: f32,
    /// **Where it landed.**
    pub path: String,
}

/// **A run that has finished.**
#[derive(Debug, Clone, Serialize)]
pub struct Finished {
    pub id: u64,
    pub label: String,
    pub out_dir: String,
    pub frames: Vec<Frame>,
    /// **When the operation was applied**, in seconds from the start. `None` when there was none.
    pub operation_applied_t_s: Option<f32>,
    /// **What the operation said.**
    pub operation_result: Option<Value>,
    /// **Whether it failed, and why.**
    pub error: Option<String>,
}

/// **The parameters of a status query.**
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusParams {
    /// **Which run.**
    pub id: u64,
}

impl Sequencer {
    /// **Start a sequence, and return its id.**
    ///
    /// **A second sequence while one is running is refused.** Two sequences writing frames at
    /// once would interleave their file names, and a reader could not tell which frame belongs
    /// to which.
    pub fn start(&mut self, p: &SequenceParams, now: f32) -> Result<u64, String> {
        if self.run.is_some() {
            return Err("a sequence is already running. Wait for it to finish.".to_owned());
        }
        if !bevy_emerge_core::naming::is_id(&p.label) {
            return Err(format!(
                "`{}` is not a sequence name. Names are snake_case, with `/` between the parts.",
                p.label
            ));
        }
        if !(0.0..=10.0).contains(&p.seconds) || p.seconds <= 0.0 {
            return Err(format!(
                "a sequence runs for between a moment and ten seconds; `{}` is neither.",
                p.seconds
            ));
        }
        if p.fps != 4 && p.fps != 2 {
            return Err(format!(
                "a sequence is captured at four frames a second, or two for a long slow operation; \
                 `{}` is neither.",
                p.fps
            ));
        }
        let out_dir = std::path::PathBuf::from(&p.out_dir);
        std::fs::create_dir_all(&out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
        let id = self.finished.len() as u64 + 1;
        self.run = Some(Run {
            id,
            label: p.label.clone(),
            out_dir,
            started: now,
            seconds: p.seconds,
            fps: p.fps,
            next: 0,
            operation: p.operation.clone(),
            applied: false,
            applied_at: None,
            operation_result: None,
            asked: Vec::new(),
            written: Vec::new(),
        });
        Ok(id)
    }

    /// **What is happening, in one value.**
    pub fn status(&self, id: u64) -> Value {
        if let Some(run) = &self.run
            && run.id == id
        {
            return json!({
                "status": "running",
                "id": run.id,
                "label": run.label,
                "frames": run.asked.len(),
                "written": run.written.len(),
                "expected_frames": (run.seconds * run.fps as f32) as u32,
            });
        }
        match self.finished.iter().find(|f| f.id == id) {
            Some(f) => json!({
                "status": if f.error.is_some() { "failed" } else { "done" },
                "id": f.id,
                "label": f.label,
                "out_dir": f.out_dir,
                "frames": f.frames,
                "operation_applied_t_s": f.operation_applied_t_s,
                "operation_result": f.operation_result,
                "error": f.error,
            }),
            None => json!({"status": "unknown", "id": id}),
        }
    }
}

/// **The plugin that runs the sequencer.**
pub struct SequencerPlugin;

impl Plugin for SequencerPlugin {
    fn build(&self, app: &mut App) {
        // **Whether a frame can be written at all is settled here, once.**
        //
        // **A headless editor has no render world** — `RenderPlugin` creates the `RenderApp`
        // sub-app only when a backend was available — and so a screenshot cannot produce a
        // readback. **The sub-app's presence is the fact**, checked at startup rather than
        // guessed at every frame, and a run that waited for a frame that cannot exist would
        // wait forever.
        let can_render = app.get_sub_app(bevy::render::RenderApp).is_some();
        app.init_resource::<Sequencer>()
            .insert_resource(CanRender(can_render))
            .add_systems(Update, drive_sequencer.in_set(crate::keys::Phase::Sense));
    }
}

/// **What the sequencer reads and writes, bundled to stay under the parameter limit.**
///
/// **The redraw flag is here** because a turn applied inside a sequence must reach the screen the
/// same way an agent's turn does.
#[derive(bevy::ecs::system::SystemParam)]
struct Operated<'w> {
    project: Option<ResMut<'w, crate::project::Project>>,
    open: Option<ResMut<'w, crate::project::OpenMap>>,
    due: Option<ResMut<'w, crate::watch::Due>>,
    state: Option<ResMut<'w, crate::editor::EditorState>>,
}

/// **Take the frames that are due, and finish the run when the last one is in.**
///
/// **The editor's own clock**, and not the caller's — the caller asked for a *duration* and the
/// editor knows what a frame costs.
fn drive_sequencer(
    time: Res<Time>,
    mut sequencer: ResMut<Sequencer>,
    surface: Option<Res<Surface>>,
    mut commands: Commands,
    mut operated: Operated,
    can_render: Option<Res<CanRender>>,
) {
    let Some(run) = sequencer.run.as_mut() else {
        return;
    };
    // **A headless editor has no surface, and so no renderer and no readback.** The frames are
    // still *asked for* -- the schedule is the sequencer's own and does not need a GPU -- and
    // the run still finishes. **What the headless case cannot do is produce a PNG**, and that is
    // said in the test rather than papered over.
    let surface = surface.as_ref();
    let now = time.elapsed_secs() - run.started;
    let total = (run.seconds * run.fps as f32) as u32;

    // **And a run that waited for a frame that cannot exist would wait forever**, so the
    // headless editor ends the run and says why.
    let can_render = can_render.map(|c| c.0).unwrap_or(false);

    // **The operation goes in AFTER frame 0 has been asked for.**
    //
    // **This is the whole reason the sequencer is on the editor's side.** The screenshot reads
    // the *rendered* frame, so an operation applied before frame 0's readback would be *in*
    // frame 0 — and the sequence would begin with the thing it was supposed to show the absence
    // of. **So the operation waits for frame 0**, and the delay is recorded in the manifest
    // rather than hidden.
    if !run.applied && run.next >= 1 {
        run.applied = true;
        if let Some(op) = run.operation.clone() {
            let mut ctx = crate::ops::Ctx {
                project: operated.project.as_deref().cloned(),
                open: operated.open.as_deref().cloned(),
                source: crate::ops::Source::Agent,
                session: crate::ops::session_id(),
            };
            // **Through the registry, and not around it.** The sequencer is a way of *when* an
            // operation happens, and it must not be a second way of *what* happens.
            let outcome = crate::ops::dispatch(&mut ctx, &op.name, &op.parameters);
            // **Redrawn, as `emerge/operation_apply` is.** The write-back is the watcher's own
            // write and the watcher ignores it, so a saved turn would stay off the screen until
            // some unrelated redraw. The sequence's frames are the proof, and they would show
            // nothing.
            if let (Some(due), Ok(crate::ops::Applied::Done { outcome, .. })) =
                (operated.due.as_mut(), &outcome)
            {
                due.map_replaced |= outcome.saved;
            }
            // **The agent's change joins the person's one undo history (B-019)**, as it does through
            // `emerge/operation_apply`: a turn a sequence applies is an agent's turn.
            if let Ok(crate::ops::Applied::Done { outcome, operation_id, .. }) = &outcome
                && let Some(state) = operated.state.as_mut()
            {
                state.record_logged(operation_id.clone(), outcome.summary.clone());
            }
            run.applied_at = Some(now);
            run.operation_result = Some(match outcome {
                Ok(crate::ops::Applied::Done { outcome, .. }) => {
                    json!({"outcome": "applied", "summary": outcome.summary, "saved": outcome.saved})
                }
                Ok(crate::ops::Applied::Refused(e)) => {
                    json!({"outcome": "refused", "refusal": e.0})
                }
                Err(e) => json!({"outcome": "refused", "refusal": e.0}),
            });
            // **The map goes back**, changed or not.
            if let Some(fresh) = ctx.open.take()
                && let Some(handle) = operated.open.as_deref_mut()
            {
                *handle = fresh;
            }
        }
    }

    // **A frame is due when its scheduled time has passed.** The schedule is the intent; the
    // editor renders when it renders, and the manifest records both.
    while run.next < total {
        let t = run.next as f32 / run.fps as f32;
        if now < t {
            break;
        }
        let index = run.next;
        run.next += 1;
        let path = run.out_dir.join(format!("f{index:03}_t{t:.2}s.png"));
        let actual = now;
        let Some(handle) = surface.map(|s| s.image.clone()).filter(|_| can_render) else {
            // **No renderer: the frame is asked for and nothing more.** The schedule still
            // advances, so the run finishes and the manifest still says when each frame was due.
            run.asked.push(Frame {
                index,
                t_s: t,
                actual_t_s: actual,
                path: path.to_string_lossy().into_owned(),
            });
            continue;
        };
        // **The frame is written by the observer, when the readback lands.**
        //
        // The screenshot is spawned here and the PNG is written when the capture completes,
        // because the bytes do not exist until then — and a frame written from a spawn would be
        // the frame *before* the one that was asked for.
        let out = path.clone();
        let written_path = out.to_string_lossy().into_owned();
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(handle))
            .observe(
                // **Bevy's own writer, and not a hand-rolled encoder.** `save_to_disk` is the
                // crate's PNG writer and it is what every other capture in this editor uses, so
                // a frame written here is a frame written the same way as one written by the
                // `Ctrl+P` path. **And it refuses to overwrite**, which is the property that
                // matters here: a capture that silently replaced the last run's frames would
                // make two runs indistinguishable.
                bevy::render::view::screenshot::save_to_disk(out.clone()),
            )
            // **And the write is recorded, so the run knows the frame has landed.**
            .observe(
                move |_t: On<bevy::render::view::screenshot::ScreenshotCaptured>,
                      mut sequencer: ResMut<Sequencer>| {
                    let Some(run) = sequencer.run.as_mut() else {
                        return;
                    };
                    run.written.push(written_path.clone());
                },
            );
        // **The frame is counted when it is ASKED for**, and the run is finished when it is
        // WRITTEN. The two are different moments — the readback lands a frame or two after the
        // spawn — and a run that finished on the ask would report itself done while its last
        // PNG was still being written. **A caller that then read the files would find one
        // missing**, and it would be missing for no reason the caller could see.
        run.asked.push(Frame {
            index,
            t_s: t,
            actual_t_s: actual,
            path: out.to_string_lossy().into_owned(),
        });
    }

    // **The run is finished when every frame has been WRITTEN**, and not when they were asked
    // for. The readback lands a frame or two after the spawn, so a run that finished on the ask
    // would report itself done while its last PNG was still on its way — and a caller that read
    // the files then would find one missing, for no reason it could see.
    let all_asked = run.asked.len() as u32 >= total;
    let all_written = run.written.len() as u32 >= total;
    if all_asked && (all_written || !can_render) {
        // **And a headless run says why there is no picture**, rather than finishing as though the
        // frames were there. A caller that is told "done" and then finds no file is owed an
        // answer; a caller that is told "no render world" knows exactly what happened.
        let error = if can_render {
            None
        } else {
            Some("this editor has no render world, so no frame could be written".to_owned())
        };
        finish_run(&mut sequencer, error);
    }
}

/// **Close the run and put it on the finished list.**
///
/// **One place, so the two ways a run ends — every frame written, or no frame possible — leave
/// the same shape behind.**
fn finish_run(sequencer: &mut Sequencer, error: Option<String>) {
    let Some(run) = sequencer.run.take() else {
        return;
    };
    let finished = Finished {
        id: run.id,
        label: run.label,
        out_dir: run.out_dir.to_string_lossy().into_owned(),
        frames: run.asked,
        operation_applied_t_s: run.applied_at,
        operation_result: run.operation_result.clone(),
        error,
    };
    sequencer.finished.push(finished);
}
