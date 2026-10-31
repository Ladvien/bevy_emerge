#!/bin/bash
# herdr-loops-sync.sh [--fresh]
#
# Makes this host's Herdr session "loops" hold one tab per LIVE Wiggum loop (every ~/.local/state/wiggum/<repo>/
# driver.log whose driver is running), each running wiggum-pane.sh on that loop's driver log, and closes the tab of
# every loop that is not live any more: a worker retired, a loop moved to another host, a driver that gave up.
# A log is live when the pid on its last "driver START" line is still a wiggum driver, or when the log changed in the
# last 10 minutes (the pause before a supervisor restarts a driver, and the last lines of a loop just retired).
# Idempotent: a live loop's tab is left alone. --fresh closes every tab first (after a server restart, whose
# restored panes are plain shells). Owner session 2026-10-30; live-only 2026-10-31 ("clean up herdr").
HERDR=$(command -v herdr || echo "$HOME/.local/bin/herdr")
[ -x "$HERDR" ] || HERDR=/opt/homebrew/bin/herdr
h() { "$HERDR" --session loops "$@" 2>/dev/null; }
pane_sh="$HOME/.local/share/wiggum/wiggum-pane.sh"
js() { python3 -c "import json,sys; d=json.load(sys.stdin); $1"; }

h status server >/dev/null || { echo "herdr loops server is not running" >&2; exit 1; }
if [ "${1:-}" = "--fresh" ]; then
  for t in $(h tab list | js 'print("\n".join(t["tab_id"] for t in d["result"]["tabs"]))'); do h tab close "$t" >/dev/null; done
fi
labels=$(h tab list | js 'print("\n".join(t.get("label","") for t in d["result"]["tabs"]))')

loop_live() {
  local log=$1 pid
  [ -n "$(find "$log" -mmin -10 2>/dev/null)" ] && return 0
  pid=$(grep -o 'driver START pid=[0-9]*' "$log" | tail -1 | cut -d= -f2)
  [ -n "$pid" ] && ps -p "$pid" -o args= 2>/dev/null | grep -q wiggum
}
live=""
for log in "$HOME"/.local/state/wiggum/*/driver.log; do
  [ -f "$log" ] || continue
  loop_live "$log" || continue
  live="$live $(basename "$(dirname "$log")")"
done

for log in "$HOME"/.local/state/wiggum/*/driver.log; do
  [ -f "$log" ] || continue
  case " $live " in *" $(basename "$(dirname "$log")") "*) ;; *) continue ;; esac
  name=$(basename "$(dirname "$log")")
  printf '%s\n' "$labels" | grep -qx "$name" && continue
  ws=$(h workspace list | js 'w=d["result"]["workspaces"]; print(w[0]["workspace_id"] if w else "")')
  if [ -z "$ws" ]; then
    out=$(h workspace create --cwd "$HOME" --label "loops@$(hostname -s)" --no-focus)
    tab=$(printf '%s' "$out" | js 'print(d["result"]["tab"]["tab_id"])')
    pane=$(printf '%s' "$out" | js 'print(d["result"]["root_pane"]["pane_id"])')
    h tab rename "$tab" "$name" >/dev/null
  else
    out=$(h tab create --workspace "$ws" --cwd "$HOME" --label "$name" --no-focus)
    pane=$(printf '%s' "$out" | js 'print(d["result"]["root_pane"]["pane_id"])')
  fi
  [ -n "$pane" ] && h pane run "$pane" "bash '$pane_sh' '$name' '$log'" >/dev/null && echo "added $name ($pane)"
  labels="$labels
$name"
done

# Close the tabs of loops that are gone. Never when nothing is live: a server whose last tab closes loses its workspace.
if [ -n "${live// /}" ]; then
  h tab list | js 'print("\n".join(t["tab_id"] + " " + t.get("label", "") for t in d["result"]["tabs"]))' | while read -r tid label; do
    case " $live " in *" $label "*) ;; *) h tab close "$tid" </dev/null >/dev/null && echo "closed $label" ;; esac
  done
fi
