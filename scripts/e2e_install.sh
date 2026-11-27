#!/usr/bin/env bash
# R-021 runtime check: packaging/{linux,macos}/install.sh against a local mirror signed with a throwaway key,
# using their documented test hooks (--no-system, UPDATE_ROOT, CLOUD_HOME_BASE_URL, CLOUD_HOME_PUBLIC_KEY).
#
#   scripts/e2e_install.sh
#
# The packages hold a tiny shell script that answers --self-test, so the Linux script can run on a Mac too.
# NOT covered: the real systemd / launchd parts (user creation, unit file, enabling the service) need root and
# a Linux machine; --no-system skips exactly those.
set -uo pipefail
# shellcheck source=scripts/e2e_lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/e2e_lib.sh"

for c in minisign python3 curl tar; do
  command -v "$c" >/dev/null || die "missing command: $c"
done

WORK="$(mktemp -d)"
MIRROR_PID=""
cleanup() {
  [ -n "$MIRROR_PID" ] && kill "$MIRROR_PID" 2>/dev/null
  wait 2>/dev/null
  rm -rf "$WORK"
}
trap cleanup EXIT

VER=9.9.9
MIRROR="$WORK/mirror"
mkdir -p "$MIRROR"
gen_key "$WORK/key-good"
gen_key "$WORK/key-other"
start_mirror "$MIRROR" "$WORK/mirror.log"
URL="http://127.0.0.1:$MIRROR_PORT"

# fake_binary <file> <version> <target>: stands in for cloud_home, answers --self-test only.
fake_binary() {
  printf '#!/bin/sh\n[ "$1" = --self-test ] && echo "ok %s %s"\n' "$2" "$3" > "$1"
}

# install <script> <public key or empty for the built-in one>: runs the installer into a fresh root.
# Sets OUT (output) and RC (exit code); the root is $WORK/root.
install() {
  rm -rf "$WORK/root"
  run_install "$1" "$2"
}
run_install() {
  local env=(UPDATE_ROOT="$WORK/root" CLOUD_HOME_BASE_URL="$URL")
  [ -n "$2" ] && env+=(CLOUD_HOME_PUBLIC_KEY="$2")
  OUT="$(env "${env[@]}" bash "$1" --no-system 2>&1)"
  RC=$?
}
refused() { # refused <description> <expected message fragment>
  if [ "$RC" != 0 ]; then ok "$1: installer exits non-zero"; else bad "$1: installer succeeded"; fi
  case "$OUT" in
    *"$2"*) ok "$1: says '$2'" ;;
    *) bad "$1: expected '$2' in: $OUT" ;;
  esac
  if [ ! -e "$WORK/root/current" ]; then ok "$1: nothing installed"; else bad "$1: current link exists"; fi
}
perms() { stat -f %Lp "$1" 2>/dev/null || stat -c %a "$1"; }

suite() { # suite <name> <script> <target>
  local name="$1" script="$2" target="$3" good dist="$WORK/dist-$1"
  good="$(cat "$WORK/key-good/pubkey.txt")"
  echo "== $name ($script, target $target)"
  fake_binary "$WORK/bin" "$VER" "$target"
  make_pkg "$dist" "$VER" "$target" "$WORK/bin"
  publish "$dist" "$VER" "$MIRROR" "$WORK/key-good/key.sec" "$URL"

  install "$script" "$good"
  expect "happy: exit code" "$RC" 0
  if [ -x "$WORK/root/versions/$VER/cloud_home" ]; then ok "happy: binary placed and executable"; else bad "happy: no executable binary ($OUT)"; fi
  expect "happy: current link" "$(readlink "$WORK/root/current")" "versions/$VER"
  expect "happy: .env mode" "$(perms "$WORK/root/.env")" 600
  if grep -Eq '^JWT_SECRET=.{40,}$' "$WORK/root/.env"; then ok "happy: random JWT_SECRET generated"; else bad "happy: JWT_SECRET missing or short"; fi
  case "$OUT" in
    *"$(sed -n 's/^JWT_SECRET=//p' "$WORK/root/.env")"*) bad "happy: JWT_SECRET leaked into the output" ;;
    *) ok "happy: JWT_SECRET not printed" ;;
  esac

  run_install "$script" "$good"
  expect "rerun: exit code" "$RC" 0
  case "$OUT" in
    *不覆盖*) ok "rerun: existing install is not overwritten" ;;
    *) bad "rerun: expected the 'not overwritten' message, got: $OUT" ;;
  esac

  echo "-- refusals"
  python3 -I - "$MIRROR/manifest.json" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
open(p, "w").write(s.replace('"size":', '"size": 1,\n      "x":', 1))
PY
  install "$script" "$good"
  refused "tampered manifest" 签名校验失败

  publish "$dist" "$VER" "$MIRROR" "$WORK/key-other/key.sec" "$URL"
  install "$script" "$good"
  refused "manifest signed with another key" 签名校验失败

  install "$script" ""
  refused "throwaway signature against the built-in key" 签名校验失败

  publish "$dist" "$VER" "$MIRROR" "$WORK/key-good/key.sec" "$URL"
  python3 -I - "$MIRROR/cloud_home-$VER-$target.tar.gz" <<'PY'
import sys
p = sys.argv[1]
b = bytearray(open(p, "rb").read())
b[len(b) // 2] ^= 0xFF
open(p, "wb").write(b)
PY
  install "$script" "$good"
  refused "tampered package" "sha256 不符"

  make_pkg "$WORK/dist-other" "$VER" some-other-target "$WORK/bin"
  publish "$WORK/dist-other" "$VER" "$MIRROR" "$WORK/key-good/key.sec" "$URL"
  install "$script" "$good"
  refused "no package for this machine" "没有 $target 的安装包"

  fake_binary "$WORK/bin-bad" "0.0.1" "$target"
  rm -rf "$WORK/dist-bad"
  make_pkg "$WORK/dist-bad" "$VER" "$target" "$WORK/bin-bad"
  publish "$WORK/dist-bad" "$VER" "$MIRROR" "$WORK/key-good/key.sec" "$URL"
  install "$script" "$good"
  refused "binary whose self-test reports another version" 自检失败
}

suite "linux" "$REPO/packaging/linux/install.sh" "$([ "$(uname -m)" = x86_64 ] && echo x86_64-unknown-linux-musl || echo aarch64-unknown-linux-musl)"
if [ "$(uname -s)-$(uname -m)" = Darwin-arm64 ]; then
  suite "macos" "$REPO/packaging/macos/install.sh" aarch64-apple-darwin
else
  echo "== macos: skipped, needs an Apple silicon Mac"
fi
summary
