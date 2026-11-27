#!/bin/bash
# Publish the report site to the owner's server (berton.home).
#
#   scripts/deploy_report.sh            deploy manager-site/report_web
#   DRY_RUN=1 scripts/deploy_report.sh  print what would happen, change nothing
#
# The server serves ~/report_web with `python3 -m http.server` behind nginx +
# Cloudflare. The folder is replaced as a whole (the owner's rule: "override"),
# so manager-site/report_web must always be a COMPLETE build including every
# past report. No backup is kept: the old folder is deleted on every deploy.
set -euo pipefail

HOST="berton.home"
URL="https://report.cia1099.cloudns.ch"
SRC="$(cd "$(dirname "$0")/.." && pwd)/manager-site/report_web"
run() { if [ "${DRY_RUN:-0}" = "1" ]; then echo "[dry-run] $*"; else "$@"; fi; }

[ -f "$SRC/index.html" ] || { echo "error: $SRC/index.html missing — run 'bun run build' in manager-site/ first" >&2; exit 1; }

# The site is PUBLIC (no login). Refuse to publish anything that looks like a secret,
# a private path, or the owner's contact details.
if grep -rIlE 'SITE_TOKEN|JWT_SECRET|ADMIN_PASSWORD|MINISIGN_SECRET|CHAT_BOT_TOKEN|chat\.env|[0-9]{8,10}:[A-Za-z0-9_-]{35}|BEGIN ([A-Z]+ )?PRIVATE KEY|/Users/otto|cia1099@|\.ssh/|manager\.env' "$SRC" | head -5 | grep . ; then
  echo "error: the files above contain something that must not be published. Fix and rebuild." >&2
  exit 1
fi

SIZE_MB=$(du -sm "$SRC" | cut -f1)
echo "deploying $SRC (${SIZE_MB} MB) -> $HOST:~/report_web"
[ "$SIZE_MB" -le 400 ] || { echo "error: build is ${SIZE_MB} MB (limit 400) — shrink the media" >&2; exit 1; }

run ssh -o BatchMode=yes "$HOST" 'rm -rf ~/report_web.new'
run scp -rq "$SRC" "$HOST:~/report_web.new"
# swap: delete the old folder, then move the new one into place (no backup)
run ssh -o BatchMode=yes "$HOST" 'rm -rf ~/report_web && mv ~/report_web.new ~/report_web'

if [ "${DRY_RUN:-0}" != "1" ]; then
  code=$(curl -sS -m 20 -o /dev/null -w '%{http_code}' "$URL/" || true)
  echo "live check: $URL/ -> HTTP $code"
  [ "$code" = "200" ] || { echo "error: site did not answer 200 after deploy; fix the build and deploy again" >&2; exit 1; }
fi
echo "done: $URL/"
