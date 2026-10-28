# Running a Wiggum loop: a new one, or one that already exists

`tools/loop/README-wiggum.md` is how to *read* a loop that is already running. This file is how to
**start one on a machine that does not have it yet**, and the facts that cost a cycle to learn. Read it
before touching a unit or a plist.

## The four pieces

| Piece | Lives in | Owns |
|---|---|---|
| driver | a clone's `tools/loop/wiggum.sh` | *how* a pass runs: one fresh `omp -p`, never mid-edit, refuse a bad tree |
| command | `<repo>/.omp/commands/<name>.md` | *what* a pass does, with that repo's conventions |
| supervisor | `wiggum@<name>.service` (systemd) or `com.ladvien.wiggum-<name>.plist` (launchd) | restart, boot persistence, per-loop display |
| state | `~/.local/state/wiggum/<repo>/` | `driver.log`, `passes/pass-NNNN-*.log`, counters |

`omp` is on PATH only in a login shell; **every** supervisor must set `PATH` explicitly or every pass
dies `rc=127` while the job still reports `active`.

## Start a new loop, in order

1. **A dedicated clone, never a working tree.** `git clone` the repo as `<repo>_loop` (or a clone already
   used by that loop) and require `git status --short` empty at `main` before starting.
2. **Install the driver, and hide it from `git status`.** `tools/loop/` is deliberately untracked in the
   clones. The driver aborts a pass in a tree with >40 modified files, but `dirty=1` (its own directory)
   is still noise a pass will try to "fix" — add `/tools/loop/` to `.git/info/exclude`.
3. **Prove the command resolves** before writing any unit. From the clone root:

   ```sh
   omp --model ollama-cloud/deepseek-v4.1-flash:1017 --thinking low \
     -p "Reply with exactly one token: CMD-OK-<name> if a slash command named <command> exists"
   ```

4. **Write the supervisor** (templates below) and verify it: `systemd-analyze --user verify` must print
   nothing; `plutil -lint` must say `OK`.
5. **Build the gate's prerequisite.** A gate that is not built fails every pass for a reason that is not
   in the code — for `bevy_emerge` that is
   `cargo build -p bevy_emerge_mapper -p bevy_emerge_mcp` in the *same* `CARGO_TARGET_DIR` the unit sets,
   because `accept.py` asks `cargo metadata` where the binary is and exits if it is absent.
6. **Start, then read the log for `pass 1 START`.** `command=` and `model=` in that line are the proof
   the right thing is running. `rc=127` is PATH or a missing binary, never a model failure.

## Put a build where there is room, not where the checkout happens to be

**Never pick a compile host by habit.** Ask Home Cloud's OTEL first:

```sh
python3 tools/loop/otel_host_pick.py            # every host, ranked, with the exclusion reason
python3 tools/loop/otel_host_pick.py --pick     # one ssh destination
python3 tools/loop/otel_host_pick.py --arch arm64 --need-cores 6 --min-mem-gb 8
python3 tools/loop/otel_host_pick.py --json     # eligible + all, for another script
```

It is also installed on both hosts as **`otel-host-pick`** (`~/.local/bin`), which is what a pass in
*another* repo should use — `bevy_lights` and `bevy_society` do not contain this file, and a decision
about where to compile is not a property of one repo. The copy in this repo is the source of truth;
re-install it after changing it:

```sh
cat tools/loop/otel_host_pick.py | ssh big_mac 'cat > $HOME/.local/bin/otel-host-pick && chmod 755 $HOME/.local/bin/otel-host-pick'
```

Dashboard: `http://192.168.1.104:3000/d/default-home-dashboard/home` (Grafana 13.2.4, one Prometheus
datasource). The script reads Prometheus directly at `http://192.168.1.104:9090` — the Grafana proxy
works too (`/api/datasources/proxy/uid/prometheus/…`) but adds a hop for a tool that runs in a pass.

**Rank by idle cores, never by raw load.** `free_cores = cores × (1 − busy)`, with `busy` from
`rate(node_cpu_seconds_total{mode="idle"}[5m])`. Raw `load1` always favours the biggest machine: on
2026-10-26 `big` looked busiest (load 25) yet was a worse choice than `bmb`, which had 12.9 idle cores
against big's 4.

**It exits non-zero when nothing qualifies**, and prints why each host was excluded. Treat that as an
answer — a compile sent to a host with no free RAM fails after the linker, hours in. The observed split on
2026-10-26:

| host | cores | cpu% | free cores | note |
|---|---|---|---|---|
| `bmb` | 14 | 7.6 | 12.9 | M4 Pro; the right target for a heavy build |
| `Cs-MacBook-Pro` | 10 | 96.3 | 0.4 | **saturated** — four loops and a build share it |
| `big` | 24 | 83.2 | 4.0 | 2.8 GB available, swap already deep |
| `four`, `one`, `two`, `three` | 4 | <10 | ~3.7 | cheap, roomy, but small |
| `mac_air` | 10 | 32.9 | 6.7 | 7.4 GB disk free — too little for a Bevy `target/` |

Two measurement traps the script already handles, and that a hand-written query will hit: the macOS
exporter publishes **lowercase** memory names (`node_memory_free_bytes`, `node_memory_inactive_bytes`,
`node_memory_total_bytes`) and has no `MemAvailable`, so a Linux-only query silently excludes every Mac;
and `node_filesystem_avail_bytes` must be filtered to the real data mount, because a macOS
`AppTranslocation`/`nullfs` stub reports 191 GB on a host whose `/` has 22 GB.

## A killed pass must not strand its work

A pass commits at its **last** step, behind implement, gate and benchmark. So a pass killed at the
`WIGGUM_PASS_TIMEOUT` ceiling, or by the OOM killer, has usually done the work and never reaches its own
commit. Measured 2026-10-26: society accumulated **15** uncommitted files and lights **8** across killed
passes, none of it on a branch — and the clone was left **ahead of origin and behind it**, so the next
pass's `--ff-only` could never recover it (lights sat 1 ahead, 2 behind).

So the driver saves it to **a snapshot ref, never a commit on a branch**:

- `snapshot()` writes the tree with a private `GIT_INDEX_FILE` to
  `refs/wiggum/pass-N-{checkpoint,final}-<ts>`, parented on `HEAD`, then pushes **that ref**. The
  index, `HEAD` and the branch are untouched, so the pass's own `git status` is unchanged and this
  cannot move a branch anybody builds from;
- the private index is the load-bearing detail. `git stash create` moves the real index, which leaves
  the pass's `git status --short` empty mid-work and breaks the loop command's own rule that the tree
  must be clean before its commit step;
- a ref under `refs/wiggum/` is **not a branch**: `git branch` and `git log` do not show it, so a pass
  cannot mistake it for its own work. Recover one with `git -C <clone> log --all --oneline
  --grep='WIP(pass'` and then `git show <ref>`;
- a plain push only — never a force, never a rebase.

**Why not commit it to the branch, which is what this used to do.** Measured the same day: committing
the checkpoint to the pass's branch put **ungated work on `origin/main` as the branch tip** in all four
shared repos (lights `35c7824`, society `89d9563` — a mid-pass checkpoint —, elements `911ca5b`). Each
loop command builds a `loop/iter-N` branch from `main` and merges back **only after its gate**, so a
commit written straight onto `main` bypasses the gate, and Sanctuary path-deps `bevy_emerge_core`. A
safety net that skips the evaluator is worse than losing the last ten minutes.

Verified on a throwaway repo: after a checkpoint + final snapshot, `HEAD` was still `main`,
`origin/main` still had its original single commit, the dirty untracked file was still untracked (the
index was not moved), and both refs were on origin and recoverable.

**It is not a substitute for the pass committing.** A snapshot has not been through the gate, but unlike
a commit it is not on the branch either — so every loop command now tells its pass to commit
incrementally (research note, then implementation, then docs and log) rather than once at the end, and
that the only thing which puts work on `main` is its own commit step.

**A snapshot saves the work; it does not clear the clone.** The tree stays dirty on whatever branch the pass was on, and the next pass used to refuse it. The driver now parks that tree on a branch before each pass (see the facts list below, "A tree a pass leaves behind is parked"), so a clone starts every pass on `main`.

**Refs accumulate and are never auto-pruned.** One ref and one commit per checkpoint. Past ~50 in a
clone, prune by hand — never from the driver, because an automatic delete of the only copy of a killed
pass's tree is the failure this design exists to prevent:

```sh
git -C <clone> for-each-ref --format='%(refname)' refs/wiggum | head -n -50 | xargs -r -n1 git update-ref -d
```

## A pass will not start on a full disk

`WIGGUM_MIN_FREE_GB` (default 150) is checked at the top of every pass. Measured 2026-10-26: the society
tree alone was growing ~1 GB/min and `/home` was ~10 hours from full, which breaks all four loops and
`hs-distill` at once. Below the threshold the driver first prunes only what cargo regenerates —
`target/debug/deps/*.tmp*` older than 20 minutes (43 GB measured, abandoned linker files, none open) and
`target/debug/incremental/` — then refuses the pass and returns non-zero if that was not enough. Never
`deps/` generally, never `target/`: the deletion is limited to files a live process cannot own and to a
directory cargo rebuilds.

## A pass reads the filed issues before it picks what to build

Each loop prompt carries a "Filed issues come before new features" step immediately before the step that ranks candidates, because the four repos had 15 open issues between them on 2026-10-27 (lights 3, society 4, elements 7, emerge 1) and no prompt had mentioned one. A bug issue is the pass's item ahead of any enhancement, an enhancement issue is ranked with the pass's own candidates and its number goes in the rank list, a fix says `Fixes #N` in the commit and the report, and a release issue is recorded and skipped because publishing is the owner's call. big has an authenticated `gh`; big_mac has none, so elements and emerge read `https://api.github.com/repos/Ladvien/<repo>/issues?state=open`, which allows 60 requests an hour per IP and answers 403 or 429 beyond that. The prompt therefore says a 403 or 429 is logged as "could not be read" and never as "no issues", because a pass that records silence it never looked at has hidden the list from every later pass.

The same ship step carries "README, demos and docs stay true", for a matching reason: the 2026-10-27 audit found a false line in each of the four READMEs, and society's and emerge's prompts had never told a pass to touch the README or the clips at all.

## Facts that cost a cycle

- **A loop cannot simply be moved to the idle machine, because the disk is the binding constraint, not
  the cores.** Measured 2026-10-26: `bevy_society`'s `target/` is **239 GB** and `bevy_lights`' is 41 GB.
  `bmb` — the host the picker ranks first on idle cores — has **23 GB** free, and `mac_air` 14 GB. There
  is no room to build either on a Mac, so re-homing them would trade an OOM for a full disk. `big` has
  804 GB free and 22 idle cores, which is why the fix there is a memory bound, not a new host. Run
  `du -sh <clone>/target` before proposing a move; a Bevy workspace's target is larger than most
  machines' free space.
- **Two loops that compile do not fit on `big`.** Measured 2026-10-26: `wiggum@society` died
  `Result: oom-kill` at a 6.9 GB peak with 8.7 GB of swap while `hs-distill` held 9.9 GB resident and
  the host showed 1 GB available. `CARGO_BUILD_JOBS` is now 4 there, and one loop is held until the
  other's pass finishes. A loop that OOMs is worse than a loop that waits: systemd counts the kill as a
  restart, and `StartLimitBurst=3` can retire the unit entirely.
- **`MemorySwapMax` is what oomd kills on, and 2 G was not enough.** Measured 2026-10-27, twice in an
  hour: `wiggum@lights` died `oom-kill` at exactly `6G memory peak, 2G memory swap peak` — both caps hit
  together — and three restarts then retired the unit with `Failed with result 'start-limit-hit'`
  (`StartLimitBurst=3` in `[Unit]`). `oomctl` names the rule: *Swap Used Limit 90 %* against the
  **monitored ancestor** `/user.slice/user-1000.slice`, whose `memory.swap.current` was 9.4 GB of
  31.2 GB (30 %) before a loop spilled into swap and pushed the slice's ratio over. `MemorySwapMax=0` on
  both big units stopped a unit adding to that ratio, and it was enough while the loops were only
  refusing dirty trees. **It was not enough once lights ran real builds:** oomd killed `wiggum@lights`
  three times in 40 minutes (17:24, 17:54, 18:02), each at a 6 GB peak, while society, with four build
  jobs, was killed zero times. Big's swap was 74 % full by then and almost none of it was the loops'
  (vLLM 5.8 GB, `hs-mcp` 4.9 GB, qdrant 2.5 GB; the llama-swap cgroup holds 7.8 GB), so oomd reached its
  90 % line on other services' memory and took the largest eligible cgroup, a lights build. Two changes
  followed, and only one is more than a mitigation: `StartLimitBurst=8` on both units, because three oomd
  kills are not a spin and had retired a healthy loop; and `CARGO_BUILD_JOBS=2` on lights, which is a guess
  at lowering its peak, with kills per hour as the test. The real lever is the machine's swap, which is not
  the loops' to free. `MemoryHigh`/`MemoryMax` stay: the rule is to bound swap, never to remove the memory
  cap. Check a unit's real position with `cat .../memory.swap.current` against `memory.swap.max`, not with
  `MemoryPeak` alone: society sat at 95 % of its swap cap while its peak looked unremarkable.
- **A `target/` for a Bevy workspace is tens of GB, so disk is a real eligibility check.** `mac_air` has
  10 cores and 7.6 GB of disk, which makes it useless for a compile however idle it is;
  `--min-disk-gb 40` on the picker is the honest default. Reclaiming `~/target/sanctuary/release`
  (6.3 GB, 28 h untouched) took it to 14 GB free — deleting a *profile* directory is safe where deleting
  a whole `target/` is not: the dependency cache the next build needs is in the other profile.
- **Check `WIGGUM_MAX` is actually in the installed plist/unit**, not only in the committed one. The
  emerge agent ran `max=0` (forever) because the key was absent, and nothing in a pass log says
  "unbounded" — `driver START … max=0` is the only tell.
- **`launchctl kickstart -k` and `bootout` both kill the pass in flight** (`last exit = -15`). If a pass
  was killed, remove the driver's `stop` file and zero `consecutive-failures` before restarting, then
  `bootout` + `bootstrap` in strict sequence: kickstarting a job that launchd throttled (`state = spawn
  scheduled`, `active count = 0`) does nothing until `ThrottleInterval``ThrottleInterval` (600 s) elapses.
- **macOS has no `timeout(1)`** (and no coreutils: `timeout --foreground 5 echo hi` → 127). A driver that
  wraps a pass in `timeout` cannot run there. `tools/loop/wiggum.sh` since commit `338c5d3` implements the
  ceiling as a pid watchdog, so the same script runs on both hosts — **do not copy an older driver**.
- **One Xvfb per loop, and never `:0`.** `bevy_lights`, `bevy_society` and `bevy_emerge` open real
  windows. Two units whose `ExecStartPre` reaps `Xvfb :99` reap *each other's* server. Each unit owns its
  own display (`:97`, `:98`, …) and matches its own exact command line. Check `pgrep -a Xvfb` first — a
  display already held by something not yours means pick another number, not kill it.
- **`StartLimit*` are `[Unit]` keys.** systemd ignores them in `[Service]` with only a warning.
- **`StandardOutput=journal`, not `append:`.** An `append:` path fails the whole start
  (`status=209/STDOUT`) when its directory is missing, and `LogsDirectory=` does not create it early
  enough.
- **`WIGGUM_MAX`, not a small cap.** The driver exits **0** at the cap, so `Restart=on-failure` /
  `KeepAlive.Crashed` will not restart it — a small number silently ends the loop.
- **`TimeoutStopSec`/`ExitTimeOut` > `WIGGUM_PASS_TIMEOUT`**, or a stop cuts a pass mid-edit.
- **`CARGO_TARGET_DIR` outside the repo, one per loop.** Workspace crates at one version share an artifact
  hash across checkouts, so a shared dir makes cargo call one tree's units `Fresh` and silently run the
  other tree's code (measured 2026-10-21). A clone's own `target/` is fine; `~/.cargo-target-shared/global`
  never is.
- **launchd does not expand `~` or `$HOME`.** The committed plists use a `/Users/USER` placeholder that the
  install step substitutes; installing an unsubstituted plist yields a job that looks `running` and does
  nothing.
- **Every way of stopping a loop ends the pass in flight, and nothing in this repo drains one.** That covers `launchctl kickstart -k`, `bootout`, `systemctl --user stop` and the driver's own stop file, which sends the pass `TERM` and then `KILL` five seconds later (`df7324e`, 2026-10-26). The stop file is the tidy way (the driver exits cleanly and snapshots the tree), not the gentle one. Measured 2026-10-27, on the word of stale comments in the driver, both launchd plists, the unit template and `README-wiggum.md` that all said a stop "finishes the pass in flight": society's pass 20 (78 minutes in, 7 uncommitted files) and elements' pass 22 (48 minutes in, 11 files) ended `rc=143` nine seconds after a stop file was written to restart both onto a new driver. Nothing was lost, because the final snapshot saved each tree and the new park step committed it to a branch (`WIP(pass N, parked)`) and pushed it, but the passes' momentum was, and the next pass starts from `main`. Those comments now say what the code does. To restart onto a new driver or prompt without cutting a pass, stop between passes, when `driver.log` shows `pass N END` or `idle (…): next pass in …s`; `systemctl --user restart` is no better, since KillMode=control-group sends SIGTERM to the pass too. After a stop, reset `consecutive-failures` before restarting; the driver clears a leftover `stop` file itself when it starts.
- **A `TimeoutStopSec` that is longer than a pass turns an unrelated cgroup leak into a 7-hour
  `deactivating`.** Measured 2026-10-26: stopping `wiggum@lights` hung in `deactivating` for its full
  `TimeoutStopSec=25200` because a `home-still_qdrant_1` container (`rootlessport`, `conmon`,
  `entrypoint.sh`) had been parented **into `wiggum@lights.service`'s cgroup** at container start — a
  live CI runner job owned it, so killing it was not an option either. `ActiveState=deactivating`,
  `SubState=final-sigterm` and `cat <cg>/cgroup.procs` showing PIDs you never started is the signature.
  Writing those PIDs to another cgroup's `cgroup.procs` fails **`EBUSY`** while the target is a cgroup
  systemd manages for a service (no internal processes allowed), so the way out is a **transient scope**,
  which is a leaf that accepts them:

  ```sh
  systemd-run --user --scope --unit=wiggum-rescue-$(date +%s) sleep 7200 &
  # then, for each pid in the stuck cgroup:
  echo <pid> > /sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice/<scope>.scope/cgroup.procs
  ```

  The unit then finishes its stop and can be started. Root cause is elsewhere (a `podman run` from a
  service inherits that service's cgroup, so the container should be given its own `--cgroup-parent` or an
  `ExecStart` under a transient unit); treat the rescue as per-incident, not a fix.
- **The driver refuses a tree with >40 modified files**, so it will not run a pass in an rsync scratch tree.
- **An uncommitted edit to a loop's own prompt is parked on a branch, and the pass runs the old prompt.** Measured 2026-10-27: a rule added to `bevy_lights`' skill file in the live clone and left uncommitted was found by 50 consecutive passes (iterations 61 to 110), each of which saw a tracked file it had not changed, stopped as its own rule says, built nothing, and logged the stop as a commit. The same happened once on 2026-10-26. The driver now parks a leftover tree before each pass (next facts), so the loop no longer stalls, but the failure has changed shape and not gone: the edit is committed to a `wiggum/parked-pass-N-…` branch, the clone returns to `main`, and the pass runs the **committed** prompt, so the edit has no effect until it is on `main`. After editing a command or skill in a clone, stop the loop first (`wiggum.sh stop`), then commit it at once by explicit path (`git commit -- <path>`) and push it, unless the clone already holds the loop's own unpushed commit: a plain `git push origin main` would publish that too, and publishing a pass's work is the pass's decision. A pass's own `git add -A` will otherwise sweep the edit into its commit (society's `688af75` carried one), which hides what changed.
- **A loop whose model has no credit looks healthy.** Measured 2026-10-27: ollama-cloud answered `402 Insufficient credits` 109 times on big while both units were `active`, `consecutive-failures` was 0 and no unit had failed; every pass log held the 11 bytes `Working...` and nothing else. The tell is `grep -c 402 ~/.omp/logs/omp.<date>.<pid>.log` against a pass log that never grows. Switching `WIGGUM_MODEL` is not enough: omp's auxiliary roles in `~/.omp/agent/config.yml` (`modelRoles`, notably `smol`) each name their own provider, and `smol` was still on a second dry one, so skill-description compression and job-widget calls kept returning 402 after the main model answered. Check both, then confirm with a pass whose log grows.
- **A GIF whose frames all carry a 0-centisecond delay is a still.** Measured 2026-10-27: two of `bevy_elements`' README GIFs (35 frames each) had every Graphic Control Extension delay at 0, so a viewer that honours the delay shows one frame under a caption that promises motion. Count the delays by scanning for the `21 f9 04` block header and reading the two bytes after its packed-flags byte; rewriting those two bytes (8 cs, 12.5 fps, was used) leaves the file size and every pixel unchanged, so it needs no GPU recapture.
- **A pass can commit 131 MB in one commit.** Measured 2026-10-27 in society's `688af75`: ten `docs/loop/runs/2026-10-27-03/evidence/…/series.csv` files of 67,200 lines each (about 14 MB apiece), 69 files and 682,564 insertions, on a repository whose `.git` was already 253 MB. Earlier runs commit files of the same name (`36c4d8c`, `9938002`), so it is the loop's practice and not an accident. It was pushed on 2026-10-27 at the owner's instruction (185 MB of pack, 11 s); each such commit is permanent once pushed, and whether evidence of that size should keep going into history is still open.
- **A pass waiting on a detached gate looks exactly like a hung one, and only its session file tells them apart.** Measured 2026-10-27 on society's pass 19, 4 h 45 m old: 0.5% CPU, no compiler among its children, no repository change for 83 minutes and the omp debug log silent for 27 minutes, which are every usual sign of a hang. It was inside a `wait` on a `setsid nohup` gate whose `cargo test` had passed (321 tests, 0 failed) and whose second step wrote its output file that same minute. The signal is `~/.omp/agent/sessions/-<repo>/<start>_<id>.jsonl`: its mtime is the last time the pass did anything, and its final entries name the tool it is inside, so read `tail -n 3` of it before calling a pass dead.
- **A detached gate is not reached by the driver's kill, and the cgroup is what ends it.** `setsid nohup` takes a process out of the pass's process group but not out of the unit's cgroup, and the driver signals only the pass's own pid (`kill -TERM "$pass_pid"`), so by reading the code a ceiling kill leaves the gate running with the cargo target-dir lock and several GB against the next pass inside a 7 GB, no-swap cgroup (inferred from the code, not observed). What was observed: stopping the driver through its stop file made the unit inactive and systemd tore the whole cgroup down, with 0 orphan processes afterwards, and the restarted unit's pass 20 was editing code within two minutes.
- **Society's gate does not fit a 6-hour pass that spends its first four hours on research.** Measured 2026-10-27: after the commit at 15:01 the pass launched `cargo test` (47 minutes, 321 tests across 80 suites) and then `cargo test --no-default-features` (50 minutes in, 70 of 80 suites done), with clippy and rustdoc still queued. That is over two hours, started an hour after the commit, and it was still on the second of four steps when the pass was stopped three minutes before its ceiling. Either the gate or the ceiling has to give; it is recorded here, not decided.
- **A loop with nothing it may do costs as much as one that is working, so the driver backs off.** Measured 2026-10-27: lights ran 50 passes of about 20 s that each refused a stranded tree, and emerge ran 25 passes of about 26 s whose only output was a log line, 19 of them pushed to `main` as "a pass that shipped nothing" commits, each pass a full model call every 90 seconds. The driver now treats `rc=0` in under `WIGGUM_IDLE_UNDER` (default 120 s) as idle and doubles the sleep for each consecutive idle pass up to `WIGGUM_IDLE_MAX` (default 3600 s), so 60 s becomes 2, 4, 8, 16 and 32 minutes and then an hour, which still notices a new issue within the hour; a longer or failed pass resets it and `WIGGUM_IDLE_MAX=0` turns it off. Proven against a fake `omp` on a throwaway repo: it doubles and caps, a 12 s pass resets the streak, failures are not counted as idle and keep the five-failure stop, and a stop lands in under a second in the middle of a 60 s backoff. A unit picks it up only when its driver restarts.
- **A baseline worktree holding `main` blocks the way back to it.** Measured 2026-10-27: lights' clone was stranded on `loop/iter-115` with an ungated tree, and `git checkout main` failed with `'main' is already used by worktree at '/home/ladvien/base_l115'`, a comparison checkout a pass had made. The skill says a pass starts on `main` with no tracked changes, so a clone left elsewhere stops every later pass. The driver now detaches a *clean* holder itself when it returns a clone to `main`, and refuses the pass, with the holder's path in the log, when the holder has changes in it. The lights skill now tells a pass to make its baseline with `git worktree add --detach` and to remove it when done. Removing the seven old ones freed about 2 GB: the other 37 GB under `~/l13`, `~/l15` and `~/l20` is in sibling `*-target` build directories, which `git worktree remove` does not touch and which are not worktrees.
- **A tree a pass leaves behind is parked on a branch, and the clone returns to `main`, before the next pass.** Measured 2026-10-27: lights logged 61 passes that each refused a tree they had not made and stopped in about 20 s (50 for an uncommitted prompt edit, 11 for a stranded `loop/iter-115` holding 412 lines of ungated shader work), and both were cleared by hand the same way. `park_stranded_tree` in `wiggum.sh` does it at the start of every pass: dirty on `main` cuts a `wiggum/parked-pass-N-<time>` branch from HEAD and commits there; dirty on another branch commits on that branch; a detached HEAD whose commit no branch holds gets a branch cut for it; then the branch is pushed and the clone checks out `main`. N is the pass that left the work. It **never writes to or pushes `main`**, because committing a leftover onto a branch somebody builds from put ungated work on `origin/main` in all four repos on 2026-10-26. It refuses the pass, and says why in the log, in the middle of a merge, rebase, revert or cherry-pick, when `main` is held by a worktree with changes, and (as before) above 40 changed files; each counts as a failure, so five in a row stop the loop. The commit is titled `WIP(pass N, parked)`, which is what every loop command tells a pass to search for (`git log --all --oneline --grep='WIP(pass'`), so a pass looking for a killed pass's work finds it as it finds a snapshot; the prompts do not tell a pass to adopt it, and the pass starts clean on `main`. It is also in `driver.log` (`pass N pre: parked 3 file(s) on wiggum/parked-pass-N-…`), on origin, and in `git branch --list 'wiggum/parked-*'`. `WIGGUM_PARK=0` turns it off; `WIGGUM_BRANCH` names the branch to return to when origin has no `HEAD`. It leans on the rule that nothing but the driver writes to a loop's clone, so a person who edits one between passes has the edit parked, not lost. Pushing a parked branch triggers no CI in any of the four repos, whose workflows run on `main` and pull requests only. Tested with 57 checks across 18 cases, on macOS (bash 3.2) and on Linux (bash 5); the tests found a data-loss bug before it shipped: `git branch --contains HEAD` lists the detached HEAD itself, so a commit left on a detached HEAD looked held, the clone moved to `main`, and the commit lost its only name. `git for-each-ref --contains` names real refs only.
- **A parked branch or a snapshot that adds more than `WIGGUM_PUSH_MAX_MB` (default 10) of new data is committed and kept local.** Measured 2026-10-27: the park step pushed an 82 MB branch to society's **public** repository, six raw `series.csv` of about 14 MB each swept up by `git add -A` before the new `.gitignore` reached that clone (the cap was 100 MB then, and the six totalled 84), and the older snapshot push had already put the same blobs into 51 hidden `refs/wiggum/*` refs (passes 14 to 20); a 129 MB branch of nine more CSVs, `loop/wip-2026-10-26`, had been pushed at the owner's instruction without anyone opening it. The owner had ruled raw run output out of git an hour earlier. Both push paths now share one guard, measured on what the commit adds beyond its first parent (not on file sizes, so content already in history is free); a legitimate leftover is small, and elements' 11 files were 0.45 MB. The owner approved deleting the two branches and the 51 refs from origin: `refs/wiggum` there went from 75 to 24 and no blob over 5 MB is reachable from any side-channel ref, but GitHub keeps unreachable objects until it garbage-collects, which cannot be forced. Everything stayed on big. The other three loop repos hold 3 to 6 MB of side-channel data each. Before pushing anything a loop made, open it: `git diff --numstat <base> <branch>` with sizes takes a second.

## Templates

### systemd (`~/.config/systemd/user/wiggum@<name>.service`)

```ini
[Unit]
Description=<repo> Wiggum loop (one fresh <command> pass at a time)
StartLimitIntervalSec=86400
StartLimitBurst=8
Documentation=file:%h/<clone>/.omp/commands/<command>.md
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=%h/<clone>
ExecStartPre=-/usr/bin/pkill -x -f "Xvfb :97 -screen 0 1920x1080x24 -nolisten tcp -ac"
ExecStartPre=/bin/sh -c 'sleep 1; /usr/bin/Xvfb :97 -screen 0 1920x1080x24 -nolisten tcp -ac & sleep 2; test -S /tmp/.X11-unix/X97'
ExecStopPost=-/usr/bin/pkill -x -f "Xvfb :97 -screen 0 1920x1080x24 -nolisten tcp -ac"
ExecStart=%h/<clone>/tools/loop/wiggum.sh __driver
Environment=PATH=%h/.local/bin:%h/.cargo/bin:/usr/local/bin:/usr/bin:/bin
Environment=WIGGUM_REPO=%h/<clone>
Environment=WIGGUM_COMMAND=<command>
Environment=WIGGUM_MODEL=ollama-cloud/deepseek-v4.1-flash:1017
Environment=WIGGUM_THINK=medium
Environment=WIGGUM_SLEEP=60
Environment=WIGGUM_MAX=400
Environment=WIGGUM_PASS_TIMEOUT=21600
Environment=CARGO_BUILD_JOBS=6
Environment=DISPLAY=:97
Nice=19
IOSchedulingClass=idle
# **Bounded so a pass is throttled, not shot, and so oomd cannot pick it on swap.** oomd kills on a
# 90 % swap-used limit against the monitored ancestor `/user.slice/<uid>.slice`; a unit that can swap
# feeds that ratio, and at 2G both big loops were OOM-killed at their cap. `0` makes the unit
# contribute no swap at all. `MemoryHigh` reclaims under pressure and `MemoryMax` bounds the cgroup —
# keep both; the rule is to bound swap, never to remove the memory cap. `ManagedOOMPreference` belongs
# on the *slice* (`app-wiggum.slice`), not here: a unit-level setting is ignored because oomd monitors
# the ancestor.
MemoryHigh=6G
MemoryMax=7G
MemorySwapMax=0
KillSignal=SIGTERM
TimeoutStopSec=25200
Restart=on-failure
RestartSec=120
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

```sh
systemctl --user daemon-reload
systemd-analyze --user verify ~/.config/systemd/user/wiggum@<name>.service
systemctl --user enable --now wiggum@<name>.service
```

`loginctl enable-linger` must be on, or a user unit dies with the last session.

### launchd (`~/Library/LaunchAgents/com.ladvien.wiggum-<name>.plist`)

The committed pair in `tools/loop/` is the reference: `com.ladvien.wiggum-emerge.plist` (two-hour pass,
`CARGO_TARGET_DIR`) and `com.ladvien.wiggum-elements.plist` (six-hour pass). Substitute `/Users/USER`
before installing, and keep `KeepAlive` as `Crashed` only so `stop` is observable:

```sh
sed -i '' "s|/Users/USER|$HOME|g" com.ladvien.wiggum-<name>.plist
plutil -lint com.ladvien.wiggum-<name>.plist
cp com.ladvien.wiggum-<name>.plist ~/Library/LaunchAgents/
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.ladvien.wiggum-<name>.plist
launchctl kickstart -k gui/$(id -u)/com.ladvien.wiggum-<name>
```

`RunAtLoad` is `false` deliberately: installing an agent must not start a pass while a human is setting
things up.

## What the four loops are, today

| loop | host | clone | command | model | display |
|---|---|---|---|---|---|
| lights | big | `~/bevy_lights_loop` | `lights-loop` | `claude-sonnet-5-5` | `:97` |
| society | big | `~/bevy_society_loop` | `build-loop` | `claude-sonnet-5-5` | `:98` |
| elements | big_mac | `~/bevy_elements` | `elements-loop` | `ollama-cloud/deepseek-v4.1-flash:1017` | native Metal |
| emerge | big_mac | `~/bevy_emerge_loop` | `build-loop` | `ollama-cloud/deepseek-v4.1-flash:1017` | native Metal |

Lights and society moved off ollama-cloud on 2026-10-27 after it answered 402 on big (see the fact above); elements and emerge had no 402s and were left on it.

## Verify a loop is real, not just active

1. supervisor: `systemctl --user is-active wiggum@<name>` → `active`; `launchctl print gui/$(id -u)/…` →
   `state = running` with a pid.
2. the pass is the right one: `grep "pass 1 START" ~/.local/state/wiggum/<repo>/driver.log` →
   `command=/<command> ... dirty=0`.
3. nothing is driving a human's tree: `git -C <human checkout> status --short` unchanged.
4. **a pass reached its own gate once.** This is the only proof the loop can do anything; watch one pass
   to completion and read its own verdict, not just the exit code.
