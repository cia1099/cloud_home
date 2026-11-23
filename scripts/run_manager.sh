#!/bin/bash
# Start the Cloud Home manager as the main Claude Code session.
#   scripts/run_manager.sh            interactive (you can watch / step in)
#   scripts/run_manager.sh -p "..."   headless, prints the result and exits
#
# The manager's guardrails live in .claude/manager.settings.json: it may build,
# test and commit locally, but git push/tag, releases, ssh/scp/rsync, sudo and
# the signing key are denied. A command outside the allow-list makes the session
# wait for your approval instead of running (safe, but unattended runs stall).
set -euo pipefail
cd "$(dirname "$0")/.."
# In headless (-p) mode Claude Code kills background subagents ~10 min after the last turn and then
# exits. Subagent tasks (design, builds) routinely take longer, so wait for them indefinitely.
export CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0
if [ "$#" -eq 0 ]; then
  set -- "Read SPEC.md and plan/STATUS.md, then run your operating loop."
fi
exec claude --agent manager \
  --settings .claude/manager.settings.json \
  --permission-mode acceptEdits \
  "$@"
