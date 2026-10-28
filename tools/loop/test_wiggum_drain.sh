#!/bin/bash
# Prove drain against a fake `omp`: the pass in flight is never cut (rc=0, its work lands), the driver exits
# only AFTER it, an idle driver drains within seconds, a stale drain file does not stop a fresh driver, and plain
# `stop` still ends the pass (rc=143), so the two are different on purpose.
# Usage: bash tools/loop/test_wiggum_drain.sh <path-to-wiggum.sh>   (about a minute; macOS bash 3.2 and Linux bash 5)
set -uo pipefail
DRIVER="$1"
T=/tmp/drain_t
rm -rf "$T"; mkdir -p "$T/bin"
bash -n "$DRIVER" && echo "parse: ok"
cat > "$T/bin/omp" <<'EOF'
#!/bin/bash
[ -n "${FAKE_OMP_SLEEP:-}" ] && sleep "$FAKE_OMP_SLEEP"
[ -n "${FAKE_OMP_MARK:-}" ] && echo "done $(date +%s)" >> "$FAKE_OMP_MARK"
exit 0
EOF
chmod +x "$T/bin/omp"

pass=0; fail=0
check() { if eval "$2" >/dev/null 2>&1; then pass=$((pass+1)); printf '  ok    %s\n' "$1"; else fail=$((fail+1)); printf '  FAIL  %s\n' "$1"; fi; }
fresh() {
  rm -rf "$T/o" "$T/r" "$T/state" "$T/mark"
  git init -q --bare "$T/o" && git -C "$T/o" symbolic-ref HEAD refs/heads/main
  git clone -q "$T/o" "$T/r" 2>/dev/null
  ( cd "$T/r" && git config user.email t@t && git config user.name t && echo one > f && mkdir -p .omp/commands \
    && echo noop > .omp/commands/probe.md && git add -A && git commit -qm base && git branch -M main && git push -q -u origin main )
}
cenv() { env PATH="$T/bin:$PATH" FAKE_OMP_MARK="$T/mark" WIGGUM_REPO="$T/r" WIGGUM_COMMAND=probe WIGGUM_STATE="$T/state" \
  WIGGUM_SLEEP="${SLEEP:-30}" WIGGUM_MIN_FREE_GB=1 WIGGUM_CHECKPOINT=0 WIGGUM_IDLE_MAX=0 WIGGUM_MAX="${MAXP:-0}" "$@"; }
start() { cenv "$@" bash "$DRIVER" __driver "wiggum-$USER-repo" > "$T/out.log" 2>&1 & DPID=$!; }
ctl()   { cenv "$@" ; }                      # prefix for `bash $DRIVER <cmd>`
log()   { cat "$T/state/driver.log" 2>/dev/null; }
waitlog() { local re="$1" cap="${2:-30}" i=0; while [ $i -lt $((cap*2)) ]; do log | grep -q -E "$re" && return 0; sleep 0.5; i=$((i+1)); done; return 1; }
gone()  { local cap="${1:-30}" i=0; while [ $i -lt $((cap*2)) ]; do kill -0 "$DPID" 2>/dev/null || return 0; sleep 0.5; i=$((i+1)); done; return 1; }
cleanup() { kill "$DPID" 2>/dev/null; pkill -f "$T/bin" 2>/dev/null; true; }

echo "== D1 idle driver (between passes, in its sleep): drain exits within seconds, no second pass"
fresh; start FAKE_OMP_SLEEP=0; waitlog 'pass 1 END' 20
t0=$(date +%s); ctl bash "$DRIVER" drain --wait > "$T/d1.out" 2>&1; rc=$?; t1=$(date +%s)
check "drain --wait returned 0"            '[ "$rc" = 0 ]'
check "it took under 8 s (a 30 s sleep was interrupted)" '[ $((t1 - t0)) -lt 8 ]'
check "the driver is gone"                  'gone 5'
check "no second pass ran"                  '[ "$(cat $T/state/pass-count)" = 1 ]'
check "log says drain, not stop"            'log | grep -q "driver STOP: drain requested" && ! log | grep -q "driver STOP: stop file"'
check "the one pass ended rc=0"             'log | grep -q "pass 1 END rc=0"'

echo "== D2 drain DURING a pass: the pass finishes, its work lands, the driver exits after it"
fresh; start FAKE_OMP_SLEEP=8; waitlog 'pass 1 START' 20; sleep 1
ctl bash "$DRIVER" drain --wait > "$T/d2.out" 2>&1; rc=$?
check "drain --wait returned 0"            '[ "$rc" = 0 ]'
check "the pass was NOT cut: END rc=0"      'log | grep -q "pass 1 END rc=0 timed_out=0"'
check "its work landed (marker written)"    '[ -s "$T/mark" ]'
check "no pass was cut with rc=143"         '! log | grep -q "rc=143"'
check "no second pass started"             '! log | grep -q "pass 2 START"'
check "log says drain"                      'log | grep -q "driver STOP: drain requested"'

echo "== D3 a stale drain file does not stop a FRESH driver"
fresh; mkdir -p "$T/state"; touch "$T/state/drain"; MAXP=1 SLEEP=1 start FAKE_OMP_SLEEP=0; gone 30
check "the fresh driver ran its pass"       'log | grep -q "pass 1 END rc=0"'
check "the stale file was cleared at start" '[ ! -e "$T/state/drain" ]'

echo "== D4 drain with no driver running: says so, exits 0, leaves nothing behind"
fresh; mkdir -p "$T/state"; out=$(ctl bash "$DRIVER" drain 2>&1); rc=$?
check "exit 0 and a clear message"          '[ "$rc" = 0 ] && echo "$out" | grep -q "no driver is running"'
check "no drain file left to surprise a later start" '[ ! -e "$T/state/drain" ]'

echo "== D5 plain STOP still ends the pass in flight (rc=143): the two are different on purpose"
fresh; start FAKE_OMP_SLEEP=25; waitlog 'pass 1 START' 20; sleep 1
ctl bash "$DRIVER" stop > /dev/null 2>&1; gone 25
check "the pass was cut: END rc=143"        'log | grep -q "pass 1 END rc=143"'
check "the driver exited"                   '! kill -0 $DPID 2>/dev/null'
check "its work did NOT land"               '[ ! -s "$T/mark" ]'

echo "== D6 status says DRAINING while the pass runs; --wait has a cap and leaves the file"
fresh; start FAKE_OMP_SLEEP=10; waitlog 'pass 1 START' 20; sleep 1
ctl bash "$DRIVER" drain > /dev/null 2>&1
st=$(ctl bash "$DRIVER" status 2>&1)
check "status shows DRAINING"               'grep -q "^DRAINING" <<<"$st"'
out=$(ctl WIGGUM_DRAIN_WAIT=3 bash "$DRIVER" drain --wait 2>&1); rc=$?
check "a capped wait gives up with exit 1"  '[ "$rc" = 1 ] && echo "$out" | grep -q "still running after 3s"'
check "the drain file is still there"       '[ -e "$T/state/drain" ]'
gone 25
check "and the pass still finished, then the driver exited" 'log | grep -q "pass 1 END rc=0" && ! kill -0 $DPID 2>/dev/null'

echo; echo "RESULT: $pass ok, $fail failed"
cleanup
[ "$fail" = 0 ]
