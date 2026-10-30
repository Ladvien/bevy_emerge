#!/bin/bash
# herdr-loops-sync.sh [--fresh]
#
# Makes this host's Herdr session "loops" hold one tab per Wiggum loop that ran here in the last 12 hours
# (every ~/.local/state/wiggum/<repo>/driver.log), each running wiggum-pane.sh on that loop's driver log.
# Idempotent: an existing tab is left alone. --fresh closes every tab first (after a server restart, whose
# restored panes are plain shells). Owner session 2026-10-30.
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
for log in "$HOME"/.local/state/wiggum/*/driver.log; do
  [ -f "$log" ] || continue
  [ -n "$(find "$log" -mmin -720 2>/dev/null)" ] || continue
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
