#!/bin/bash
# Exercise the driver's park step against a fake `omp`: no model, no waiting. Each case builds a throwaway
# origin and clone, puts the clone in one stranded state, runs exactly ONE pass of the real driver script,
# and checks what the pass would have SEEN when it started (the fake omp records branch and dirty count),
# plus the git state the driver left. Usage: bash tools/loop/test_wiggum_park.sh <path-to-wiggum.sh>   (2 minutes)
set -uo pipefail
DRIVER="$1"
T=/tmp/park_t
rm -rf "$T"; mkdir -p "$T/bin"
bash -n "$DRIVER" && echo "parse: ok"
cat > "$T/bin/omp" <<'EOF'
#!/bin/bash
r="$WIGGUM_REPO"
echo "branch=$(git -C "$r" symbolic-ref --short -q HEAD || echo DETACHED) dirty=$(git -C "$r" status --short | wc -l | tr -d ' ')" >> "$FAKE_OMP_SEEN"
[ -n "${FAKE_OMP_WRITE_MB:-}" ] && head -c $(( FAKE_OMP_WRITE_MB * 1048576 )) /dev/urandom > "$r/leftover.bin"
exit 0
EOF
chmod +x "$T/bin/omp"

pass=0; fail=0
check() { # check <name> <shell expression>
  if eval "$2" >/dev/null 2>&1; then pass=$((pass+1)); printf '  ok    %s\n' "$1"; else fail=$((fail+1)); printf '  FAIL  %s\n' "$1"; fi
}
fresh() {
  rm -rf "$T/o" "$T/r" "$T/r2" "$T/w" "$T/state" "$T/seen"
  git init -q --bare "$T/o" && git -C "$T/o" symbolic-ref HEAD refs/heads/main
  git clone -q "$T/o" "$T/r" 2>/dev/null
  ( cd "$T/r" && git config user.email t@t && git config user.name t && echo one > f && echo two > g \
    && mkdir -p .omp/commands && echo noop > .omp/commands/probe.md && git add -A && git commit -qm base \
    && git branch -M main && git push -q -u origin main )
}
drive() {
  rm -rf "$T/state" "$T/seen"; mkdir -p "$T/state"; echo 7 > "$T/state/pass-count"   # pass 7 left the work; pass 8 is about to start
  env PATH="$T/bin:$PATH" FAKE_OMP_SEEN="$T/seen" WIGGUM_REPO="$T/r" WIGGUM_COMMAND=probe WIGGUM_STATE="$T/state" \
      WIGGUM_SLEEP=1 WIGGUM_MIN_FREE_GB=1 WIGGUM_CHECKPOINT=0 WIGGUM_MAX=8 WIGGUM_IDLE_MAX=0 "$@" \
      bash "$DRIVER" __driver "wiggum-$USER-repo" > "$T/out.log" 2>&1
}
seen() { cat "$T/seen" 2>/dev/null | tr '\n' ' ' | sed 's/ $//'; }
parked() { git -C "$T/r" branch --list 'wiggum/parked-pass-7-*' | tr -d ' *' | head -1; }
log() { cat "$T/state/driver.log" 2>/dev/null; }

echo "== P1 clean on main: nothing happens"
fresh; drive
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "no branch cut, nothing logged"   '[ -z "$(parked)" ] && ! log | grep -q "pre: parked"'

echo "== P2 dirty on main: parked on a new branch, main untouched, branch pushed"
fresh; echo changed >> "$T/r/f"; echo new > "$T/r/newfile"; before=$(git -C "$T/r" rev-parse main); drive; br=$(parked)
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "a wiggum/parked branch was cut"  '[ -n "$br" ]'
check "its commit holds both files"     '[ "$(git -C "$T/r" show --name-only --format= "$br" | sort | tr "\n" " ")" = "f newfile " ]'
check "local main did not move"         '[ "$(git -C "$T/r" rev-parse main)" = "$before" ]'
check "origin main did not move"        '[ "$(git -C "$T/o" rev-parse main)" = "$before" ]'
check "parked branch is on origin"      '[ "$(git -C "$T/o" rev-parse "$br")" = "$(git -C "$T/r" rev-parse "$br")" ]'
check "the recovery grep every prompt carries finds it" 'git -C "$T/r" log --all --oneline --grep="WIP(pass" | grep -q "WIP(pass 7, parked): 2 file(s) left behind"'
check "log says what and where"         'log | grep -q "pass 8 pre: parked 2 file(s) on wiggum/parked-pass-7.*pushed: yes.*back on main"'

echo "== P3 dirty on a loop branch: committed ON that branch, no new branch"
fresh; ( cd "$T/r" && git checkout -qb loop/iter-9 && echo l > loopfile && git add loopfile && git commit -qm "iter 9" ); echo dirt >> "$T/r/f"; drive
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "dirty file committed on loop/iter-9" '[ "$(git -C "$T/r" show --name-only --format= loop/iter-9 | tr "\n" " ")" = "f " ]'
check "its title is a WIP(pass 7, parked) the grep finds" '[ "$(git -C "$T/r" log -1 --format=%s loop/iter-9 | cut -c1-20)" = "WIP(pass 7, parked):" ] && git -C "$T/r" log --all --oneline --grep="WIP(pass" | grep -q "WIP(pass 7, parked)"'
check "no wiggum/parked branch"         '[ -z "$(git -C "$T/r" branch --list "wiggum/*")" ]'
check "loop/iter-9 on origin, same tip" '[ "$(git -C "$T/o" rev-parse loop/iter-9)" = "$(git -C "$T/r" rev-parse loop/iter-9)" ]'

echo "== P4 clean on a loop branch with an unpushed commit: pushed, back on main"
fresh; ( cd "$T/r" && git checkout -qb loop/iter-9 && echo l > loopfile && git add loopfile && git commit -qm "iter 9" ); drive
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "the commit reached origin"       '[ "$(git -C "$T/o" rev-parse loop/iter-9)" = "$(git -C "$T/r" rev-parse loop/iter-9)" ]'

echo "== P5 detached on a commit nothing holds: a branch is cut for it"
fresh; ( cd "$T/r" && git checkout -q --detach && echo d > loose && git add loose && git commit -qm loose ); loose=$(git -C "$T/r" rev-parse HEAD); drive; br=$(parked)
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "the branch points at that commit" '[ "$(git -C "$T/r" rev-parse "$br")" = "$loose" ]'
check "and is on origin"                '[ "$(git -C "$T/o" rev-parse "$br")" = "$loose" ]'

echo "== P5b detached on a commit main already has, clean: just go back, cut nothing"
fresh; git -C "$T/r" checkout -q --detach; drive
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "no wiggum branch"                '[ -z "$(git -C "$T/r" branch --list "wiggum/*")" ]'

echo "== P6 main held by a CLEAN worktree: the holder is detached, the clone returns"
fresh; ( cd "$T/r" && git checkout -qb loop/iter-9 && echo l > loopfile && git add loopfile && git commit -qm "iter 9" ); git -C "$T/r" worktree add -q "$T/w" main; drive
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "the holder is now detached"      '! git -C "$T/w" symbolic-ref -q HEAD'
check "log names the holder"            'log | grep -q "detached .*park_t/w"'

echo "== P7 main held by a worktree WITH CHANGES: refused, nothing destroyed"
fresh; ( cd "$T/r" && git checkout -qb loop/iter-9 && echo l > loopfile && git add loopfile && git commit -qm "iter 9" ); git -C "$T/r" worktree add -q "$T/w" main; echo mine >> "$T/w/f"; drive
check "the pass never started"          '[ ! -e "$T/seen" ]'
check "log says why"                    'log | grep -q "ABORTED: could not return to main"'
check "the holder keeps its change"     '[ "$(git -C "$T/w" status --short | wc -l | tr -d " ")" = 1 ] && git -C "$T/w" symbolic-ref -q HEAD'

echo "== P8 mid-cherry-pick: refused, nothing touched"
fresh; ( cd "$T/r" && git checkout -qb a && echo A > f && git commit -qam a && git checkout -q main && git checkout -qb b && echo B > f && git commit -qam b && { git cherry-pick a >/dev/null 2>&1 || true; } )
drive
check "the pass never started"          '[ ! -e "$T/seen" ]'
check "log names the state"             'log | grep -q "ABORTED: .*in the middle of CHERRY_PICK_HEAD"'
check "no wiggum branch cut"            '[ -z "$(git -C "$T/r" branch --list "wiggum/*")" ]'
check "the conflict is still there"     'git -C "$T/r" status --short | grep -q "^UU f"'

echo "== P9 WIGGUM_PARK=0: the old behaviour, left alone"
fresh; echo c >> "$T/r/f"; drive WIGGUM_PARK=0
check "pass saw the dirty tree"         '[ "$(seen)" = "branch=main dirty=1" ]'
check "no branch cut"                   '[ -z "$(git -C "$T/r" branch --list "wiggum/*")" ]'

echo "== P10 no reachable origin: committed locally, tree clean, pass proceeds"
fresh; echo c >> "$T/r/f"; git -C "$T/r" remote set-url origin /nonexistent/origin; drive; br=$(parked)
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "the work is on the local branch" '[ -n "$br" ] && [ "$(git -C "$T/r" show --name-only --format= "$br" | tr "\n" " ")" = "f " ]'
check "log says push failed"            'log | grep -q "pushed: no (push failed, kept local)"'

echo "== P11 oversize leftovers: committed, not pushed"
fresh; head -c 2000000 /dev/zero > "$T/r/big.bin"; drive WIGGUM_PUSH_MAX_MB=1; br=$(parked)
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "committed locally"               '[ -n "$br" ] && git -C "$T/r" show --name-only --format= "$br" | grep -q big.bin'
check "origin does not have the branch" '[ -z "$(git -C "$T/o" branch --list "$br")" ]'
check "log says why"                    'log | grep -q "over WIGGUM_PUSH_MAX_MB"'

echo "== P11b the DEFAULT cap stops the real case: six 2 MB data files (12 MB), not one big file"
fresh; for i in 1 2 3 4 5 6; do head -c 2097152 /dev/urandom > "$T/r/data$i.csv"; done; drive; br=$(parked)
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "committed locally with all six"  '[ -n "$br" ] && [ "$(git -C "$T/r" show --name-only --format= "$br" | grep -c "data.\.csv")" = 6 ]'
check "origin does not have the branch" '[ -z "$(git -C "$T/o" branch --list "$br")" ]'
check "log names the cap"               'log | grep -q "12 MB of new data is over WIGGUM_PUSH_MAX_MB, kept local"'

echo "== P11c 5 MB of leftovers is under the default: pushed as usual"
fresh; head -c 5242880 /dev/urandom > "$T/r/notes.bin"; drive; br=$(parked)
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "the branch is on origin"         '[ -n "$br" ] && [ "$(git -C "$T/o" rev-parse "$br" 2>/dev/null)" = "$(git -C "$T/r" rev-parse "$br")" ]'

echo "== S1 snapshot: a pass that leaves 12 MB behind gets a LOCAL-ONLY snapshot, nothing on origin"
fresh; drive FAKE_OMP_WRITE_MB=12
check "a snapshot ref exists locally"   '[ "$(git -C "$T/r" for-each-ref refs/wiggum | wc -l | tr -d " ")" -ge 1 ]'
check "none reached origin"             '[ "$(git -C "$T/o" for-each-ref refs/wiggum | wc -l | tr -d " ")" = 0 ]'
check "log says local only and why"     'log | grep -q "SNAPSHOT .*LOCAL ONLY .* 12 MB of new data is over WIGGUM_PUSH_MAX_MB, not pushed"'

echo "== S2 snapshot: a tiny leftover is pushed as before"
fresh; drive FAKE_OMP_WRITE_MB=0
check "the snapshot reached origin"     '[ "$(git -C "$T/o" for-each-ref refs/wiggum | wc -l | tr -d " ")" -ge 1 ]'
check "log says pushed"                 'log | grep -q "SNAPSHOT .*(pushed)"'

echo "== P12 more than 40 changed files: the scratch-tree guard still wins, nothing parked"
fresh; for i in $(seq 1 45); do echo "$i" > "$T/r/junk$i"; done; drive
check "the pass never started"          '[ ! -e "$T/seen" ]'
check "nothing was parked"              '[ -z "$(git -C "$T/r" branch --list "wiggum/*")" ] && [ "$(git -C "$T/r" status --short | wc -l | tr -d " ")" = 45 ]'

echo "== P13 the fast-forward still works after a park"
fresh; git clone -q "$T/o" "$T/r2"; ( cd "$T/r2" && git config user.email t@t && git config user.name t && echo h > h && git add h && git commit -qm other && git push -q origin main )
echo c >> "$T/r/f"; drive
check "pass saw main, clean"            '[ "$(seen)" = "branch=main dirty=0" ]'
check "local main caught up to origin"  '[ "$(git -C "$T/r" rev-parse main)" = "$(git -C "$T/o" rev-parse main)" ]'
check "log: parked, then fast-forwarded" 'log | grep -q "pre: parked 1 file" && log | grep -q "pre: fast-forwarded 1 commit"'

echo; echo "RESULT: $pass ok, $fail failed"
[ "$fail" = 0 ]
