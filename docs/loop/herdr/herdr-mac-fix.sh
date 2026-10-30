#!/bin/bash
# Put the Herdr "loops" server back under launchd and refill its tabs.
H=$HOME/.cargo/bin/herdr
uid=$(id -u)
"$H" --session loops server stop >/dev/null 2>&1
for i in 1 2 3 4 5 6 7 8 9 10; do "$H" --session loops status server >/dev/null 2>&1 || break; sleep 1; done
for l in com.ladvien.herdr-loops com.ladvien.herdr-loops-sync; do
  for i in 1 2 3 4 5; do
    launchctl bootstrap gui/$uid "$HOME/Library/LaunchAgents/$l.plist" 2>/dev/null && break
    launchctl bootout gui/$uid/$l >/dev/null 2>&1; sleep 2
  done
  [ $l = com.ladvien.herdr-loops ] && { for i in 1 2 3 4 5 6 7 8 9 10; do "$H" --session loops status server >/dev/null 2>&1 && break; sleep 1; done; bash "$HOME/.local/share/wiggum/herdr-loops-sync.sh" --fresh; }
done
for l in com.ladvien.herdr-loops com.ladvien.herdr-loops-sync; do printf '%s: %s\n' "$l" "$(launchctl print gui/$uid/$l 2>&1 | grep -m1 -E 'state =|Could not')"; done
"$H" --session loops agent list | python3 -c 'import json,sys; print([(a["agent"],a["agent_status"]) for a in json.load(sys.stdin)["result"]["agents"]])'
