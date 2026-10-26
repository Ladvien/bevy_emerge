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
#   WIGGUM_REPO    default $HOME/bevy_emerge
#   WIGGUM_SLEEP   seconds between passes, default 60
#   WIGGUM_MAX     stop after this many passes, default 0 (forever)
#   WIGGUM_THINK   thinking level, default medium

set -uo pipefail

MODEL="${WIGGUM_MODEL:-ollama-cloud/deepseek-v4.1-flash:1017}"
REPO="${WIGGUM_REPO:-$HOME/bevy_emerge}"
SLEEP_BETWEEN="${WIGGUM_SLEEP:-60}"
MAX_PASSES="${WIGGUM_MAX:-0}"
THINKING="${WIGGUM_THINK:-medium}"

STATE="$REPO/target/wiggum"
PIDFILE="$STATE/driver.pid"
COUNTER="$STATE/pass-count"
DRIVER_LOG="$STATE/driver.log"
PASS_LOG_DIR="$STATE/passes"
STOPFILE="$STATE/stop"
MARKER="wiggum-driver-$USER"

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
  log "pass $n START model=$MODEL dirty=$dirty"
  # **A fresh process per pass is the whole design.** `omp -p` is one-shot: it reads the prompt,
  # does the work, exits. No `--continue`, no `--resume`, no session carried over — the repository is
  # the memory.
  ( cd "$REPO" && omp --model "$MODEL" --thinking "$THINKING" -p "/build-loop" ) \
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
  if pgrep -f "${WIGGUM_MARKER:-$MARKER}" >/dev/null 2>&1; then
    log "refusing to start: another driver is already running"
    echo "another wiggum driver is already running (pgrep -f $MARKER)"
    exit 1
  fi
  echo $$ >"$PIDFILE"
  rm -f "$STOPFILE"
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
    # **setsid + nohup, so an ssh drop cannot kill the loop.** This is the same reason the loop's
    # own cargo jobs are launched that way.
    # **A plain re-exec of this same script**, not a serialised copy of its functions: the script is
    # the program, and a driver rebuilt from `declare -f` is a second copy that can drift from it.
    # The environment carries every setting across.
    WIGGUM_MARKER="$MARKER" setsid nohup env \
      WIGGUM_MODEL="$MODEL" WIGGUM_REPO="$REPO" WIGGUM_SLEEP="$SLEEP_BETWEEN" \
      WIGGUM_MAX="$MAX_PASSES" WIGGUM_THINK="$THINKING" \
      bash "$0" __driver >>"$DRIVER_LOG" 2>&1 < /dev/null &
    sleep 1
    echo "started. status: $0 status   /   follow: $0 tail"
    ;;
  __driver)
    # **The marker is what `pgrep` finds**, so one machine has one driver even across a `kill -9`
    # that left the pidfile behind.
    export WIGGUM_MARKER="${WIGGUM_MARKER:-$MARKER}"
    driver
    ;;
  one)
    # **A smoke test that cannot lie about the model.** One pass, foreground, its output on screen.
    one_pass
    ;;
  status)
    if running; then echo "RUNNING pid $(cat "$PIDFILE")"; else echo "not running"; fi
    echo "passes completed: $(cat "$COUNTER" 2>/dev/null || echo 0)"
    echo "consecutive failures: $(cat "$STATE/consecutive-failures" 2>/dev/null || echo 0)"
    echo "repo: $REPO   model: $MODEL"
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
