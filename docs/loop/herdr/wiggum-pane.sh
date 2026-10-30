#!/bin/bash
# wiggum-pane.sh <loop-name> <driver-log>
#
# Runs inside a Herdr pane (session "loops", one tab per loop). Shows the loop's driver log and reports the
# loop's state to Herdr from it, so the sidebar of any Herdr client attached to this host (or to all hosts,
# with `herdr machine add`) says which loop is working, idle or failing:
#   pass START                  -> working
#   pass END rc=0, idle, STOP   -> idle
#   rc!=0, TIMED OUT, failed    -> blocked (needs a look)
# Owner session 2026-10-30. Reads only; never touches the loop.
name=$1 log=$2
HERDR=$(command -v herdr || echo "$HOME/.local/bin/herdr")
[ -x "$HERDR" ] || HERDR=/opt/homebrew/bin/herdr
seq=0
report() {
  seq=$((seq + 1))
  "$HERDR" pane report-agent "$HERDR_PANE_ID" --source wiggum --agent "$name" --state "$1" \
    --message "$2" --seq "$seq" >/dev/null 2>&1
}
printf '\033]0;%s\007' "$name"
touch "$log" 2>/dev/null
tail -n 60 -F "$log" 2>/dev/null | while IFS= read -r line; do
  case "$line" in
    *SNAPSHOT*) continue ;;
  esac
  printf '%s\n' "$line"
  msg=${line#* }
  case "$line" in
    *" START command="*) report working "${msg:0:80}" ;;
    *"END rc=0 "*|*" idle ("*|*"driver STOP"*|*"driver EXIT"*) report idle "${msg:0:80}" ;;
    *"END rc="*|*"TIMED OUT"*|*"pass failed"*|*"REFUSED"*) report blocked "${msg:0:80}" ;;
  esac
done
