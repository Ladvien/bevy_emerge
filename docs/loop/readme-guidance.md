# How to write the bevy_emerge README: guidance from the Home Still corpus

These sources were found with `distill_search` on 2026-10-22. Cite them by DOI if a rationale is needed; never quote them in the README itself.

## What the research says

1. **What a README must answer** (Prana et al. 2018, *Categorizing the Content of GitHub README Files*, 10.1007/s10664-018-9660-3; GitHub's own guidance quoted there):
   - **what** the project does;
   - **why** it's useful: rare in READMEs, and exactly what readers lack;
   - **how** to start, plus the common scenarios;
   - **when**: status, what works and what is in progress;
   - **who**: licence, maintainer;
   - **references**: links to the full docs;
   - **contribution**.

   Readers surveyed expected usage, installation, prerequisites, licence, purpose and audience, known bugs and troubleshooting, and screenshots.
2. **Shape of popular READMEs** (Venigalla & Chimalakonda 2022, 10.48550/arXiv.2206.10772): they're organised with **lists and images**, link out to deeper docs, and carry contribution guidelines and references.
3. **Treat the README as the only doc they'll read** (Lee 2018, *Ten simple rules for documenting scientific software*, 10.1371/journal.pcbi.1006561, rules 3–4):
   - put the quickstart in the README;
   - say how to install, configure and test, where the full docs are, and the licence;
   - the quickstart is a "dating profile": it shows the strengths, gives a feel, and entices;
   - check it by having someone who has never used the tool follow it;
   - a status badge (CI) signals health at a glance.
4. **Minimalism** (Carroll 2014, *Creating Minimalist Instruction*, 10.14434/ijdl.v5i2.12887):
   - get people doing a real task immediately;
   - lean on what they already know (game dev, maps, kits), not on exhaustive step lists;
   - be brief: the Minimal Manual was a quarter of the official length, and users learned more in less time;
   - treat errors as normal and say how to recover;
   - end with small open-ended things to try.
5. **Demo videos** (van der Meij & van der Meij 2016, 10.1007/s11251-016-9394-9; 2017, 10.1007/s11423-017-9560-z):
   - demonstrations work best **anchored in the user's task** (show why the result matters);
   - **segment** them into short clips, sequenced simple to complex;
   - a one-line **preview** of the goal before a clip, and a one-line **review** after it, measurably improve recall and motivation.
6. **Onboarding** (Hachet et al. 2023, *User Onboarding in VR*, 10.1145/3544548.3581211):
   - show the final goal;
   - don't go too fast;
   - name the exact key or button for every action. Images without the key confuse people.

## What that means for this README

- **Open with one sentence:** what bevy_emerge is and who it's for. Follow it with a **hero GIF**: about 10–15 s, looping, 960 px wide, ≤ ~5 MB, showing the core loop (a person marks or describes, an agent acts, the result is visible, Cmd+Z takes it back).
- **Why it's useful**, as 3–5 bullets in the developer's terms, each true today and checked against the code:
  - the human describes and an agent drives, through MCP;
  - one undo history for both;
  - the agent sees what you see (stills, sequences, snips);
  - content is data that hot-reloads without a rebuild;
  - layouts are learned from an example block.
- **Status:** a CI badge, plus an honest "works today / in progress" list. No overclaiming: say there's no crates.io release yet.
- **Quickstart:** prerequisites, then the exact command to open a project and a map, then the exact command and config to attach an agent through the MCP server. Verify every command by running it from a clean state.
- **Common tasks:** each is one short segmented GIF (4–8 s, ≤ 3 MB). Before each, a one-line preview of the goal; after it, a one-line review that names the exact keys or tools. Add a one-line "if it doesn't work" where a real failure is common, such as a refused save or the editor not reachable.
  - frame, pan and zoom;
  - place, then Cmd+Z;
  - an agent turns or removes a piece, then Cmd+Z;
  - snip an area for the agent (Cmd+\, drag, note, Enter);
  - optionally the Kit door.
- **For agents:** the MCP config snippet, the tool families, and a link to `docs/mcp-tools.md`.
- **How it fits your game:** the crates (`bevy_emerge_core`, `bevy_emerge`, `bevy_emerge_mapper`, the MCP server) and `<root>/assets/emerge`. Keep it short and link the crate READMEs.
- **Then:** links to the guide, plus contributing (the loop docs) and the licence.
- **Concise throughout:**
  - lists and images over prose;
  - no internal jargon (no "doors" without explaining them once, no backlog ids);
  - no history of changes. The README describes the editor as it is now.