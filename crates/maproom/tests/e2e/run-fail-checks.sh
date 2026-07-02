#!/bin/sh
# Re-runs the canonical probes for the 21 CLI E2E regressions (R01-R21) against
# a fresh dev build and fails on any mismatch with the post-fix expectations.
#
# Provenance: tests/fixtures/e2e-regressions.json (vendored fail_checks from the
# 2026-07-02 E2E sweep; each check names its probe here via the "probe" field).
# Spec: _SPECS/crewchief/research/maproom-cli-e2e-fix-spec.md §8.5.
#
# Usage: run from the REPO ROOT:  crates/maproom/tests/e2e/run-fail-checks.sh
#   - Requires: git, sqlite3, jq, timeout. Builds target/debug/maproom if absent.
#   - PG-gated probes (R09_PG, R10_PG) run only when MAPROOM_TEST_PG_URL is set;
#     when unset they are reported loudly as SKIPPED-PG (the milestone gate in
#     the fix spec §8.5 separately REQUIRES the URL to be set).
#   - R14/R15 are guarded by their designated cargo tests (spec §5.5/§5.6 DoD).
# POSIX sh: no [[ ]], no arrays, no $RANDOM.

set -u

BIN=target/debug/maproom
FAILURES=0
PASS=0
SKIPPED_PG=0

say()  { printf '%s\n' "$*"; }
pass() { PASS=$((PASS + 1)); say "PASS  $1"; }
fail() { FAILURES=$((FAILURES + 1)); say "FAIL  $1  -- $2"; }

[ -x "$BIN" ] || cargo build -p maproom || exit 1
command -v sqlite3 >/dev/null || { say "sqlite3 required"; exit 1; }
command -v jq >/dev/null || { say "jq required"; exit 1; }

# fx fixture: $1 = workdir (created); DB at $1/w.db, repo at $1/fx
make_fx() {
  ( cd "$1" && git init -q fx && cd fx \
    && printf 'export function alphaOne() { return 1; }\n' > a.ts \
    && git add a.ts && git -c user.email=t@t -c user.name=t commit -qm init )
  MAPROOM_DATABASE_URL="sqlite://$1/w.db" "$PWD/$BIN" scan --repo fx --path "$1/fx" >/dev/null 2>&1
}

# ---------- R01: cache warm reachable (no clap panic) ----------
if "$BIN" cache warm --help >/dev/null 2>&1; then pass R01; else fail R01 "cache warm --help exited $?"; fi

# ---------- R12: empty --database-url rejected with exit 2 ----------
"$BIN" --database-url "" status >/dev/null 2>&1; c=$?
if [ "$c" -eq 2 ]; then pass "R12(flag)"; else fail "R12(flag)" "exit=$c want 2"; fi
MAPROOM_DATABASE_URL="" "$BIN" status >/dev/null 2>&1; c=$?
if [ "$c" -eq 2 ]; then pass "R12(env)"; else fail "R12(env)" "exit=$c want 2"; fi

# ---------- R11: json search carries total_estimate == total_matches ----------
w=$(mktemp -d); make_fx "$w"
if MAPROOM_DATABASE_URL="sqlite://$w/w.db" "$BIN" search --repo fx --query alphaOne --format json \
  | jq -e '(.total_estimate != null) and (.total_matches != null) and (.total_estimate == .total_matches)' >/dev/null 2>&1
then pass R11; else fail R11 "total_estimate/total_matches mismatch or missing"; fi
rm -rf "$w"

# ---------- R09 (SQLite): no per-commit accumulation; deleted symbol unsearchable ----------
w=$(mktemp -d); make_fx "$w"
n1=$(sqlite3 "$w/w.db" "SELECT count(*) FROM chunks;")
( cd "$w/fx" \
  && printf 'export function gammaThree() { return 3; }\n' > a.ts \
  && git add a.ts && git -c user.email=t@t -c user.name=t commit -qm edit )
MAPROOM_DATABASE_URL="sqlite://$w/w.db" "$BIN" scan --repo fx --path "$w/fx" --force >/dev/null 2>&1
MAPROOM_DATABASE_URL="sqlite://$w/w.db" "$BIN" scan --repo fx --path "$w/fx" --force >/dev/null 2>&1
n2=$(sqlite3 "$w/w.db" "SELECT count(*) FROM chunks;")
if [ "$n2" -le $((n1 + 1)) ]; then pass "R09(count)"; else fail "R09(count)" "chunks $n1 -> $n2 (accumulation)"; fi
if MAPROOM_DATABASE_URL="sqlite://$w/w.db" "$BIN" search --repo fx --query alphaOne --format json \
  | jq -e '.hits | length == 0' >/dev/null 2>&1
then pass "R09(stale-gone)"; else fail "R09(stale-gone)" "replaced symbol alphaOne still searchable"; fi
rm -rf "$w"

# ---------- R06-R08: watch indexes an uncommitted edit; scan not poisoned ----------
w=$(mktemp -d); make_fx "$w"
printf 'export function betaTwo() { return 2; }\n' >> "$w/fx/a.ts"    # UNCOMMITTED
( MAPROOM_DATABASE_URL="sqlite://$w/w.db" timeout 15 "$PWD/$BIN" watch --repo fx --path "$w/fx" --json >/dev/null 2>&1 || true )
if MAPROOM_DATABASE_URL="sqlite://$w/w.db" "$BIN" search --repo fx --query betaTwo --format agent 2>/dev/null | grep -q betaTwo
then pass "R06-08(watch)"; else fail "R06-08(watch)" "uncommitted betaTwo not searchable after watch"; fi
# poison probe: a plain incremental scan after watch must still index new commits
( cd "$w/fx" && git add a.ts && git -c user.email=t@t -c user.name=t commit -qm beta \
  && printf 'export function deltaFour() { return 4; }\n' >> a.ts \
  && git add a.ts && git -c user.email=t@t -c user.name=t commit -qm delta )
MAPROOM_DATABASE_URL="sqlite://$w/w.db" "$BIN" scan --repo fx --path "$w/fx" >/dev/null 2>&1
if MAPROOM_DATABASE_URL="sqlite://$w/w.db" "$BIN" search --repo fx --query deltaFour --format agent 2>/dev/null | grep -q deltaFour
then pass "R06-08(no-poison)"; else fail "R06-08(no-poison)" "post-watch incremental scan served a stale index"; fi
rm -rf "$w"

# ---------- R05: cleanup-stale removes registration and converges ----------
w=$(mktemp -d)
( cd "$w" && git init -q doomed && cd doomed && printf 'x\n' > f.txt \
  && git add f.txt && git -c user.email=t@t -c user.name=t commit -qm i )
MAPROOM_DATABASE_URL="sqlite://$w/c.db" "$BIN" scan --repo doomed --path "$w/doomed" >/dev/null 2>&1
rm -rf "$w/doomed"
MAPROOM_DATABASE_URL="sqlite://$w/c.db" "$BIN" db cleanup-stale --confirm >/dev/null 2>&1
left=$(sqlite3 "$w/c.db" "SELECT count(*) FROM worktrees;")
if [ "$left" -eq 0 ]; then pass R05; else fail R05 "worktrees rows left: $left"; fi
rm -rf "$w"

# ---------- R04: migrate markdown pre-flight (no orphan backup) ----------
w=$(mktemp -d)
MAPROOM_DATABASE_URL="sqlite://$w/md.db" "$BIN" db migrate >/dev/null 2>&1
MAPROOM_DATABASE_URL="sqlite://$w/md.db" "$BIN" migrate markdown --repo any >"$w/out" 2>"$w/err"; c=$?
ok=1
[ "$c" -ne 0 ] || ok=0
grep -q 'file_contents' "$w/err" || ok=0
[ "$(sqlite3 "$w/md.db" "SELECT count(*) FROM sqlite_master WHERE name LIKE 'chunks_backup_%';")" -eq 0 ] || ok=0
if [ "$ok" -eq 1 ]; then pass R04; else fail R04 "exit=$c; err/backup state wrong"; fi
rm -rf "$w"

# ---------- R02: delete-backup validation ----------
w=$(mktemp -d)
MAPROOM_DATABASE_URL="sqlite://$w/p.db" "$BIN" db migrate >/dev/null 2>&1
MAPROOM_DATABASE_URL="sqlite://$w/p.db" "$BIN" migrate delete-backup --backup chunks >/dev/null 2>"$w/e1"; c1=$?
have=$(sqlite3 "$w/p.db" "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='chunks';")
MAPROOM_DATABASE_URL="sqlite://$w/p.db" "$BIN" migrate delete-backup --backup chunks_backup_19990101_000000 >/dev/null 2>"$w/e2"; c2=$?
ok=1
[ "$c1" -ne 0 ] || ok=0
[ "$have" -eq 1 ] || ok=0
[ "$c2" -ne 0 ] || ok=0
grep -q 'does not exist' "$w/e2" || ok=0
if [ "$ok" -eq 1 ]; then pass R02; else fail R02 "c1=$c1 c2=$c2 chunks-table=$have"; fi
rm -rf "$w"

# ---------- R03: db migrate detects structural damage ----------
w=$(mktemp -d)
MAPROOM_DATABASE_URL="sqlite://$w/d.db" "$BIN" db migrate >/dev/null 2>&1
sqlite3 "$w/d.db" "DROP TABLE chunks;"
out=$(MAPROOM_DATABASE_URL="sqlite://$w/d.db" "$BIN" db migrate 2>&1); c=$?
if [ "$c" -eq 1 ] && printf '%s' "$out" | grep -q chunks; then pass R03; else fail R03 "exit=$c out=$(printf '%s' "$out" | head -1)"; fi
rm -rf "$w"

# ---------- R13: config error exits 2 regardless of format ----------
w=$(mktemp -d)
MAPROOM_EMBEDDING_PROVIDER=invalid MAPROOM_DATABASE_URL="sqlite://$w/x.db" \
  "$BIN" vector-search --repo x --query y >/dev/null 2>&1; c1=$?
MAPROOM_EMBEDDING_PROVIDER=invalid MAPROOM_DATABASE_URL="sqlite://$w/x.db" \
  "$BIN" vector-search --repo x --query y --format agent >/dev/null 2>&1; c2=$?
if [ "$c1" -eq 2 ] && [ "$c2" -eq 2 ]; then pass R13; else fail R13 "json=$c1 agent=$c2 want 2/2"; fi
rm -rf "$w"

# ---------- R16: serve starts without a working provider ----------
w=$(mktemp -d)
printf '{"jsonrpc":"2.0","method":"ping","id":1}\n' \
  | MAPROOM_DATABASE_URL="sqlite://$w/r16.db" MAPROOM_EMBEDDING_PROVIDER=google GOOGLE_APPLICATION_CREDENTIALS=/nonexistent \
    timeout 30 "$BIN" serve >"$w/out" 2>"$w/err"; c=$?
if [ "$c" -eq 0 ] && grep -q '"result":"pong"' "$w/out"; then pass "R16(stdio)"; else fail "R16(stdio)" "exit=$c pong=$(grep -c pong "$w/out" 2>/dev/null)"; fi
timeout 60 env MAPROOM_DATABASE_URL="sqlite://$w/r16.db" MAPROOM_EMBEDDING_PROVIDER=google GOOGLE_APPLICATION_CREDENTIALS=/nonexistent \
  "$BIN" serve --socket --socket-path "$w/r16.sock" --idle-timeout 3 2>"$w/sockerr"; c=$?
ok=1
[ "$c" -eq 0 ] || ok=0
grep -q 'Database error' "$w/sockerr" && ok=0
if [ "$ok" -eq 1 ]; then pass "R16(socket)"; else fail "R16(socket)" "exit=$c (124=hung; mislabel grep=$(grep -c 'Database error' "$w/sockerr" 2>/dev/null))"; fi
rm -rf "$w"

# ---------- R17: stdout purity under RUST_LOG=info ----------
w=$(mktemp -d)
printf '{"jsonrpc":"2.0","method":"ping","id":1}\n' \
  | RUST_LOG=info MAPROOM_DATABASE_URL="sqlite://$w/x.db" timeout 30 "$BIN" serve >"$w/out" 2>"$w/err"
ok=1
grep -q '"result":"pong"' "$w/out" || ok=0
[ "$(grep -cv '^{' "$w/out")" -eq 0 ] || ok=0
test -s "$w/err" || ok=0
grep -q "$(printf '\033')" "$w/out" && ok=0
grep -q "$(printf '\033')" "$w/err" && ok=0
if [ "$ok" -eq 1 ]; then pass R17; else fail R17 "stdout impure or logs missing from stderr"; fi
rm -rf "$w"

# ---------- R18: unknown worktree -> -32602 ----------
w=$(mktemp -d); make_fx "$w"
if printf '{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","query":"alphaOne","worktree":"no-such-wt"},"id":7}\n' \
  | MAPROOM_DATABASE_URL="sqlite://$w/w.db" timeout 30 "$BIN" serve 2>/dev/null | grep -qF -- '-32602'
then pass R18; else fail R18 "unknown worktree not rejected with -32602"; fi
rm -rf "$w"

# ---------- R19: JSON-RPC conformance ----------
w=$(mktemp -d)
printf '{"jsonrpc":"2.0","method":"ping"}\n' | MAPROOM_DATABASE_URL="sqlite://$w/x.db" timeout 30 "$BIN" serve 2>/dev/null >"$w/o1"
n1=$(wc -l < "$w/o1")
o2=$(printf '{"jsonrpc":"2.0","method":"ping","id":null}\n' | MAPROOM_DATABASE_URL="sqlite://$w/x.db" timeout 30 "$BIN" serve 2>/dev/null)
o3=$(printf '{"jsonrpc":"1.0","method":"ping","id":23}\n' | MAPROOM_DATABASE_URL="sqlite://$w/x.db" timeout 30 "$BIN" serve 2>/dev/null)
o4=$(printf '[{"jsonrpc":"2.0","method":"ping","id":1}]\n' | MAPROOM_DATABASE_URL="sqlite://$w/x.db" timeout 30 "$BIN" serve 2>/dev/null)
ok=1
[ "$n1" -eq 0 ] || ok=0
printf '%s' "$o2" | grep -q '"result":"pong"' || ok=0
printf '%s' "$o3" | grep -qF -- '-32600' || ok=0
printf '%s' "$o4" | grep -qF 'Batch requests are not supported' || ok=0
if [ "$ok" -eq 1 ]; then pass R19; else fail R19 "notif-lines=$n1 null-id/version/batch handling wrong"; fi
rm -rf "$w"

# ---------- R20: idle-timeout 5 exits promptly ----------
w=$(mktemp -d)
start=$(date +%s)
timeout 30 "$BIN" serve --socket --socket-path "$w/r20.sock" --idle-timeout 5 \
  >/dev/null 2>&1; c=$?
end=$(date +%s)
if [ "$c" -eq 0 ] && [ $((end - start)) -le 15 ]; then pass R20; else fail R20 "exit=$c elapsed=$((end - start))s want <=15"; fi
rm -rf "$w"

# ---------- R21: socket unlinked on SIGTERM ----------
w=$(mktemp -d)
"$BIN" serve --socket --socket-path "$w/r21.sock" --idle-timeout 300 >/dev/null 2>&1 &
pid=$!
i=0; while [ $i -lt 100 ] && test ! -S "$w/r21.sock"; do sleep 0.1; i=$((i+1)); done
if test -S "$w/r21.sock"; then
  kill -TERM $pid 2>/dev/null; wait $pid 2>/dev/null
  if test ! -e "$w/r21.sock"; then pass R21; else fail R21 "socket file survives SIGTERM"; fi
else
  kill $pid 2>/dev/null; fail R21 "socket never appeared"
fi
rm -rf "$w"

# ---------- R14 / R15: designated cargo-test guards (spec §5.5/§5.6 DoD) ----------
if cargo test -q -p maproom --lib embedding::factory -- --test-threads=1 >/dev/null 2>&1
then pass "R14(cargo)"; else fail "R14(cargo)" "embedding::factory unit tests red"; fi
if cargo test -q -p maproom --test embedding_service_test -- --test-threads=1 >/dev/null 2>&1
then pass "R15(cargo)"; else fail "R15(cargo)" "embedding_service_test red"; fi

# ---------- PG-gated probes ----------
if [ -n "${MAPROOM_TEST_PG_URL:-}" ]; then
  PGBIN=target/debug/maproom   # requires a --features postgres build for PG probes
  cargo build -q -p maproom --features postgres >/dev/null 2>&1 || true
  # R10: PG search emits preview
  w=$(mktemp -d)
  ( cd "$w" && git init -q fx && cd fx \
    && printf 'export function alphaOne() { return 1; }\n' > a.ts \
    && git add a.ts && git -c user.email=t@t -c user.name=t commit -qm init )
  PGDB="${MAPROOM_TEST_PG_URL%/*}/maproom_e2e_runner"
  MAPROOM_DATABASE_URL="$PGDB" "$PGBIN" db migrate >/dev/null 2>&1
  MAPROOM_DATABASE_URL="$PGDB" "$PGBIN" scan --repo fxr --path "$w/fx" >/dev/null 2>&1
  if MAPROOM_DATABASE_URL="$PGDB" "$PGBIN" search --repo fxr --query alphaOne --format json \
    | jq -e '.hits[0].preview != null' >/dev/null 2>&1
  then pass "R10(pg)"; else fail "R10(pg)" "PG search json lacks preview"; fi
  # R09 PG: no accumulation
  n1=$(MAPROOM_DATABASE_URL="$PGDB" "$PGBIN" status --format json 2>/dev/null | jq -r '.. | numbers' | head -1)
  ( cd "$w/fx" && printf 'export function gammaThree() { return 3; }\n' > a.ts \
    && git add a.ts && git -c user.email=t@t -c user.name=t commit -qm edit )
  MAPROOM_DATABASE_URL="$PGDB" "$PGBIN" scan --repo fxr --path "$w/fx" --force >/dev/null 2>&1
  MAPROOM_DATABASE_URL="$PGDB" "$PGBIN" scan --repo fxr --path "$w/fx" --force >/dev/null 2>&1
  if MAPROOM_DATABASE_URL="$PGDB" "$PGBIN" search --repo fxr --query alphaOne --format json \
    | jq -e '.hits | length == 0' >/dev/null 2>&1
  then pass "R09(pg-stale-gone)"; else fail "R09(pg-stale-gone)" "replaced symbol still searchable on PG"; fi
  rm -rf "$w"
else
  SKIPPED_PG=1
  say "SKIPPED-PG: MAPROOM_TEST_PG_URL unset — R10(pg) and R09(pg) probes not run (milestone gate requires them; see fix spec §8.5)"
fi

say ""
say "=== run-fail-checks: $PASS passed, $FAILURES failed$( [ $SKIPPED_PG -eq 1 ] && printf ', PG probes SKIPPED' )"
[ "$FAILURES" -eq 0 ] || exit 1
exit 0
