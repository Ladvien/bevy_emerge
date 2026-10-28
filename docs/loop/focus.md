# Focus for the bevy_emerge loop

The owner's steer for `/build-loop`. A pass reads it in Orient and uses it when it chooses the enhancement (Phase 2); bugs and filed bug issues still come first. A pass never edits this file. To change the focus, write it below the line, commit and push: the loop's clone fast-forwards at the start of its next pass, with no restart. Nothing below the line, or no file, means no focus.

---

Performance, hot reloading, and notes a person leaves for an agent. When more than one is ready, take them in this order:

1. **Measure first: E-011** (frame time through MCP, so the editor can be told apart from a loaded machine). It is small and every performance claim after it rests on it. From then on, a pass under this focus that touches anything run every frame reports `perf.py` before and after, and a regression it causes is a bug.
2. **Notes for agents: series S-004**, the main thread. Take its next open step. The design defaults are in the series' `evidence`; follow them rather than stopping to ask.
3. **Hot reloading: series S-003**, when S-004's next step is blocked or waits for a decision. Its next open step is S3.8; S3.7 is blocked on the owner.
4. **Performance work beyond E-011**: E-051 (kit ops clone the whole `Project` per click), then E-023, then the slowest path `perf.py` names, when 2 and 3 are both blocked.
5. **When 2, 3 and 4 are all blocked, the code still moves: take the first of these that is open, in this order.** B-030 (undo across a rename), E-004 (two agents claiming one oplog sequence number; notes are worked by agents, so this is on the notes path), E-066 (`map_remove`, so Cmd+Z takes back a new map), E-069 (an agent shows its plan before it acts), E-072 (an agent authors a composition), E-012 (ratchets see inside `SystemParam` bundles). Each is a real item with a failing-first test; none needs the owner.

The owner wants the code to evolve every pass (2026-10-28). Earlier log lines that say "same blockers, nothing shipped" describe the blocked items only; they are not a reason for this pass to stop, because item 5 is never all blocked at once. A pass ends with nothing shipped only after it has tried an item from this list and logged why it failed.
