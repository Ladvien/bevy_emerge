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
- **`launchctl kickstart -k` kills the pass in flight.** That is `bootout`'s behaviour too: stop the driver
  through its own stop file first (`wiggum.sh stop`), and if a pass was killed, delete the leftover
  `stop` file and reset `consecutive-failures` before restarting, or the new driver refuses to start.
- **The driver refuses a tree with >40 modified files**, so it will not commit into an rsync scratch tree.

## Templates

### systemd (`~/.config/systemd/user/wiggum@<name>.service`)

```ini
[Unit]
Description=<repo> Wiggum loop (one fresh <command> pass at a time)
StartLimitIntervalSec=86400
StartLimitBurst=3
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

| loop | host | clone | command | display |
|---|---|---|---|---|
| lights | big | `~/bevy_lights_loop` | `lights-loop` | `:97` |
| society | big | `~/bevy_society_loop` | `build-loop` | `:98` |
| elements | big_mac | `~/bevy_elements` | `elements-loop` | native Metal |
| emerge | big_mac | `~/bevy_emerge_loop` | `build-loop` | native Metal |

## Verify a loop is real, not just active

1. supervisor: `systemctl --user is-active wiggum@<name>` → `active`; `launchctl print gui/$(id -u)/…` →
   `state = running` with a pid.
2. the pass is the right one: `grep "pass 1 START" ~/.local/state/wiggum/<repo>/driver.log` →
   `command=/<command> ... dirty=0`.
3. nothing is driving a human's tree: `git -C <human checkout> status --short` unchanged.
4. **a pass reached its own gate once.** This is the only proof the loop can do anything; watch one pass
   to completion and read its own verdict, not just the exit code.
