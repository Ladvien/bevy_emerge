#!/bin/bash
# Tests for the per-repo settings file (<repo>/.omp/wiggum/<command>.env), written 2026-10-28 for Sanctuary's grimoire loop.
# What the file is for: a checkout other people and agents work in must never be parked (git add -A, commit, check main
# out), whatever start line was typed. Case 5 is the control: the same dirty tree WITHOUT the file is parked, so case 4
# proves something; case 11 is the same for the final snapshot (WIGGUM_SNAPSHOT). Cases 8-10: the omp overlay
# (<repo>/.omp/wiggum/<command>.omp.yml) reaches omp as --config, and only when it exists. Throwaway repos, fake omp.
# Usage: bash tools/loop/test_wiggum_settings.sh <path-to-wiggum.sh>   (about 40 seconds; run it under /bin/bash on a Mac)
set -u
DRV="$1"
SH="${BASH:-bash}"                      # the driver runs under the bash that runs this test
T=$(mktemp -d /tmp/wig_settings_test.XXXXXX); pass=0; fail=0
ok()  { pass=$((pass+1)); echo "ok    $*"; }
bad() { fail=$((fail+1)); echo "FAIL  $*"; }
trap 'rm -rf "$T"' EXIT
# the caller's own loop settings must not leak into a test about which settings win
for v in $(env | sed -n 's/^\(WIGGUM_[A-Z_]*\)=.*/\1/p'); do unset "$v"; done
# nor the caller's build directory: a refused pass prunes it, and this Mac's real one is shared by every project
unset CARGO_TARGET_DIR

mkdir -p "$T/bin"
cat > "$T/bin/omp" <<EOF
#!/bin/bash
echo ran > "$T/omp_ran"
printf '%s\n' "\$@" > "$T/omp_argv"
exit 0
EOF
chmod +x "$T/bin/omp"
export PATH="$T/bin:$PATH"
export WIGGUM_SLEEP=1 WIGGUM_IDLE_MAX=0 WIGGUM_COMMAND=emerge-loop

# mkrepo <name> <yes|no>: a clone on main with an origin, optionally carrying a settings file, then three files
# somebody else is working on: one modified, two untracked (one of them a binary-ish .blend).
mkrepo() {
  local r="$T/$1"
  git init -q "$T/$1.origin.git" --bare -b main
  git init -q "$r" -b main
  ( cd "$r" && git config user.email t@t && git config user.name t && echo hi > a.txt && echo seed > b.txt )
  if [ "$2" = yes ]; then
    mkdir -p "$r/.omp/wiggum"
    printf '%s\n' '# a checkout other people work in' 'WIGGUM_PARK=0' 'WIGGUM_CHECKPOINT=0' 'WIGGUM_SNAPSHOT=0' 'WIGGUM_MAX_DIRTY=100' 'WIGGUM_MIN_FREE_GB=0' 'WIGGUM_MODEL=ollama-cloud/gpt-oss:20b' > "$r/.omp/wiggum/emerge-loop.env"
  fi
  ( cd "$r" && git add -A && git commit -q -m init && git remote add origin "$T/$1.origin.git" && git push -q origin main \
    && echo "an agent's edit" >> b.txt && echo blend > Abbey.blend && mkdir -p tools && echo py > tools/x.py )
}
sums() { ( cd "$1" && cat b.txt Abbey.blend tools/x.py 2>/dev/null | cksum ); }
drv() { local r="$1"; shift; WIGGUM_REPO="$T/$r" WIGGUM_STATE="$T/state_$r" "$SH" "$DRV" "$@"; }

echo "this machine: bash=${BASH_VERSION}"

# 1. the file's values are what `settings` reports, and it exits 0
mkrepo shared yes
out=$(drv shared settings 2>&1); rc=$?
echo "$out" | sed -n '1,5p'
[ $rc -eq 0 ] && echo "$out" | grep -q 'WIGGUM_PARK=0   (from the file)' && echo "$out" | grep -q 'WIGGUM_MAX_DIRTY=100   (from the file)' \
  && ok "settings lists the file's PARK=0 and MAX_DIRTY=100 as coming from the file (rc 0)" || bad "settings did not report the file's values (rc $rc)"

# 2. the environment wins over the file, and says so
out=$(WIGGUM_PARK=1 drv shared settings 2>&1)
echo "$out" | grep -q "WIGGUM_PARK=1   (the environment overrides the file's 0)" && ok "WIGGUM_PARK=1 in the environment overrides the file's 0, and settings says so" || bad "environment did not win: $out"

# 3. a line the driver cannot use is named, settings exits 1, and `one` refuses with exit 2 without running a pass
mkrepo bad yes
printf '%s\n' 'WIGGUM_PARK = 0' 'WIGGUM_REPO=/tmp/elsewhere' 'PATH=/evil' "WIGGUM_MAX_DIRTY=\$(touch $T/PWNED)" 'WIGGUM_CHECKPOINT=0' 'WIGGUM_CHECKPOINT=5' > "$T/bad/.omp/wiggum/emerge-loop.env"
out=$(drv bad settings 2>&1); rc=$?
[ $rc -eq 1 ] && ok "settings exits 1 on a file with unusable lines" || bad "settings rc $rc on a bad file"
for want in "line 1: 'WIGGUM_PARK '" "line 2: 'WIGGUM_REPO'" "line 3: 'PATH'" "line 4: WIGGUM_MAX_DIRTY has an empty value or characters" "line 6: WIGGUM_CHECKPOINT is set twice"; do
  echo "$out" | grep -q -F "$want" && ok "rejected and named: $want" || bad "not rejected or not named: $want"
done
rm -f "$T/omp_ran"
out=$(drv bad one 2>&1); rc=$?
[ $rc -eq 2 ] && [ ! -e "$T/omp_ran" ] && echo "$out" | grep -q 'refusing to run' && ok "one refuses a bad settings file (rc 2) and starts no pass" || bad "one on a bad file: rc $rc, omp ran: $([ -e "$T/omp_ran" ] && echo yes || echo no)"
[ ! -e "$T/PWNED" ] && ok "a value with \$(...) in it ran nothing" || bad "a settings value was executed"

# 4. THE POINT: a plain `one` (no flags) on a dirty shared checkout leaves it exactly as it found it
rm -f "$T/omp_ran"; before=$(sums "$T/shared")
out=$(drv shared one 2>&1); rc=$?
after=$(sums "$T/shared")
[ -e "$T/omp_ran" ] && ok "the pass ran (the file's MIN_FREE_GB=0 got it past the disk gate with no flag typed)" || { bad "no pass ran: $(echo "$out" | head -2)"; }
[ "$(git -C "$T/shared" symbolic-ref --short HEAD)" = main ] && ok "still on main" || bad "moved to $(git -C "$T/shared" symbolic-ref --short HEAD)"
[ -z "$(git -C "$T/shared" branch --list 'wiggum/*')" ] && [ -z "$(git -C "$T/shared" for-each-ref refs/wiggum)" ] && ok "no parked branch and no snapshot ref" || bad "driver wrote branches or refs: $(git -C "$T/shared" branch --list 'wiggum/*')"
[ "$before" = "$after" ] && [ -e "$T/shared/Abbey.blend" ] && [ -e "$T/shared/tools/x.py" ] && ok "the three files other agents were editing are on disk, byte for byte" || bad "the dirty files changed or vanished"
[ "$(git -C "$T/shared" log --oneline | wc -l | tr -d ' ')" = 1 ] && ok "no commit was made in the shared checkout" || bad "the driver committed in the shared checkout"

# 5. CONTROL: the same dirty tree with no settings file and the same env is parked, which is what case 4 prevents
mkrepo control no
rm -f "$T/omp_ran"
out=$(WIGGUM_MIN_FREE_GB=0 drv control one 2>&1)
if [ -n "$(git -C "$T/control" branch --list 'wiggum/parked-pass-0-*')" ] && [ ! -e "$T/control/Abbey.blend" ]; then
  ok "control: with no settings file the driver parked the dirty files on a branch and they left the working tree (the hazard is real)"
else
  bad "control did not park: the test in case 4 proves nothing. branches: $(git -C "$T/control" branch --list | tr '\n' ' ')"
fi

# 6. the environment beats the file all the way through: a huge MIN_FREE_GB in the environment refuses the pass
mkrepo envwins yes
rm -f "$T/omp_ran"
out=$(WIGGUM_MIN_FREE_GB=100000 drv envwins one 2>&1)
[ ! -e "$T/omp_ran" ] && echo "$out" | grep -q 'ABORTED: only' && ok "WIGGUM_MIN_FREE_GB=100000 in the environment refused the pass although the file says 0" || bad "environment did not beat the file: $(echo "$out" | head -2)"
[ "$(sums "$T/envwins")" = "$(sums "$T/shared")" ] && ok "and the refused pass still left the dirty files alone" || bad "dirty files changed on a refused pass"

# 6b. a refused pass prunes the build directory cargo actually uses: $CARGO_TARGET_DIR when it is set (this Mac's
# loops build into ~/.cargo-target-shared/global; on 2026-10-28 the driver pruned <repo>/target, freed nothing and
# stopped Sanctuary's loop at 14 GB free). The build itself (deps/*.rlib) and a young linker temp file stay.
mkrepo pruned yes
ctd="$T/ctd"; mkdir -p "$ctd/debug/incremental/crate-x" "$ctd/debug/deps"
echo x > "$ctd/debug/incremental/crate-x/q.bin"; echo rlib > "$ctd/debug/deps/libkeep.rlib"
echo stale > "$ctd/debug/deps/libgone.tmp1"; touch -t 202601010000 "$ctd/debug/deps/libgone.tmp1"
echo live > "$ctd/debug/deps/liblive.tmp2"
out=$(CARGO_TARGET_DIR="$ctd" WIGGUM_MIN_FREE_GB=100000 drv pruned one 2>&1)
[ ! -e "$ctd/debug/incremental" ] && [ ! -e "$ctd/debug/deps/libgone.tmp1" ] \
  && ok "a refused pass pruned \$CARGO_TARGET_DIR: incremental/ and the stale linker temp file are gone" \
  || bad "\$CARGO_TARGET_DIR was not pruned: $(find "$ctd" -type f | tr '\n' ' ')"
[ -e "$ctd/debug/deps/libkeep.rlib" ] && [ -e "$ctd/debug/deps/liblive.tmp2" ] \
  && ok "and kept the build (deps/libkeep.rlib) and a linker temp file under 20 minutes old" \
  || bad "pruning removed the build or a live temp file: $(find "$ctd" -type f | tr '\n' ' ')"

# 7. no file at all: settings says so and exits 0; the driver's own defaults apply
mkrepo plain no
out=$(drv plain settings 2>&1); rc=$?
[ $rc -eq 0 ] && echo "$out" | grep -q '(none: ' && ok "no settings file: reported as none, exit 0" || bad "no-file settings: rc $rc, $out"

# 8. an omp overlay beside the settings file is passed to omp as --config, and the file's model as --model
mkrepo overlay yes
printf '%s\n' 'retry:' '  maxRetries: 2' > "$T/overlay/.omp/wiggum/emerge-loop.omp.yml"
rm -f "$T/omp_ran" "$T/omp_argv"
out=$(drv overlay one 2>&1)
argv=$(cat "$T/omp_argv" 2>/dev/null)
echo "$argv" | grep -A1 -x -e '--config' | sed -n 2p | grep -q -x -F "$T/overlay/.omp/wiggum/emerge-loop.omp.yml" \
  && ok "the overlay reached omp as --config <repo>/.omp/wiggum/emerge-loop.omp.yml" || bad "no --config <overlay> in omp's argv: $(echo "$argv" | tr '\n' ' ')"
echo "$argv" | grep -A1 -x -e '--model' | sed -n 2p | grep -q -x -F 'ollama-cloud/gpt-oss:20b' \
  && ok "the file's WIGGUM_MODEL reached omp as --model ollama-cloud/gpt-oss:20b" || bad "--model was not the file's: $(echo "$argv" | tr '\n' ' ')"

# 9. no overlay file: no --config at all (omp hard-errors on a missing overlay)
rm -f "$T/omp_ran" "$T/omp_argv"
out=$(drv shared one 2>&1)
[ -e "$T/omp_argv" ] && ! grep -q -x -e '--config' "$T/omp_argv" && ok "no overlay file: omp got no --config" || bad "no overlay file but argv: $(tr '\n' ' ' < "$T/omp_argv" 2>/dev/null)"

# 10. settings names the overlay, or says there is none
out=$(drv overlay settings 2>&1)
echo "$out" | grep -q -F 'omp overlay: .omp/wiggum/emerge-loop.omp.yml (passed to omp with --config)' && ok "settings names the overlay" || bad "settings did not name the overlay: $out"
out=$(drv shared settings 2>&1)
echo "$out" | grep -q -x 'omp overlay: none' && ok "settings says 'omp overlay: none' without one" || bad "settings without an overlay: $out"

# 11. CONTROL for case 4: PARK=0 and CHECKPOINT=0 without SNAPSHOT=0 still write a final snapshot of a dirty tree
mkrepo snapctl no
out=$(WIGGUM_PARK=0 WIGGUM_CHECKPOINT=0 WIGGUM_MAX_DIRTY=100 WIGGUM_MIN_FREE_GB=0 drv snapctl one 2>&1)
[ -n "$(git -C "$T/snapctl" for-each-ref 'refs/wiggum/pass-*-final-*')" ] \
  && ok "control: without WIGGUM_SNAPSHOT=0 a dirty tree gets a refs/wiggum final snapshot (case 4's no-ref check is not vacuous)" \
  || bad "control wrote no final snapshot: case 4 proves nothing. refs: $(git -C "$T/snapctl" for-each-ref refs/wiggum | tr '\n' ' ')"

echo; echo "== $pass passed, $fail failed =="
[ "$fail" -eq 0 ]
