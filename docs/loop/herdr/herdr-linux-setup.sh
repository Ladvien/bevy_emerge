#!/bin/bash
# Installs the Herdr "loops" session on a Linux loop host (systemd user units). Idempotent.
set -e
export PATH=$HOME/.local/bin:$PATH
command -v herdr >/dev/null || curl -fsSL https://herdr.dev/install.sh | sh >/dev/null 2>&1
chmod +x ~/.local/share/wiggum/wiggum-pane.sh ~/.local/share/wiggum/herdr-loops-sync.sh
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/herdr-loops.service <<'U'
[Unit]
Description=Herdr headless server, session "loops": one tab per Wiggum loop on this host (owner, 2026-10-30)

[Service]
Type=simple
ExecStart=%h/.local/bin/herdr --session loops server
ExecStartPost=/bin/sh -c 'sleep 3; %h/.local/share/wiggum/herdr-loops-sync.sh --fresh'
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
U
cat > ~/.config/systemd/user/herdr-loops-sync.service <<'U'
[Unit]
Description=Add a Herdr tab for any new Wiggum loop on this host

[Service]
Type=oneshot
ExecStart=%h/.local/share/wiggum/herdr-loops-sync.sh
U
cat > ~/.config/systemd/user/herdr-loops-sync.timer <<'U'
[Unit]
Description=Every 5 minutes: a Herdr tab for any new Wiggum loop on this host

[Timer]
OnBootSec=2min
OnUnitActiveSec=5min

[Install]
WantedBy=timers.target
U
systemctl --user daemon-reload
systemctl --user enable herdr-loops herdr-loops-sync.timer >/dev/null 2>&1
systemctl --user restart herdr-loops
systemctl --user start herdr-loops-sync.timer
sleep 6
herdr --session loops agent list | python3 -c 'import json,sys; [print(" ", a["agent"], a["agent_status"]) for a in json.load(sys.stdin)["result"]["agents"]]'
