#!/bin/bash
# Installs the Herdr "loops" session on a macOS loop host (launchd agents). Idempotent.
set -e
export PATH=/opt/homebrew/bin:$HOME/.local/bin:$PATH
command -v herdr >/dev/null || brew install herdr >/dev/null 2>&1
ln -sf /opt/homebrew/bin/herdr "$HOME/.cargo/bin/herdr"; HERDR=$HOME/.cargo/bin/herdr
W=$HOME/.local/share/wiggum
chmod +x "$W/wiggum-pane.sh" "$W/herdr-loops-sync.sh"
mkdir -p "$HOME/Library/Logs/herdr-loops"
uid=$(id -u)
P=$HOME/Library/LaunchAgents/com.ladvien.herdr-loops.plist
cat > "$P" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key><string>com.ladvien.herdr-loops</string>
	<key>ProgramArguments</key>
	<array><string>$HERDR</string><string>--session</string><string>loops</string><string>server</string></array>
	<key>EnvironmentVariables</key>
	<dict><key>PATH</key><string>/opt/homebrew/bin:$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin</string></dict>
	<key>RunAtLoad</key><true/>
	<key>KeepAlive</key><true/>
	<key>StandardOutPath</key><string>$HOME/Library/Logs/herdr-loops/server.log</string>
	<key>StandardErrorPath</key><string>$HOME/Library/Logs/herdr-loops/server.log</string>
</dict>
</plist>
PL
S=$HOME/Library/LaunchAgents/com.ladvien.herdr-loops-sync.plist
cat > "$S" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key><string>com.ladvien.herdr-loops-sync</string>
	<key>ProgramArguments</key>
	<array><string>/bin/bash</string><string>$W/herdr-loops-sync.sh</string></array>
	<key>EnvironmentVariables</key>
	<dict><key>PATH</key><string>/opt/homebrew/bin:$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin</string></dict>
	<key>StartInterval</key><integer>300</integer>
	<key>RunAtLoad</key><true/>
	<key>StandardOutPath</key><string>$HOME/Library/Logs/herdr-loops/sync.log</string>
	<key>StandardErrorPath</key><string>$HOME/Library/Logs/herdr-loops/sync.log</string>
</dict>
</plist>
PL
for l in com.ladvien.herdr-loops com.ladvien.herdr-loops-sync; do launchctl bootout gui/$uid/$l >/dev/null 2>&1 || true; done
launchctl bootstrap gui/$uid "$P"
sleep 3
bash "$W/herdr-loops-sync.sh" --fresh
launchctl bootstrap gui/$uid "$S"
sleep 4
herdr --session loops agent list | python3 -c 'import json,sys; [print(" ", a["agent"], a["agent_status"]) for a in json.load(sys.stdin)["result"]["agents"]]'
