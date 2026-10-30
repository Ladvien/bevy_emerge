# Decision checks for the loop (issue #112)

`check.py` asks `bevy_emerge_decide`'s checks about captures and changelog clips. See its docstring (`python3 tools/decide/check.py --help`) for commands, endpoint variables and exit codes.

- `captures DIR`: the editor's checks (`crates/bevy_emerge_decide/fixtures/checks.ron`) on `DIR/<image>.png` plus `facts.json`.
- `clip ID`: `clip_checks.ron`'s `clip_matches_title` on a changelog clip's still and three frames, against the line's title.
- `calibrate-clips [--ask]`: builds the labelled clip set from `CHANGELOG.md` (each real clip with its own title = no, with another clip's title = yes) and, with `--ask`, measures the live model on it.

Only a check whose `calibration.agreement` is at or above `--bar` (0.9) can exit 1; everything else prints. Wiring it into the gate is the owner's change (`build-loop.md` is protected). Tests (`test_check.py`, `cargo test -p bevy_emerge_decide --test gate`) use a stand-in endpoint, never a live model.

Left for a live run: asking clef-flash the 8 clip samples (4 real clips, so the set is small) and the builtin checks' labelled captures, then writing the printed `calibration` into the check.
