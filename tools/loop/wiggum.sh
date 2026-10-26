#!/bin/bash
# **The Wiggum loop: run `build-loop` forever, one fresh context per pass.**
#
# Dumb on purpose. It starts an `omp` process with the command, waits for it to exit, sleeps a
# little, and starts another. It holds no state, makes no decisions, and never inspects the result —
# everything a pass learns and everything it must not forget lives in the repository, so a pass that
# dies mid-thought is repaired by the next one rather than by this script.
#
# **Why not a smart driver?** Because the smart driver is a second agent with its own context, its
# own bugs, and a second opinion about when the work is done. The research is unambiguous that the
# evaluator must not be part of the loop; a driver that decides "that pass looked good, skip ahead"
# is exactly that. This one only counts.
#
# Usage, on big:
#   ~/wiggum.sh start          # start in the background, detached from the shell
#   ~/wiggum.sh status         # is it running, how many passes, what is the last log line
#   ~/wiggum.sh stop           # stop after the current pass ends (never mid-pass)
#   ~/wiggum.sh tail           # follow the driver's own log
#   ~/wiggum.sh one            # run exactly one pass in the foreground, for a smoke test
#   ~/wiggum.sh guard          # exit 1 if a pass is running — call before reset/checkout/clean
#
# Environment, all optional:
#   WIGGUM_MODEL   default ollama-cloud/deepseek-v4.1-flash:1017
#   WIGGUM_REPO    default $HOME/bevy_emerge — any git clone
#   WIGGUM_COMMAND slash command a pass runs, default build-loop
#   WIGGUM_STATE   where the driver keeps its logs, default ~/.local/state/wiggum/<repo>
#   WIGGUM_SLEEP   seconds between passes, default 60
#   WIGGUM_MAX     stop after this many passes, default 0 (forever)
#   WIGGUM_THINK   thinking level, default medium
#   WIGGUM_PASS_TIMEOUT  hard ceiling on one pass in seconds, default 4500 (75 min)

set -uo pipefail

MODEL="${WIGGUM_MODEL:-ollama-cloud/deepseek-v4.1-flash:1017}"
REPO="${WIGGUM_REPO:-$HOME/bevy_emerge}"
# **The command this repo's passes run.** A repo that has its own loop command keeps it: this driver is
# about *how* a loop runs (one fresh pass, stop between passes, never mid-edit), not about what a pass
# does. `bevy_elements` has `elements-loop`, `bevy_debugger_mcp` has `bevy-debugger-loop`; anything else
# gets the generic `build-loop`.
COMMAND="${WIGGUM_COMMAND:-build-loop}"
SLEEP_BETWEEN="${WIGGUM_SLEEP:-60}"
MAX_PASSES="${WIGGUM_MAX:-0}"
THINKING="${WIGGUM_THINK:-medium}"
# **A hard ceiling on one pass, in seconds, default 75 minutes.** The command tells a pass its budget is
# 45 minutes and that running long is worse than shipping nothing — but a command is advice, and a model
# that has lost track of time will keep going. Measured: the first real pass ran past an hour with work
# still in flight. This is the enforcement: past the ceiling the pass process is terminated, which is a
# non-zero return, so the driver counts it as a failure (five in a row stops the loop) and the next pass
# finds the tree exactly as the killed pass left it. **Kill only the pass, never with `-f` on a pattern**
# — the shell that launched it would match too.
PASS_TIMEOUT="${WIGGUM_PASS_TIMEOUT:-4500}"

# **State lives outside the repository.** A loop's own bookkeeping must never appear in a pass's
# `git status` — a dirty tree is the first thing a pass is told to fix, and a driver writing logs into the
# tree would make every pass start by diagnosing itself. `target/` is gitignored, but a repo without that
# rule, or a pass that runs `git clean`, would lose the logs. `XDG_STATE_HOME` is the right home.
STATE="${WIGGUM_STATE:-${XDG_STATE_HOME:-$HOME/.local/state}/wiggum/$(basename "$REPO")}"
PIDFILE="$STATE/driver.pid"
COUNTER="$STATE/pass-count"
DRIVER_LOG="$STATE/driver.log"
PASS_LOG_DIR="$STATE/passes"
STOPFILE="$STATE/stop"
MARKER="wiggum-$USER-$(basename "$REPO")"

# **The state directory is created only once the repo is known to be a clone.** A driver pointed at a
# scratch tree used to `mkdir` inside it before checking, which is the one write this script must never
# make into somebody else's directory.
ensure_state() { mkdir -p "$STATE" "$PASS_LOG_DIR"; }

log() { printf '%s %s\n' "$(date '+%Y-%m-%dT%H:%M:%S')" "$*" >>"$DRIVER_LOG"; }

running() {
  [ -f "$PIDFILE" ] || return 1
  local pid; pid=$(cat "$PIDFILE" 2>/dev/null) || return 1
  [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null
}

# **A second driver on one checkout, found in the process table.** The pidfile alone cannot answer this
# for the case the unit creates: `ExecStart=… __driver` is the driver, so a stale pidfile or a driver
# started by hand and a unit-owned one both claim the same tree, and two `omp` passes interleaving in one
# checkout is the expensive failure this script exists to prevent.
#
# **Measured 2026-10-26: the marker was in the environment, and `pgrep -f` reads argv.** `env
# WIGGUM_MARKER=… bash "$0" __driver` never puts the marker in the child's command line, so the guard
# matched nothing and two drivers ran together — passes numbered 1, 2, 3 out of order in one
# `driver.log`. The marker is now an argv token on the re-exec, which is where `pgrep -f` looks.
#
# **This driver matches its own argv, and so does every subshell it forks** — bash children inherit the
# command line, and a `$( … )` running this function can still be alive when it asks. So the guard
# counts only a marker process that does **not** descend from this driver.
other_driver_running() {
  local pid p
  for pid in $(pgrep -f -- "$MARKER" 2>/dev/null); do
    # **Walk up from the candidate.** Any marker process that descends from this driver — a `$( … )`
    # subshell, a `pipeline` stage — is this driver, not another one. Bounded by the tree's depth.
    p=$pid
    while [ -n "$p" ] && [ "$p" != 0 ] && [ "$p" != 1 ]; do
      [ "$p" = "$$" ] && { p=""; break; }
      p=$(ps -o ppid= -p "$p" 2>/dev/null | tr -d ' ')
    done
    # `p` is empty when the walk reached this driver, so the candidate is ours and is skipped.
    [ -z "$p" ] && continue
    return 0
  done
  return 1
}

# **One pass.** The whole of what this script does to the world.
one_pass() {
  # **Both preconditions come before the first write, and that order is the point.** `one` runs this
  # in the foreground, so a pass that wrote its own counter into somebody's scratch tree before
  # checking would be the script doing the thing it exists to refuse. Measured on big: the first
  # version wrote `pass-count` and `driver.log` into the repo before deciding it was not a clone.
  if [ ! -d "$REPO/.git" ]; then
    echo "ABORTED: $REPO is not a git clone" >&2
    return 1
  fi
  # **Warn before a pass whose command may publish.** `bevy_debugger_mcp`'s `bevy-debugger-loop` says
  # "every iteration ends pushed and **published**", and a publish cannot be undone: crates.io has no
  # unpublish and a yanked version is burned. A driver cannot read a command's intent, so it states what
  # it can see and lets the operator decide. A warning, not a block — only a human knows whether the
  # command in *their* repo publishes.
  local cmd_file="$REPO/.omp/commands/$COMMAND.md"
  if [ -f "$cmd_file" ] && grep -qiE 'cargo publish|ends pushed and published' "$cmd_file"; then
    echo "WARNING: /$COMMAND may publish to crates.io — irreversible, and a loop cannot ask." >&2
  fi

  local dirty; dirty=$(git -C "$REPO" status --short 2>/dev/null | wc -l | tr -d ' ')
  if [ "${dirty:-0}" -gt 40 ]; then
    echo "ABORTED: $REPO has $dirty modified files — a scratch tree, not a clone. Fix it by hand." >&2
    return 1
  fi

  # **A clean clone fast-forwards to origin before the pass starts, and a dirty one is left alone.**
  # Measured 2026-10-26: a human session pushed iterations 11 and 12 to `bevy_lights` while the loop's
  # clone sat 7 commits behind, so the pass would have redesigned a pool the human had already rebuilt
  # — the loop's whole product is a diff, and a diff against stale code is the wrong diff. `--ff-only`
  # and only on a clean tree: a fast-forward is the one history change that cannot lose work, and a
  # leftover from a killed pass is never resolved automatically (that pass may be mid-thought).
  # A clone that has diverged from origin (local commits) is reported, not merged: choosing a merge
  # strategy is the operator's call, and a rebase is the one thing this driver must never do to a tree
  # a loop is driving.
  local behind=0 ff_from=0 ff_to=""
  if [ "${dirty:-0}" -eq 0 ] && [ "${WIGGUM_NO_FETCH:-0}" != "1" ]; then
    if git -C "$REPO" fetch --quiet origin 2>/dev/null; then
      local upstream; upstream=$(git -C "$REPO" rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null)
      if [ -n "$upstream" ]; then
        local ahead
        ahead=$(git -C "$REPO" rev-list --count "$upstream..HEAD" 2>/dev/null || echo 0)
        behind=$(git -C "$REPO" rev-list --count "HEAD..$upstream" 2>/dev/null || echo 0)
        if [ "${behind:-0}" -gt 0 ] && [ "${ahead:-0}" -eq 0 ]; then
          if git -C "$REPO" merge --ff-only "$upstream" >/dev/null 2>&1; then
            ff_from=$behind
            ff_to=$(git -C "$REPO" rev-parse --short HEAD)
            behind=0
          else
            log "pre-pass: WARNING could not fast-forward $behind commit(s) — pass works on stale code"
          fi
        elif [ "${behind:-0}" -gt 0 ]; then
          log "pre-pass: WARNING $behind behind and $ahead ahead of $upstream — diverged, left as is"
        fi
      fi
    else
      log "pre-pass: WARNING fetch failed — pass works on whatever the clone has"
    fi
  fi

  ensure_state
  local n; n=$(( $(cat "$COUNTER" 2>/dev/null || echo 0) + 1 ))
  echo "$n" >"$COUNTER"
  # **The fast-forward is recorded against the pass it precedes**, so `pass N START … behind=7` and
  # the reason the clone is current are adjacent lines in one log rather than in the previous pass's
  # story. `pre-pass:` lines above this point are deliberately pass-numbered too.
  if [ "${ff_from:-0}" -gt 0 ]; then
    log "pass $n pre: fast-forwarded $ff_from commit(s) to $ff_to"
  fi
  local passlog="$PASS_LOG_DIR/pass-$(printf '%04d' "$n")-$(date '+%Y%m%d-%H%M%S').log"
  log "pass $n START command=/$COMMAND model=$MODEL dirty=$dirty behind=$behind"
  # **A fresh process per pass is the whole design.** `omp -p` is one-shot: it reads the prompt,
  # does the work, exits. No `--continue`, no `--resume`, no session carried over — the repository is
  # the memory. `--model` is explicit so a pass never inherits an interactive choice.
  # **The timeout is done in this process, not with `timeout(1)`, because macOS does not have it.**
  # Measured on big_mac: every pass died `rc=127` with `timeout: command not found`, which looked like a
  # missing toolchain and was actually the wrapper. A background watchdog kills the pass by pid at the
  # ceiling; `wait` then reaps it. The kill is by exact pid, so it cannot touch the driver or the shell.
  local pass_pid watchdog rc timed_out=0
  ( cd "$REPO" && exec omp --model "$MODEL" --thinking "$THINKING" -p "/$COMMAND" ) \
    >"$passlog" 2>&1 &
  pass_pid=$!
  # **The watchdog marks the ceiling in a file, not by guessing from the exit code.** A pass killed by
  # SIGTERM and a pass that died for its own reasons both exit 143, and a loop that reported the second
  # as a timeout would hide the real failure. The flag is the only honest source.
  rm -f "$STATE/pass-timed-out"
  ( sleep "$PASS_TIMEOUT"; : >"$STATE/pass-timed-out"; kill -TERM "$pass_pid" 2>/dev/null; \
    sleep 10; kill -KILL "$pass_pid" 2>/dev/null ) &
  watchdog=$!
  wait "$pass_pid"
  rc=$?
  kill -TERM "$watchdog" 2>/dev/null
  wait "$watchdog" 2>/dev/null
  if [ -f "$STATE/pass-timed-out" ]; then
    timed_out=1
    rm -f "$STATE/pass-timed-out"
    log "pass $n TIMED OUT after ${PASS_TIMEOUT}s — killed, tree left as found"
  fi
  if [ "$rc" -eq 124 ]; then
    log "pass $n TIMED OUT after ${PASS_TIMEOUT}s — killed, tree left as found"
  fi
  log "pass $n END rc=$rc timed_out=$timed_out log=$passlog"

  # **A tree left dirty is the next pass's first bug**, and the command says so. This only reports it.
  #
  # **But reporting is not enough, and that is measured.** A pass is killed at the ceiling or by the
  # OOM killer *after* it has done the work and *before* it reaches its own commit step (commit is
  # step 4-5 of 6, behind implement/gate/benchmark), so the only record of what it learned stays in
  # the working tree — where the next pass is told to "finish it or revert it" and has no way to know
  # which. Measured 2026-10-26: society accumulated 15 uncommitted files and lights 8 across killed
  # passes, none of it on any branch.
  #
  # So the driver saves it instead: a **rescue branch** (`wiggum/rescue-<pass>`), committed and pushed.
  # This is deliberately NOT the loop's own commit: it is unreviewed, ungated work on its own branch,
  # so the command's own `loop/iter-N` and merge discipline is untouched and the next pass can see
  # exactly what a killed pass had. Never on the branch the loop is working on, never a force-push,
  # and never when the tree is the loop's own scratch (a `target/` is excluded by `.gitignore` already).
  local after; after=$(git -C "$REPO" status --short 2>/dev/null | wc -l | tr -d ' ')
  if [ "${after:-0}" -gt 0 ]; then
    log "pass $n WARNING: $after uncommitted file(s) left behind"
    # **The branch is captured by name, not restored with `checkout -`**, which depends on a
    # previous-branch stack that a killed pass can leave pointing elsewhere.
    local home_branch rescue
    home_branch=$(git -C "$REPO" rev-parse --abbrev-ref HEAD 2>/dev/null)
    rescue="wiggum/rescue-$n-$(date '+%Y%m%d-%H%M%S')"
    if [ -n "$home_branch" ] && [ "$home_branch" != "HEAD" ] \
       && git -C "$REPO" checkout -q -b "$rescue" 2>/dev/null; then
      git -C "$REPO" add -A 2>/dev/null
      if git -C "$REPO" -c user.email="wiggum@loop.local" -c user.name="wiggum loop" \
           commit -q -m "WIP(pass $n): work left by a pass that did not reach its commit step

Saved by the driver, not by the pass. Unreviewed, ungated, unpushed to main on purpose: the
loop's own commit and merge discipline is in its command, and this branch exists only so the
next pass can see what this one had done rather than rediscovering it from a dirty tree." 2>/dev/null; then
        if git -C "$REPO" push -q origin "$rescue" 2>/dev/null; then
          log "pass $n RESCUED $after file(s) to $rescue (pushed to origin)"
        else
          log "pass $n RESCUED $after file(s) to $rescue (local only — push failed)"
        fi
      else
        log "pass $n rescue: commit failed, $after file(s) still in the tree"
      fi
      # **Back to the loop's branch either way**, so the next pass never starts on a rescue branch.
      # A failure here is loud: the loop would otherwise run its next pass on unreviewed work.
      git -C "$REPO" checkout -q "$home_branch" 2>/dev/null \
        || log "pass $n rescue: COULD NOT RETURN to $home_branch — next pass starts on $rescue"
    else
      log "pass $n rescue: could not branch from ${home_branch:-detached HEAD}, $after file(s) still in the tree"
    fi
  fi
  return $rc
}

driver() {
  # **The clone check comes before any write.** A driver pointed at somebody's rsync scratch must not
  # create a directory in it, and the whole run is pointless there anyway.
  if [ ! -d "$REPO/.git" ]; then
    echo "refusing to run: $REPO is not a git clone" >&2
    exit 2
  fi
  ensure_state
  # **One driver per machine.** Two drivers racing on one checkout produce interleaved edits and
  # neither can trust the other's tree. The marker is in the process table, not only the pidfile,
  # because a pidfile survives a kill -9.
  if other_driver_running; then
    log "refusing to start: another driver is already running"
    echo "another wiggum driver is already running ($MARKER)"
    exit 1
  fi
  echo $$ >"$PIDFILE"
  rm -f "$STOPFILE"
  # **A stop request finishes the pass in flight and then exits — it never kills one.** systemd's
  # `systemctl --user stop` sends SIGTERM to the whole cgroup, which would otherwise kill `omp` in the
  # middle of an edit and leave the tree the next pass has to repair. Trapping it turns the signal into
  # the same thing `wiggum.sh stop` does: a stop file the loop checks between passes.
  # `TimeoutStopSec` in the unit must be longer than one pass (45 min) for this to be honest.
  trap 'touch "$STOPFILE"; log "driver: SIGTERM received, finishing the pass in flight"' TERM INT
  log "driver START pid=$$ model=$MODEL repo=$REPO sleep=${SLEEP_BETWEEN}s max=${MAX_PASSES:-forever}"

  while :; do
    [ -f "$STOPFILE" ] && { log "driver STOP: stop file"; break; }
    if [ "$MAX_PASSES" -gt 0 ] && [ "$(cat "$COUNTER" 2>/dev/null || echo 0)" -ge "$MAX_PASSES" ]; then
      log "driver STOP: reached WIGGUM_MAX=$MAX_PASSES"
      break
    fi

    one_pass
    rc=$?

    # **Five consecutive hard failures stop the loop.** A pass that exits non-zero once is ordinary —
    # a rate limit, a killed process. Five in a row means the machine or the checkout is broken, and
    # a loop that keeps hammering it burns the API budget producing nothing.
    if [ "$rc" -ne 0 ]; then
      FAILS=$(( $(cat "$STATE/consecutive-failures" 2>/dev/null || echo 0) + 1 ))
      echo "$FAILS" >"$STATE/consecutive-failures"
      log "pass failed rc=$rc consecutive=$FAILS"
      if [ "$FAILS" -ge 5 ]; then
        log "driver STOP: 5 consecutive failures — see $DRIVER_LOG and $PASS_LOG_DIR"
        break
      fi
    else
      echo 0 >"$STATE/consecutive-failures"
    fi

    sleep "$SLEEP_BETWEEN"
  done
  rm -f "$PIDFILE"
  log "driver EXIT after $(cat "$COUNTER" 2>/dev/null || echo 0) pass(es)"
}

case "${1:-}" in
  start)
    if running; then echo "already running (pid $(cat "$PIDFILE"))"; exit 0; fi
    # **The state directory first, or the redirect below fails.** `>>"$DRIVER_LOG"` needs the directory
    # to exist, and the clone guard lives in `one_pass` — so a fresh repo's first `start` died on
    # `No such file or directory` before it ever checked the tree. Measured on bevy_debugger_mcp.
    if [ ! -d "$REPO/.git" ]; then
      echo "refusing to start: $REPO is not a git clone" >&2
      exit 2
    fi
    ensure_state
    # **setsid + nohup, so an ssh drop cannot kill the loop.** This is the same reason the loop's
    # own cargo jobs are launched that way.
    # **A plain re-exec of this same script**, not a serialised copy of its functions: the script is
    # the program, and a driver rebuilt from `declare -f` is a second copy that can drift from it.
    # The environment carries every setting across.
    WIGGUM_MARKER="$MARKER" setsid nohup env \
      WIGGUM_MODEL="$MODEL" WIGGUM_REPO="$REPO" WIGGUM_SLEEP="$SLEEP_BETWEEN" \
      WIGGUM_MAX="$MAX_PASSES" WIGGUM_THINK="$THINKING" \
      WIGGUM_COMMAND="$COMMAND" WIGGUM_STATE="$STATE" \
      bash "$0" __driver "$MARKER" >>"$DRIVER_LOG" 2>&1 < /dev/null &
    sleep 1
    echo "started. status: $0 status   /   follow: $0 tail"
    ;;
  __driver)
    # **The marker is an argv token, because that is the only place `pgrep -f` reads.** The environment
    # form this used to use was measured on 2026-10-26 to match nothing at all, so two drivers shared
    # one checkout. It stays in the environment as well so an operator can see which loop they are
    # looking at, but `other_driver_running` reads the argument, which arrives even when the unit starts
    # this branch directly with no `start` case in front of it.
    export WIGGUM_MARKER="${WIGGUM_MARKER:-$MARKER}"
    # **`systemd` starts this branch with no marker argument.** `ExecStart=… __driver` is the unit's whole
    # command line, so the guard would see no marker on a unit-owned driver and let a hand-started one
    # share its checkout. One `exec` puts the marker in the argv the process finally runs with, which is
    # what `pgrep -f` reads; the condition makes it happen once, never in a loop.
    if [ "${2:-}" != "$MARKER" ]; then
      exec bash "$0" __driver "$MARKER"
    fi
    driver
    ;;
  one)
    # **A smoke test that cannot lie about the model.** One pass, foreground, its output on screen.
    # **It is still a pass in this checkout**, so it must not run beside a driver that is mid-pass: the
    # same interleaving, and `one` is how an operator checks the loop while it is running. Refused rather
    # than queued, because a smoke test that has to wait is one nobody trusts the timing of.
    if other_driver_running; then
      echo "refusing to run a pass: a wiggum driver is working this checkout ($MARKER)" >&2
      echo "use '$0 status' to see it, or '$0 stop' to end it after the pass in flight" >&2
      exit 1
    fi
    one_pass
    ;;
  status)
    if running; then echo "RUNNING pid $(cat "$PIDFILE")"; else echo "not running"; fi
    echo "passes completed: $(cat "$COUNTER" 2>/dev/null || echo 0)"
    echo "consecutive failures: $(cat "$STATE/consecutive-failures" 2>/dev/null || echo 0)"
    echo "repo: $REPO   command: /$COMMAND   model: $MODEL"
    echo "state: $STATE"
    echo "--- last 5 driver lines ---"
    tail -5 "$DRIVER_LOG" 2>/dev/null || echo "(no log yet)"
    ;;
  guard)
    # **Call this before touching a repo a loop is driving.** A `git reset --hard` (mine, on
    # 2026-10-26) ran while a pass was mid-edit and silently destroyed its uncommitted source change;
    # only the untracked test file survived, because `reset --hard` leaves untracked files alone. The
    # pass then had no idea its work had gone. Exit 0 means safe to touch, 1 means a pass is running.
    if running || pgrep -f "wiggum-$USER-$(basename "$REPO")" >/dev/null 2>&1; then
      echo "A LOOP IS DRIVING $REPO — do not reset, checkout, clean or rebase it." >&2
      echo "Stop it and wait for the pass to finish: $0 stop" >&2
      exit 1
    fi
    if [ -n "$(git -C "$REPO" status --short 2>/dev/null)" ]; then
      echo "No driver, but $REPO has uncommitted work — it may be a dead pass's." >&2
      git -C "$REPO" status --short | head -10 >&2
      exit 1
    fi
    echo "safe: no loop is driving $REPO"
    ;;
  stop)
    touch "$STOPFILE"
    echo "stop requested: the driver exits after the pass in flight. Nothing is killed mid-pass."
    running && echo "pid $(cat "$PIDFILE") is still working"
    ;;
  tail)
    tail -f "$DRIVER_LOG"
    ;;
  *)
    sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
    ;;
esac
