#!/usr/bin/env bash
# Shared helpers for scripts/e2e_update.sh and scripts/e2e_install.sh (sourced, not run).
# Needs: minisign, python3, curl, tar.

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PASS=0
FAIL=0

die() { echo "ABORT: $*" >&2; exit 2; }
ok() { PASS=$((PASS + 1)); echo "  PASS  $1"; }
bad() { FAIL=$((FAIL + 1)); echo "  FAIL  $1"; }

# expect <description> <actual> <expected>
expect() {
  if [ "$2" = "$3" ]; then ok "$1"; else bad "$1 (expected '$3', got '$2')"; fi
}

summary() {
  echo "== $PASS passed, $FAIL failed =="
  [ "$FAIL" = 0 ]
}

free_port() {
  python3 -I -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'
}

# gen_key <dir>: throwaway passwordless minisign key pair -> <dir>/key.pub, <dir>/key.sec, <dir>/pubkey.txt
gen_key() {
  mkdir -p "$1"
  minisign -G -W -f -p "$1/key.pub" -s "$1/key.sec" >/dev/null || die "minisign keygen failed"
  sed -n 2p "$1/key.pub" > "$1/pubkey.txt"
}

# make_pkg <dist_dir> <version> <target> <binary_file>: cloud_home-<ver>-<target>.tar.gz holding <name>/cloud_home
make_pkg() {
  local name="cloud_home-$2-$3" stage
  stage="$(mktemp -d)"
  mkdir -p "$stage/$name" "$1"
  cp "$4" "$stage/$name/cloud_home"
  chmod 755 "$stage/$name/cloud_home"
  COPYFILE_DISABLE=1 tar -czf "$1/$name.tar.gz" -C "$stage" "$name"
  rm -rf "$stage"
}

# publish <dist_dir> <version> <mirror_dir> <seckey> <mirror_url>
# Builds the manifest with scripts/make_manifest.py, points its package URLs at the mirror, signs it.
# Replaces whatever the mirror served before.
publish() {
  local dist="$1" version="$2" mirror="$3" sec="$4" url="$5"
  python3 -I "$REPO/scripts/make_manifest.py" "$version" example/repo "$dist" >/dev/null || die "make_manifest failed"
  sed -i.bak "s#https://github.com/example/repo/releases/download/v$version/#$url/#" "$dist/manifest.json"
  rm -f "$dist/manifest.json.bak"
  grep -q "\"url\": \"$url/" "$dist/manifest.json" || die "manifest url rewrite failed"
  minisign -S -s "$sec" -m "$dist/manifest.json" -x "$dist/manifest.json.minisig" >/dev/null || die "minisign sign failed"
  rm -f "$mirror"/*
  cp "$dist"/* "$mirror"/
}

# start_mirror <dir> <logfile>: serves <dir> on a free loopback port; sets MIRROR_PORT and MIRROR_PID.
start_mirror() {
  MIRROR_PORT="$(free_port)"
  python3 -I -m http.server "$MIRROR_PORT" --bind 127.0.0.1 --directory "$1" >"$2" 2>&1 &
  MIRROR_PID=$!
  local i
  for i in $(seq 50); do
    curl -s -o /dev/null "http://127.0.0.1:$MIRROR_PORT/" && return 0
    sleep 0.1
  done
  die "mirror did not start"
}
