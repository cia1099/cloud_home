#!/usr/bin/env bash
# R-031: OTA update end to end against a local, throwaway-key-signed mirror.
#
#   scripts/e2e_update.sh          (several minutes: five release builds + the 2 minute commit window)
#
# The product trusts only the minisign public key compiled into the binary, so the script builds
# from a patched COPY of the repo (PUBLIC_KEY replaced by a throwaway key; the real source is never
# touched). Release builds (the UI is embedded only in release builds, R-026), five of them:
# A (installed), B (good update, 7.7.7), D (7.7.9, embedded UI references a missing asset),
# E (7.7.10, embedded UI version differs from the binary; the copy's build.rs guard is disabled to
# make that possible), C (7.7.8, patched to wreck the database and then crash after boot).
# Every build embeds the real ui/dist (built here with bun when absent) plus a per-release marker
# (index.html comment + assets/e2e-marker-<label>.js), so the UI of A, B and C is visibly different.
# A tiny supervisor loop plays systemd's Restart=always.
#
# Scenarios: tampered manifest, tampered package, wrong signing key, releases with a broken UI
# (refused by --self-test before the switch), happy path (+ commit; GET / serves B's UI), crash ->
# rollback with database restore (GET / serves B's UI again), no reinstall loop.
# Env: E2E_TARGET_DIR = cargo target dir to build in (default <repo>/target).
set -uo pipefail
# shellcheck source=scripts/e2e_lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/e2e_lib.sh"

for c in cargo minisign python3 curl sqlite3 jq rsync rustc bun; do
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
VER_D=7.7.9
VER_E=7.7.10
TARGET="$(rustc -vV | sed -n 's/^host: //p')"
ROOT="$WORK/root"
MIRROR="$WORK/mirror"
ADMIN_PW=e2e-admin-pw
mkdir -p "$ROOT/versions" "$MIRROR" "$WORK/drive" "$WORK/logs"

echo "== setup: keys, UI, patched build copy, five release builds (A=$VER_A B=$VER_B C=$VER_C D=$VER_D E=$VER_E, target $TARGET)"
if [ ! -f "$REPO/ui/dist/index.html" ]; then
  (bun install --cwd "$REPO/ui" --frozen-lockfile && bun run --cwd "$REPO/ui" build) >"$WORK/ui-build.log" 2>&1 \
    || { tail -20 "$WORK/ui-build.log"; die "could not build ui/dist"; }
fi
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

# Fast release profile: the UI is only embedded when debug assertions are off.
export CARGO_PROFILE_RELEASE_OPT_LEVEL=0 CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
# build <version> <dest> <ui-label> [ui-mode]: ui-mode "missing-asset" or "wrong-version" breaks the embedded UI.
build() {
  export V="$1"
  perl -i -pe 'if (!$done && s/^version = ".*"/version = "$ENV{V}"/) { $done = 1 }' "$SRC/Cargo.toml"
  rm -rf "$SRC/ui"
  mkdir -p "$SRC/ui"
  cp -R "$REPO/ui/dist" "$SRC/ui/dist"
  python3 -I - "$SRC/ui/dist" "$1" "$3" "${4:-}" <<'PY2' || die "could not patch the embedded UI"
import json, sys
d, version, label, mode = sys.argv[1:5]
with open(d + "/assets/e2e-marker-" + label + ".js", "w") as f:
    f.write('window.__E2E_UI__="' + label + '";\n')
p = d + "/index.html"
s = open(p).read()
assert s.count("</head>") == 1
extra = '<!-- e2e-ui:' + label + ' --><script src="/assets/e2e-marker-' + label + '.js"></script>'
if mode == "missing-asset":
    extra += '<script src="/assets/e2e-missing.js"></script>'
open(p, "w").write(s.replace("</head>", extra + "</head>"))
json.dump({"version": "0.0.1" if mode == "wrong-version" else version}, open(d + "/version.json", "w"))
PY2
  (cd "$SRC" && cargo build --release --quiet) >"$WORK/build.log" 2>&1 || { tail -30 "$WORK/build.log"; die "build of $1 failed"; }
  cp "$CARGO_TARGET_DIR/release/cloud_home" "$2"
}
build "$VER_A" "$WORK/bin-a" A
build "$VER_B" "$WORK/bin-b" B
build "$VER_D" "$WORK/bin-d" D missing-asset
python3 -I - "$SRC/build.rs" <<'PY' || die "could not patch the build.rs version guard"
import sys
p = sys.argv[1]
s = open(p).read()
anchor = "if baked != version {"
assert s.count(anchor) == 1
open(p, "w").write(s.replace(anchor, "if false {"))
PY
build "$VER_E" "$WORK/bin-e" E wrong-version
cp "$REPO/build.rs" "$SRC/build.rs"
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
build "$VER_C" "$WORK/bin-c" C

expect "binary A reports its version" "$("$WORK/bin-a" --self-test | cut -d' ' -f2)" "$VER_A"
expect "binary B reports its version" "$("$WORK/bin-b" --self-test | cut -d' ' -f2)" "$VER_B"
expect "binary C reports its version" "$("$WORK/bin-c" --self-test | cut -d' ' -f2)" "$VER_C"
echo "== the embedded UI differs per release (marker bytes inside each binary)"
for l in A B C; do
  b="$WORK/bin-$(echo "$l" | tr A-Z a-z)"
  for m in A B C; do
    want=0
    [ "$l" = "$m" ] && want=1
    expect "binary $l: marker $m present=$want" "$(grep -c "e2e-ui:$m" "$b")" "$want"
  done
done
if "$WORK/bin-d" --self-test >/dev/null 2>&1; then bad "self-test: D (index.html references a missing asset) should fail"; else ok "self-test: D (index.html references a missing asset) fails"; fi
if "$WORK/bin-e" --self-test >/dev/null 2>&1; then bad "self-test: E (UI version differs from the binary) should fail"; else ok "self-test: E (UI version differs from the binary) fails"; fi

make_pkg "$WORK/dist-b" "$VER_B" "$TARGET" "$WORK/bin-b"
make_pkg "$WORK/dist-c" "$VER_C" "$TARGET" "$WORK/bin-c"
make_pkg "$WORK/dist-d" "$VER_D" "$TARGET" "$WORK/bin-d"
make_pkg "$WORK/dist-e" "$VER_E" "$TARGET" "$WORK/bin-e"

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
ui_check() { # ui_check <description> <label> <other>: GET / and its assets belong to UI <label>, not <other>
  local d="$1" l="$2" o="$3" root="http://127.0.0.1:$APP_PORT" html js
  html="$(curl -s --max-time 5 "$root/")"
  case "$html" in
    *"e2e-ui:$l"*) ok "$d: GET / serves UI $l" ;;
    *) bad "$d: GET / does not carry marker $l" ;;
  esac
  case "$html" in
    *"e2e-ui:$o"*) bad "$d: GET / still carries marker $o" ;;
    *) ok "$d: GET / has no marker $o" ;;
  esac
  expect "$d: marker asset of $l served" "$(curl -s --max-time 5 "$root/assets/e2e-marker-$l.js")" "window.__E2E_UI__=\"$l\";"
  expect "$d: marker asset of $o gone" "$(curl -s -o /dev/null -w '%{http_code}' "$root/assets/e2e-marker-$o.js")" 404
  expect "$d: version.json is the running version" "$(curl -s "$root/version.json" | jq -r .version)" "$(running_version)"
  js="$(printf '%s' "$html" | grep -o '/assets/index-[A-Za-z0-9_-]*\.js' | head -1)"
  expect "$d: the real bundle referenced by index.html is served" "$(curl -s -o /dev/null -w '%{http_code}' "$root$js")" 200
}
published_count() { grep -c "GET /cloud_home-$1-" "$WORK/mirror.log"; }

wait_until 60 version_is "$VER_A" || { tail -5 "$WORK/server.log" | cut -c1-300; die "server A did not come up"; }
expect "A runs and the updater is enabled" "$(snap '.data.enabled')" true

# Some data that must survive an update and a rollback.
curl -s -o /dev/null -X POST -H 'Content-Type: application/json' \
  -d '{"username":"owner","email":"owner@example.com","password":"correct-horse-battery"}' "$API/auth/register"
expect "setup: one account exists before any update" "$(db 'SELECT COUNT(*) FROM users')" 1
ui_check "A running" A B

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

echo "== scenario: release with a broken embedded UI is refused by --self-test before the switch"
for broken in "$VER_D:d:index.html references a missing asset" "$VER_E:e:UI version differs from the binary"; do
  IFS=: read -r bv bl bdesc <<<"$broken"
  publish "$WORK/dist-$bl" "$bv" "$MIRROR" "$WORK/key-good/key.sec" "$MIRROR_URL"
  admin POST /admin/update/check >/dev/null
  expect "broken UI ($bdesc): offered, it is a validly signed newer release" "$(snap '.data.available.version')" "$bv"
  expect "broken UI ($bdesc): install accepted for processing" "$(admin_code POST /admin/update/install)" 202
  wait_until 30 phase_is failed
  expect "broken UI ($bdesc): updater ends in failed" "$(snap '.data.phase')" failed
  case "$(snap '.data.last_error')" in
    *自检失败*) ok "broken UI ($bdesc): rejected by the self-test" ;;
    *) bad "broken UI ($bdesc): expected a self-test error, got $(snap '.data.last_error')" ;;
  esac
  expect "broken UI ($bdesc): current still A" "$(link current)" "versions/$VER_A"
  expect "broken UI ($bdesc): A still serving" "$(running_version)" "$VER_A"
  if [ ! -e "$ROOT/versions/$bv" ]; then ok "broken UI ($bdesc): nothing installed"; else bad "broken UI ($bdesc): versions/$bv exists"; fi
  expect "broken UI ($bdesc): no pending state" "$(state '.pending')" none
  ui_check "broken UI ($bdesc): old UI intact" A B
done

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
ui_check "happy: after the update" B A
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
ui_check "crash: after the rollback" B C
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

summary
