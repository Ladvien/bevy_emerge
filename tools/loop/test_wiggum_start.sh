#!/bin/bash
# Tests for `start` and the dirty guard, written 2026-10-28 on a Mac (bash 3.2, no setsid): start works and reports
# truthfully, the default model is not the dead one, WIGGUM_MAX_DIRTY bounds the dirty guard. Throwaway repo, fake omp.
# Usage: bash tools/loop/test_wiggum_start.sh <path-to-wiggum.sh>   (about 30 seconds)
set -u
DRV="$1"
T=$(mktemp -d /tmp/wig_start_test.XXXXXX); pass=0; fail=0
ok()   { pass=$((pass+1)); echo "ok    $*"; }
bad()  { fail=$((fail+1)); echo "FAIL  $*"; }
cleanup() { WIGGUM_REPO="$T/repo" WIGGUM_STATE="$T/state" bash "$DRV" stop >/dev/null 2>&1; rm -rf "$T"; }
trap cleanup EXIT

# a throwaway repo with an origin, and a fake omp that finishes quickly with rc 0
git init -q "$T/origin.git" --bare -b main
git init -q "$T/repo" -b main && cd "$T/repo" && git config user.email t@t && git config user.name t
echo hi > a.txt && git add a.txt && git commit -q -m init && git remote add origin "$T/origin.git" && git push -q origin main
mkdir -p "$T/bin" && cat > "$T/bin/omp" <<'EOF'
#!/bin/bash
echo "fake omp: $*" ; sleep 2 ; exit 0
EOF
chmod +x "$T/bin/omp"
export PATH="$T/bin:$PATH"
export WIGGUM_REPO="$T/repo" WIGGUM_STATE="$T/state" WIGGUM_SLEEP=1 WIGGUM_IDLE_MAX=0 WIGGUM_MIN_FREE_GB=0 WIGGUM_COMMAND=build-loop
unset WIGGUM_MODEL

echo "this machine: setsid=$(command -v setsid || echo MISSING) perl=$(command -v perl) bash=${BASH_VERSION}"

# 1. start succeeds and says so with a pid; the driver is really running
out=$(bash "$DRV" start 2>&1); rc=$?
echo "$out" | head -3
[ $rc -eq 0 ] && echo "$out" | grep -q '^started (pid ' && ok "start reports started with a pid (rc 0)" || bad "start did not report a pid (rc $rc)"
pid=$(cat "$T/state/driver.pid" 2>/dev/null); kill -0 "$pid" 2>/dev/null && ok "the driver process is alive (pid $pid)" || bad "no live driver after start"

# 2. the default model is the live one, not ollama-cloud
sleep 2
grep -q 'model=claude-sonnet-5-5' "$T/state/driver.log" && ok "driver.log says model=claude-sonnet-5-5 (the default)" || { bad "default model wrong"; grep 'driver START' "$T/state/driver.log" | head -1; }
grep -q 'ollama' "$T/state/driver.log" && bad "ollama still appears in driver.log" || ok "no ollama in driver.log"

# 3. a pass ran (fake omp) so the whole pass path works under this start
sleep 4
grep -q -E 'pass [0-9]+ (START|END)' "$T/state/driver.log" && ok "a pass started and ended under the started driver" || bad "no pass in driver.log"

# 4. a second start refuses and says already running
out2=$(bash "$DRV" start 2>&1); echo "$out2" | grep -q 'already running' && ok "second start: already running" || bad "second start: '$out2'"

# 5. stop ends it cleanly. A pass in flight is ended with TERM and then KILL after 5 s, so allow up to 15 s.
bash "$DRV" stop >/dev/null 2>&1
for _ in $(seq 1 15); do kill -0 "$pid" 2>/dev/null || break; sleep 1; done
kill -0 "$pid" 2>/dev/null && bad "driver still alive 15 s after stop" || ok "stop ended the driver"

# 6. a launch that cannot come up is reported as FAILED with the log tail, rc 1 (not 'started.')
out3=$(WIGGUM_STATE="/dev/null/nope" bash "$DRV" start 2>&1); rc3=$?
echo "$out3" | head -2
[ $rc3 -ne 0 ] && echo "$out3" | grep -q 'FAILED to start' && ok "an impossible launch says FAILED (rc $rc3), not 'started'" || bad "impossible launch: rc $rc3, '$out3'"
echo "$out3" | grep -q '^started' && bad "claimed started on an impossible launch" || ok "never printed 'started' on the impossible launch"

# 7. WIGGUM_MAX_DIRTY: 45 untracked files, PARK off: refused at the default limit, allowed at 100
cd "$T/repo" && for i in $(seq 1 45); do echo x > "scratch_$i.txt"; done
rm -rf "$T/state2"
o4=$(WIGGUM_STATE="$T/state2" WIGGUM_PARK=0 WIGGUM_CHECKPOINT=0 bash "$DRV" one 2>&1); echo "$o4" | grep -q 'ABORTED: .* has 45 modified files (WIGGUM_MAX_DIRTY=40)' && ok "default limit 40 refuses a 45-file tree and names the knob" || { bad "default limit did not refuse"; echo "$o4" | head -3; }
rm -rf "$T/state3"
o5=$(WIGGUM_STATE="$T/state3" WIGGUM_PARK=0 WIGGUM_CHECKPOINT=0 WIGGUM_MAX_DIRTY=100 bash "$DRV" one 2>&1); echo "$o5" | grep -q 'ABORTED' && { bad "WIGGUM_MAX_DIRTY=100 still refused"; echo "$o5" | head -3; } || ok "WIGGUM_MAX_DIRTY=100 lets the 45-file tree through"
# and the tree was NOT parked or touched (PARK=0): the 45 files are still untracked, still on main
n=$(git -C "$T/repo" status --short | wc -l | tr -d ' '); b=$(git -C "$T/repo" branch --show-current)
[ "$n" -eq 45 ] && [ "$b" = main ] && ok "with PARK=0 the 45 files are untouched and the checkout is still on main" || bad "tree changed: $n files, branch $b"

echo; echo "== $pass passed, $fail failed"; [ $fail -eq 0 ]
