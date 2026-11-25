#!/bin/bash
# Start the Cloud Home manager as the main Claude Code session.
#   scripts/run_manager.sh            interactive (you can watch / step in)
#   scripts/run_manager.sh -p "..."   headless: prints the result and exits. If the usage limit
#                                     ("You've hit your 5-hour limit · resets 6:10am (Asia/Taipei)")
#                                     stops the manager, this script waits for the reset and starts a
#                                     FRESH session that recovers from the notes on disk
#                                     (plan/STATUS.md, git, worktrees). It never resumes an old session.
#
# The manager's guardrails live in .claude/manager.settings.json: it may build,
# test and commit locally, but git push/tag, releases, ssh/scp/rsync, sudo and
# the signing key are denied. A command outside the allow-list makes the session
# wait for your approval instead of running (safe, but unattended runs stall).
#
# Environment: LIMIT_RESUME_MARGIN_SECONDS (default 120) is added to the reset time;
#              LIMIT_MAX_WAITS (default 8) caps how many resets are waited out.
set -euo pipefail
cd "$(dirname "$0")/.."
# In headless (-p) mode Claude Code kills background subagents ~10 min after the last turn and then
# exits. Subagent tasks (design, builds) routinely take longer, so wait for them indefinitely.
export CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0

if [ "$#" -eq 0 ]; then
  set -- "Read SPEC.md and plan/STATUS.md, then run your operating loop."
fi

claude_args=(--agent manager --settings .claude/manager.settings.json --permission-mode acceptEdits)

# Single instance: two managers on one repo would fight over plan/STATUS.md, branches and worktrees.
# mkdir is atomic, so it works as a lock on macOS (no flock). The lock lives in .git (untracked).
# A lock whose owner is gone (crash, SIGKILL, reboot) is detected by its PID and taken over.
lock="$(git rev-parse --git-common-dir)/manager.lock"
if ! mkdir "$lock" 2>/dev/null; then
  owner="$(cat "$lock/pid" 2>/dev/null || true)"
  if [ -n "$owner" ] && kill -0 "$owner" 2>/dev/null && ps -p "$owner" -o command= | grep -q run_manager; then
    echo "[run_manager] already running (pid $owner); refusing to start a second manager." >&2
    echo "[run_manager] stop it first: pkill -f run_manager.sh; pkill -f 'claude.*--agent manager'" >&2
    exit 73
  fi
  echo "[run_manager] removing stale lock (pid ${owner:-unknown} is not running)" >&2
  rm -rf "$lock"
  mkdir "$lock"
fi
echo "$$" >"$lock/pid"
trap 'rm -rf "$lock"' EXIT

headless=0
for arg in "$@"; do
  case "$arg" in -p | --print) headless=1 ;; esac
done
# Interactive: you are watching, nothing to wrap. Not exec: the shell must stay alive to release the lock.
if [ "$headless" = 0 ]; then
  claude "${claude_args[@]}" "$@"
  exit $?
fi

# Seconds from now until the reset time named in a limit message, plus a margin.
# Understands "resets 6:10am (Asia/Taipei)", "resets 3pm", "resets Mon 9am (UTC)".
# Prints 1800 (30 min) when the message cannot be parsed, so a changed wording only slows us down.
seconds_until_reset() {
  python3 - "$1" "$2" <<'PY'
import datetime as dt
import re
import sys

line, margin = sys.argv[1], int(sys.argv[2])
m = re.search(r"resets\s+(?:(mon|tue|wed|thu|fri|sat|sun)\w*\s+)?(\d{1,2})(?::(\d{2}))?\s*([ap]m)?\s*(?:\(([^)]+)\))?", line, re.I)
if not m:
    print(1800)
    sys.exit()
day, hour, minute, ampm, tzname = m.groups()
try:
    from zoneinfo import ZoneInfo
    tz = ZoneInfo(tzname) if tzname else dt.datetime.now().astimezone().tzinfo
except Exception:
    tz = dt.datetime.now().astimezone().tzinfo
hour, minute = int(hour), int(minute or 0)
if ampm:
    hour = hour % 12 + (12 if ampm.lower() == "pm" else 0)
now = dt.datetime.now(tz)
target = now.replace(hour=hour % 24, minute=minute, second=0, microsecond=0)
if day:
    want = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"].index(day.lower()[:3])
    target += dt.timedelta(days=(want - target.weekday()) % 7)
if target <= now:
    target += dt.timedelta(days=7 if day else 1)
print(int((target - now).total_seconds()) + margin)
PY
}

margin="${LIMIT_RESUME_MARGIN_SECONDS:-120}"
max_waits="${LIMIT_MAX_WAITS:-8}"
log="$(mktemp)"
trap 'rm -f "$log"; rm -rf "$lock"' EXIT

original=("$@")
last=$(($# - 1))
note="NOTE: your previous session was interrupted by the usage limit, and the notes on disk are all you have. First run your resume protocol (plan/STATUS.md Active tasks, git status, git worktree list, the task branches), verify what is there, and continue from it rather than starting over. Your task:"

for ((attempt = 0; attempt <= max_waits; attempt++)); do
  args=("${original[@]}")
  # Only prefix the task text; never a trailing flag such as "-p" with no prompt after it.
  if [ "$attempt" -gt 0 ] && [[ "${original[$last]}" != -* ]]; then
    args[$last]="$note"$'\n\n'"${original[$last]}"
  fi

  set +e
  claude "${claude_args[@]}" "${args[@]}" 2>&1 | tee "$log"
  status=${PIPESTATUS[0]}
  set -e

  # Only a failed run whose last lines are the limit message counts; a report that merely
  # quotes the phrase must not trigger a restart.
  limit_line="$(tail -n 5 "$log" | grep -iE "hit your .*limit" | tail -n 1 || true)"
  if [ "$status" -eq 0 ] || [ -z "$limit_line" ]; then
    exit "$status"
  fi
  if [ "$attempt" -eq "$max_waits" ]; then
    echo "[run_manager] usage limit hit $((max_waits + 1)) times; giving up. Last message: $limit_line" >&2
    exit 75
  fi
  wait_seconds="$(seconds_until_reset "$limit_line" "$margin")"
  echo "[run_manager] $limit_line" >&2
  echo "[run_manager] waiting ${wait_seconds}s for the reset, then starting a fresh session (attempt $((attempt + 2)))" >&2
  sleep "$wait_seconds"
done
