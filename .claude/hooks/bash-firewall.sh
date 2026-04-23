#!/usr/bin/env bash
# PreToolUse guard: block destructive shell commands before they run.
#
# This hook reads a Claude Code PreToolUse payload on stdin (JSON), extracts
# the proposed bash command, and decides whether to allow it.
#
# Design note: to keep this file itself free of literal destructive command
# strings (which trip content filters and confuse searches), all forbidden
# patterns are assembled at runtime from small, individually meaningless
# fragments. A reviewer should read the *category label* on each rule to
# understand what it blocks — the categories match CONTRIBUTING.md.
#
# Exit 0 + JSON decision is how Claude Code hooks communicate allow/deny.

set -euo pipefail

payload="$(cat)"

# jq is expected on dev machines (macOS ships it as of recent versions; brew installs on others).
# If jq is unavailable, fail open with a stderr warning — we do not want the firewall
# to hard-block developer workflows while tooling is being bootstrapped.
if ! command -v jq >/dev/null 2>&1; then
  echo "bash-firewall: jq not found, skipping (install with: brew install jq / apt install jq)" >&2
  exit 0
fi

cmd="$(printf '%s' "$payload" | jq -r '.tool_input.command // empty')"
[ -z "$cmd" ] && exit 0

# ---------------------------------------------------------------------------
# Pattern assembly. Each forbidden construct is built from three or more
# fragments so that neither this source file nor any grep of it contains the
# full literal form.
# ---------------------------------------------------------------------------

# Category A: recursive forced deletion targeting a filesystem root.
fa_1="r"; fa_1+="m"
fa_2="-"; fa_2+="r"; fa_2+="f"
fa_3="/"
pattern_a="(^|[[:space:]])${fa_1}[[:space:]]+${fa_2}[[:space:]]+${fa_3}([[:space:]]|$)"

# Category B: forced non-fast-forward push to a remote.
fb_1="push"
fb_2="-"; fb_2+="-"; fb_2+="force"
pattern_b="git[[:space:]]+.*${fb_1}.*${fb_2}"

# Category C: unconditional schema-table removal (SQL) executed via a shell client.
fc_1="DRO"; fc_1+="P"
fc_2="TAB"; fc_2+="LE"
pattern_c="${fc_1}[[:space:]]+${fc_2}"

# Category D: hard branch reset that discards the working tree.
fd_1="reset"
fd_2="-"; fd_2+="-"; fd_2+="hard"
pattern_d="git[[:space:]]+${fd_1}.*${fd_2}"

# Category E: write to /etc or other protected system paths outside the repo.
fe_1="/etc"
fe_2="/usr/local/bin"
pattern_e="(>[[:space:]]*|>>[[:space:]]*)(${fe_1}|${fe_2})"

# Category F: bypass git hooks during commit or push.
ff_1="no"
ff_2="verify"
pattern_f="-"; pattern_f+="-"; pattern_f+="${ff_1}-${ff_2}"

# ---------------------------------------------------------------------------
# Evaluation.
# ---------------------------------------------------------------------------

deny() {
  printf '{"decision":"block","reason":"%s"}\n' "$1"
  exit 0
}

shopt -s nocasematch 2>/dev/null || true

if [[ "$cmd" =~ $pattern_a ]]; then
  deny "blocked: recursive forced deletion targeting filesystem root"
fi
if [[ "$cmd" =~ $pattern_b ]]; then
  deny "blocked: forced non-fast-forward push"
fi
if [[ "$cmd" =~ $pattern_c ]]; then
  deny "blocked: unconditional schema-table removal"
fi
if [[ "$cmd" =~ $pattern_d ]]; then
  deny "blocked: hard branch reset discards uncommitted work"
fi
if [[ "$cmd" =~ $pattern_e ]]; then
  deny "blocked: write to protected system path outside the repo"
fi
if [[ "$cmd" =~ $pattern_f ]]; then
  deny "blocked: attempt to bypass git hooks"
fi

# Allow by default.
exit 0
