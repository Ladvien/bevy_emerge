#!/bin/bash
# wiggum-pane.sh <loop-name> <driver-log>
#
# What a loop's tab in Herdr's "loops" session runs: the loop's live chat (wiggum-chat.py follows the omp
# session each pass writes), its pass START/END lines, and its state reported to Herdr. If the chat view
# ever exits, the pane falls back to the driver log rather than going blank. Owner session 2026-10-30.
name=$1 log=$2
dir=$(cd "$(dirname "$0")" && pwd)
PY=$(command -v python3 || echo /usr/bin/python3)
"$PY" "$dir/wiggum-chat.py" "$name" "$log"
echo "wiggum-chat.py exited ($?); showing the driver log instead"
exec tail -n 40 -F "$log"
