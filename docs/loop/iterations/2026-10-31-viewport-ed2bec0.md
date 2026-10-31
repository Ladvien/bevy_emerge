# 2026-10-31 viewport: the black viewport with `bevy_elements` is fixed on a display (big, Xvfb); its file-removal half is not

Focus: #193/#194 (bug) · Commits: ed2bec0 (+ the changelog commit) · Machine: laptop (edit, commit) + big (build, live probe, gate) · Outcome: shipped

## Summary
Pass 159 left #193/#194 for "a display and a warm build". big has Xvfb and an RTX 3090 with the NVIDIA Vulkan ICD, so this pass built the editor there with `--features bevy_elements` (5 min cold) and probed it live over BRP (`bevy_fairy/screenshot` with `source: offscreen`, `world.mutate_components`). With the file present the viewport was 1 colour (the ground colour). With the interface camera inactive it was 455; with its `msaa_writeback` set to `Off` it was 456: sky, box and grid. Cause: the interface camera shares the ground camera's texture, and Bevy 0.20's `MsaaWriteback::Auto` copies the opaque ground into its sampled texture instead of honouring its `Custom(Color::NONE)` clear, so its premultiplied write put the ground over the world. Pass 159's reading that "the per-camera clear is honoured" was true of the clear and missed the writeback that precedes it.

## Baseline
Gate: green on main. `accept.py`: see Acceptance. Load 2-3, laptop swap 5.8 of 6.1 GB (so build and gate ran on big), big 18 GB available.

## Bug fixed
### #193/#194 step 2 (file present): flat viewport
Must-fix: blocks yes (six clips and the environment preview), common yes (every project with the file), no workaround yes (delete the file).
Root cause: `MsaaWriteback::Auto` on the interface camera (above). Fix: `environment::composite_surface` sets it `Off` while dressed, the default when the file goes.
Regression check: `tests/the_interface_does_not_write_the_ground_back_over_the_world.rs` (failed on the base: `left: Auto right: Off`, passes). It asserts the setting, not pixels: no GPU in a test. The pixel evidence is the live capture: `docs/changelog/media/2026-10-31-193-world-under-sky.{mp4,png}`.

## Not fixed: step 3 (file removed, the world does not come back)
Pass 159's suggestion, "undress also removes `Hdr`", is wrong on its own: tried live, removing `Hdr` while the crate's atmosphere and fog components remain made the pipelines fail validation (`Caught rendering error: Validation Error ... Quitting the application`), and the editor exited. Undress has to remove the crate's dressing first, which is bevy-coven/bevy_elements#55; the editor can drive it through `ElementsCamera`'s own flags (all false, then remove) once #55 is decided. #193 and #194 stay open.

## Also seen
At `clock.hour = 22.0` the viewport goes white (auto-exposure at night over the sky); at noon the sky is dark blue and at 6.5 bright. Both are the crate's exposure behaviour on a camera whose clear colour is transparent; not investigated.

## Review
`reviewer` subagent: see the log line.

## Test diffs
One new test file and its `mod` line in `tests/all.rs` (the owner's exception for a new editor test file). No existing test touched. `family_settings_reload::an_edited_settings_file_reaches_the_editor_and_a_bad_one_is_a_problem` fails with `--features bevy_elements` on the base too (#152, half 2, the fixture text: protected).

## Release
Not made: nothing packaged changed (the change is behind the `bevy_elements` feature of the unpublished editor).

## Next focus
#129's next slice (an unmigrated action), or #193 step 3 when bevy_elements#55 lands: drive the crate's flags to false, then remove `ElementsCamera` and `Hdr`.
