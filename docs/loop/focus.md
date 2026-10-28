# Focus for the bevy_emerge loop

The owner's steer for `/build-loop`. A pass reads it in Orient and uses it when it chooses the enhancement (Phase 2); bugs and filed bug issues still come first. A pass never edits this file. To change the focus, write it below the line, commit and push: the loop's clone fast-forwards at the start of its next pass, with no restart. Nothing below the line, or no file, means no focus.

---

Notes a person leaves for an agent first; then hot reloading and performance. When more than one is ready, take them in this order:

1. **Notes for agents: issues #89 → #95, the main thread** (the owner's, labelled `owner`, 2026-10-28; they replace series S-004's steps S4.2–S4.5). In order: #89 write a note, #90 the list and history, #91 agents claim notes over MCP, #92 the fence, questions and report, #93 review and undo a note, #94 Send can launch an agent, #95 end to end. Take the first whose **Depends on** issues are closed, one verified slice per pass. The **Decisions already made** in each issue are the design: follow them rather than stopping to ask. #89's protected key edit is an owner exception, listed in `build-loop.md`'s "The protected evaluator".
2. **E-004** (two writers never share an operation-log sequence number) as soon as it is open and #91 is next, because #91 depends on it.
3. **Hot reloading: series S-003**, when the next notes issue is blocked or waits for a decision. S3.7 is blocked on the owner (#86).
4. **Performance**: E-051 (kit ops clone the whole `Project` per click), then E-023, then the slowest path `perf.py` names. A pass that touches anything run every frame reports `perf.py` before and after (E-011 made that possible), and a regression it causes is a bug.
5. **When 1–4 are all blocked, the code still moves: take the first of these that is open, in this order.** B-030 (undo across a rename), E-066 (`map_remove`, so Cmd+Z takes back a new map), E-069 (an agent shows its plan before it acts), E-072 (an agent authors a composition), E-012 (ratchets see inside `SystemParam` bundles). Each is a real item with a failing-first test; none needs the owner.

The owner wants the code to evolve every pass (2026-10-28). Earlier log lines that say "same blockers, nothing shipped" describe the blocked items only; they are not a reason for this pass to stop, because item 5 is never all blocked at once. A pass ends with nothing shipped only after it has tried an item from this list and logged why it failed.
