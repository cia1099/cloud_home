#!/usr/bin/env bash
# R-031: OTA update end to end against a local, throwaway-key-signed mirror.
#
#   scripts/e2e_update.sh          (about 4 minutes: three builds + the 2 minute commit window)
#
# The product trusts only the minisign public key compiled into the binary, so the script builds
# from a patched COPY of the repo (PUBLIC_KEY replaced by a throwaway key; the real source is never
# touched). Three builds: A (installed), B (good update, 7.7.7), C (7.7.8, patched to wreck the
# database and then crash after boot). A tiny supervisor loop plays systemd's Restart=always.
#
# Scenarios: tampered manifest, tampered package, wrong signing key, happy path (+ commit),
# crash -> rollback with database restore, no reinstall loop.
#
# NOT covered yet (R-026, the UI is not embedded in the binary): that GET / serves the new UI after the
# update and the old UI after a rollback, and that --self-test refuses a release with a broken UI.
# Env: E2E_TARGET_DIR = cargo target dir to build in (default <repo>/target).
set -uo pipefail
# shellcheck source=scripts/e2e_lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/e2e_lib.sh"

for c in cargo minisign python3 curl sqlite3 jq rsync rustc; do
  command -v "$c" >/dev/null || die "missing command: $c"
done

WORK="$(mktemp -d)"
export CARGO_TARGET_DIR="${E2E_TARGET_DIR:-$REPO/target}"
SUP_PID=""
MIRROR_PID=""
cleanup() {
  [ -n "$SUP_PID" ] && kill "$SUP_PID" 2>/dev/null
  pkill -f "$WORK/root/current/cloud_home" 2>/dev/null
  [ -n "$MIRROR_PID" ] && kill "$MIRROR_PID" 2>/dev/null
  wait 2>/dev/null
  rm -rf "$WORK"
}
trap cleanup EXIT

VER_A="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$REPO/Cargo.toml" | head -1)"
VER_B=7.7.7
VER_C=7.7.8
TARGET="$(rustc -vV | sed -n 's/^host: //p')"
ROOT="$WORK/root"
MIRROR="$WORK/mirror"
ADMIN_PW=e2e-admin-pw
mkdir -p "$ROOT/versions" "$MIRROR" "$WORK/drive" "$WORK/logs"

echo "== setup: keys, patched build copy, three builds (A=$VER_A B=$VER_B C=$VER_C, target $TARGET)"
gen_key "$WORK/key-good"
gen_key "$WORK/key-other"
PUB="$(cat "$WORK/key-good/pubkey.txt")"

SRC="$WORK/src"
rsync -a --exclude target --exclude .git --exclude ui --exclude manager-site --exclude design \
  --exclude reports --exclude logs --exclude .claude --exclude memory "$REPO/" "$SRC/"
PUB="$PUB" python3 -I - "$SRC/src/update/mod.rs" <<'PY' || die "could not patch PUBLIC_KEY in the copy"
import os, re, sys
p = sys.argv[1]
s = open(p).read()
s, n = re.subn(r'(pub const PUBLIC_KEY: &str = ")[^"]+(")', lambda m: m.group(1) + os.environ["PUB"] + m.group(2), s)
assert n == 1
open(p, "w").write(s)
PY

build() { # build <version> <dest>
  export V="$1"
  perl -i -pe 'if (!$done && s/^version = ".*"/version = "$ENV{V}"/) { $done = 1 }' "$SRC/Cargo.toml"
  (cd "$SRC" && cargo build --quiet) >"$WORK/build.log" 2>&1 || { tail -30 "$WORK/build.log"; die "build of $1 failed"; }
  cp "$CARGO_TARGET_DIR/debug/cloud_home" "$2"
  [ "$("$2" --self-test | cut -d' ' -f2)" = "$1" ] || die "built binary reports the wrong version"
}
build "$VER_A" "$WORK/bin-a"
build "$VER_B" "$WORK/bin-b"
python3 -I - "$SRC/src/main.rs" <<'PY' || die "could not patch the crash build"
import sys
p = sys.argv[1]
s = open(p).read()
anchor = 'db::run_migrations(&pool).await.context("运行迁移失败")?;\n'
assert s.count(anchor) == 1
crash = anchor + '''    sqlx::query("CREATE TABLE IF NOT EXISTS e2e_crash_marker (x INTEGER)").execute(&pool).await?;
    sqlx::query("DELETE FROM users").execute(&pool).await?;
    anyhow::bail!("e2e: simulated crash after the database was changed");
'''
open(p, "w").write(s.replace(anchor, crash))
PY
build "$VER_C" "$WORK/bin-c"


make_pkg "$WORK/dist-b" "$VER_B" "$TARGET" "$WORK/bin-b"
make_pkg "$WORK/dist-c" "$VER_C" "$TARGET" "$WORK/bin-c"

start_mirror "$MIRROR" "$WORK/mirror.log"
MIRROR_URL="http://127.0.0.1:$MIRROR_PORT"

echo "== install A like packaging/*/install.sh lays it out, start it under a restart-always supervisor"
mkdir -p "$ROOT/versions/$VER_A" "$ROOT/update"
cp "$WORK/bin-a" "$ROOT/versions/$VER_A/cloud_home"
ln -sfn "versions/$VER_A" "$ROOT/current"
APP_PORT="$(free_port)"
API="http://127.0.0.1:$APP_PORT/api/v1"
cat > "$WORK/supervise.sh" <<SUP
#!/usr/bin/env bash
cd "$ROOT" || exit 1
export UPDATE_ROOT="$ROOT" UPDATE_BASE_URL="$MIRROR_URL" DEV_DRIVE_PATH="$WORK/drive" LOG_DIR="$WORK/logs"
export JWT_SECRET=e2e-throwaway-secret SERVER_PORT=$APP_PORT SERVER_HOST=127.0.0.1 ADMIN_PASSWORD=$ADMIN_PW
export REGISTRATION_MODE=open LOG_FILE_ENABLED=false VOLUMES_WATCH_PATH="$WORK/no-volumes"
while :; do
  "$ROOT/current/cloud_home"
  sleep 1
done
SUP
chmod +x "$WORK/supervise.sh"
"$WORK/supervise.sh" >"$WORK/server.log" 2>&1 &
SUP_PID=$!

running_version() { curl -s --max-time 2 "$API/health" | jq -r '.version // empty' 2>/dev/null; }
wait_until() { # wait_until <seconds> <command...>
  local n="$1" i
  shift
  for i in $(seq $((n * 2))); do
    "$@" && return 0
    sleep 0.5
  done
  return 1
}
version_is() { [ "$(running_version)" = "$1" ]; }
admin() { # admin <method> <path>
  curl -s --max-time 70 -X "$1" -H "X-Admin-Password: $ADMIN_PW" "$API$2"
}
admin_code() { curl -s -o /dev/null -w '%{http_code}' -X "$1" -H "X-Admin-Password: $ADMIN_PW" "$API$2"; }
snap() { admin GET /admin/update | jq -r "$1"; }
link() { readlink "$ROOT/$1"; }
state() { jq -r "($1) // \"none\"" "$ROOT/update/state.json" 2>/dev/null || echo none; }
link_is() { [ "$(link "$1")" = "$2" ]; }
state_is() { [ "$(state "$1")" = "$2" ]; }
phase_is() { [ "$(snap .data.phase)" = "$1" ]; }
db() { sqlite3 "$(find "$WORK/drive" -name cloud_home.db | head -1)" "$1"; }
published_count() { grep -c "GET /cloud_home-$1-" "$WORK/mirror.log"; }

wait_until 60 version_is "$VER_A" || { tail -5 "$WORK/server.log" | cut -c1-300; die "server A did not come up"; }
expect "A runs and the updater is enabled" "$(snap '.data.enabled')" true

# Some data that must survive an update and a rollback.
curl -s -o /dev/null -X POST -H 'Content-Type: application/json' \
  -d '{"username":"owner","email":"owner@example.com","password":"correct-horse-battery"}' "$API/auth/register"
expect "setup: one account exists before any update" "$(db 'SELECT COUNT(*) FROM users')" 1

echo "== scenario: tampered manifest (valid signature broken by editing the manifest)"
publish "$WORK/dist-b" "$VER_B" "$MIRROR" "$WORK/key-good/key.sec" "$MIRROR_URL"
sed -i.bak "s/\"size\": [0-9]*/\"size\": 1/" "$MIRROR/manifest.json" && rm -f "$MIRROR/manifest.json.bak"
admin POST /admin/update/check >/dev/null
expect "tampered manifest: nothing offered" "$(snap '.data.available')" null
case "$(snap '.data.last_error')" in
  *签名校验失败*) ok "tampered manifest: rejected on the signature" ;;
  *) bad "tampered manifest: expected a signature error, got $(snap '.data.last_error')" ;;
esac
expect "tampered manifest: install refused" \
  "$(admin POST /admin/update/install | jq -r '.error.code')" UPDATE_NOT_AVAILABLE

echo "== scenario: wrong signing key"
publish "$WORK/dist-b" "$VER_B" "$MIRROR" "$WORK/key-other/key.sec" "$MIRROR_URL"
admin POST /admin/update/check >/dev/null
expect "wrong key: nothing offered" "$(snap '.data.available')" null
case "$(snap '.data.last_error')" in
  *签名校验失败*) ok "wrong key: rejected on the signature" ;;
  *) bad "wrong key: expected a signature error, got $(snap '.data.last_error')" ;;
esac
expect "wrong key: install refused" \
  "$(admin POST /admin/update/install | jq -r '.error.code')" UPDATE_NOT_AVAILABLE

echo "== scenario: tampered package (signed manifest, package bytes changed, same size)"
publish "$WORK/dist-b" "$VER_B" "$MIRROR" "$WORK/key-good/key.sec" "$MIRROR_URL"
python3 -I - "$MIRROR/cloud_home-$VER_B-$TARGET.tar.gz" <<'PY'
import sys
p = sys.argv[1]
b = bytearray(open(p, "rb").read())
b[len(b) // 2] ^= 0xFF
open(p, "wb").write(b)
PY
admin POST /admin/update/check >/dev/null
expect "tampered package: manifest itself is fine, update offered" "$(snap '.data.available.version')" "$VER_B"
expect "tampered package: install accepted for processing" "$(admin_code POST /admin/update/install)" 202
wait_until 30 phase_is failed
expect "tampered package: updater ends in failed" "$(snap '.data.phase')" failed
case "$(snap '.data.last_error')" in
  *sha256*) ok "tampered package: rejected on sha256" ;;
  *) bad "tampered package: expected sha256 error, got $(snap '.data.last_error')" ;;
esac
expect "tampered package: current still A" "$(link current)" "versions/$VER_A"
expect "tampered package: A still serving" "$(running_version)" "$VER_A"
if [ ! -e "$ROOT/versions/$VER_B" ]; then ok "tampered package: nothing installed"; else bad "tampered package: versions/$VER_B exists"; fi
expect "tampered package: no staged leftovers" "$(find "$ROOT/update/staging" -type f 2>/dev/null | wc -l | tr -d ' ')" 0
expect "tampered package: no pending state" "$(state '.pending')" none

echo "== scenario: happy path (A -> B)"
publish "$WORK/dist-b" "$VER_B" "$MIRROR" "$WORK/key-good/key.sec" "$MIRROR_URL"
admin POST /admin/update/check >/dev/null
expect "happy: B offered" "$(snap '.data.available.version')" "$VER_B"
expect "happy: install accepted" "$(admin_code POST /admin/update/install)" 202
wait_until 60 version_is "$VER_B"
expect "happy: the daemon now runs B" "$(running_version)" "$VER_B"
expect "happy: current -> B" "$(link current)" "versions/$VER_B"
expect "happy: previous -> A" "$(link previous)" "versions/$VER_A"
expect "happy: pending is B until committed" "$(state '.pending')" "$VER_B"
BACKUP="$(state '.backup')"
if [ -s "$BACKUP" ]; then ok "happy: database backup written"; else bad "happy: no database backup at '$BACKUP'"; fi
expect "happy: data survived the update" "$(db 'SELECT COUNT(*) FROM users')" 1
echo "  (waiting for the 2 minute commit window)"
wait_until 150 state_is .pending none
expect "happy: update committed after stable run" "$(state '.pending')" none

echo "== scenario: crash -> rollback with database restore (B -> C, C wipes users then crashes)"
publish "$WORK/dist-c" "$VER_C" "$MIRROR" "$WORK/key-good/key.sec" "$MIRROR_URL"
: > "$WORK/mirror.log"
admin POST /admin/update/check >/dev/null
expect "crash: C offered" "$(snap '.data.available.version')" "$VER_C"
admin POST /admin/update/install >/dev/null
wait_until 30 link_is current "versions/$VER_C"
expect "crash: switched to C" "$(link current)" "versions/$VER_C"
wait_until 60 link_is current "versions/$VER_B"
expect "crash: rolled back, current -> B" "$(link current)" "versions/$VER_B"
wait_until 30 version_is "$VER_B"
expect "crash: B serving again" "$(running_version)" "$VER_B"
expect "crash: failed version recorded" "$(state '.last_failed')" "$VER_C"
expect "crash: pending cleared" "$(state '.pending')" none
expect "crash: database restored (users back)" "$(db 'SELECT COUNT(*) FROM users')" 1
expect "crash: C's schema change undone" \
  "$(db "SELECT COUNT(*) FROM sqlite_master WHERE name='e2e_crash_marker'")" 0
expect "crash: C ran 3 times before the 4th start rolled back" \
  "$(grep -c 'e2e: simulated crash' "$WORK/server.log")" 3

echo "== scenario: no reinstall loop (C is still the newest release on the mirror)"
admin POST /admin/update/check >/dev/null
expect "loop: rolled-back C is not offered again" "$(snap '.data.available')" null
expect "loop: install refused" "$(admin POST /admin/update/install | jq -r '.error.code')" UPDATE_NOT_AVAILABLE
sleep 5
expect "loop: still on B after waiting" "$(running_version)" "$VER_B"
expect "loop: C's package was downloaded exactly once" "$(published_count "$VER_C")" 1

echo "== scenario: no downgrade or replay (mirror re-serves the equal version B)"
publish "$WORK/dist-b" "$VER_B" "$MIRROR" "$WORK/key-good/key.sec" "$MIRROR_URL"
admin POST /admin/update/check >/dev/null
expect "no downgrade: equal version not offered" "$(snap '.data.available')" null

echo "== note: UI embedding assertions (R-026) are not covered yet, see the header of this script"
summary
