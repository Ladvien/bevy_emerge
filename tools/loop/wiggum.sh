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
#
# Environment, all optional:
#   WIGGUM_MODEL   default ollama-cloud/deepseek-v4.1-flash:1017
#   WIGGUM_REPO    default $HOME/bevy_emerge — any git clone
#   WIGGUM_COMMAND slash command a pass runs, default build-loop
#   WIGGUM_STATE   where the driver keeps its logs, default ~/.local/state/wiggum/<repo>
#   WIGGUM_SLEEP   seconds between passes, default 60
#   WIGGUM_MAX     stop after this many passes, default 0 (forever)
#   WIGGUM_THINK   thinking level, default medium

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
  local dirty; dirty=$(git -C "$REPO" status --short 2>/dev/null | wc -l | tr -d ' ')
  if [ "${dirty:-0}" -gt 40 ]; then
    echo "ABORTED: $REPO has $dirty modified files — a scratch tree, not a clone. Fix it by hand." >&2
    return 1
  fi

  ensure_state
  local n; n=$(( $(cat "$COUNTER" 2>/dev/null || echo 0) + 1 ))
  echo "$n" >"$COUNTER"
  local passlog="$PASS_LOG_DIR/pass-$(printf '%04d' "$n")-$(date '+%Y%m%d-%H%M%S').log"
  log "pass $n START command=/$COMMAND model=$MODEL dirty=$dirty"
  # **A fresh process per pass is the whole design.** `omp -p` is one-shot: it reads the prompt,
  # does the work, exits. No `--continue`, no `--resume`, no session carried over — the repository is
  # the memory. `--model` is explicit so a pass never inherits an interactive choice.
  ( cd "$REPO" && omp --model "$MODEL" --thinking "$THINKING" -p "/$COMMAND" ) \
    >"$passlog" 2>&1
  local rc=$?
  log "pass $n END rc=$rc log=$passlog"

  # **A tree left dirty is the next pass's first bug**, and the command says so. This only reports it.
  local after; after=$(git -C "$REPO" status --short 2>/dev/null | wc -l | tr -d ' ')
  [ "${after:-0}" -gt 0 ] && log "pass $n WARNING: $after uncommitted file(s) left behind"
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
