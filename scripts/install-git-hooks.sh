#!/usr/bin/env bash
# scripts/install-git-hooks.sh — opt-in pre-push hook that runs verify.sh.
#
# Writes .git/hooks/pre-push. The hook is optional; running
# `./scripts/verify.sh` before every push is required per
# CONTRIBUTING.md. The hook just makes that non-forgettable.
#
# Behaviour:
#   - Skips when `git push --no-verify` is used.
#   - Skips when the caller exports `SKIP_VERIFY=1`.
#   - Runs `scripts/verify.sh --fast` by default; set
#     `VERIFY_MODE=full` to run the complete gate (slow on
#     Proton Drive — see phase-02 session report).
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
hook_path="$repo_root/.git/hooks/pre-push"

if [ ! -d "$repo_root/.git" ]; then
  echo "not a git repo at $repo_root" >&2
  exit 1
fi

cat > "$hook_path" <<'HOOK'
#!/usr/bin/env bash
# Pre-push hook — runs the local quality gate. See
# scripts/install-git-hooks.sh for install + skip-options.
set -euo pipefail

if [ "${SKIP_VERIFY:-0}" = "1" ]; then
  echo "SKIP_VERIFY=1 — skipping local verify"
  exit 0
fi

cd "$(git rev-parse --show-toplevel)"

mode="${VERIFY_MODE:-fast}"
if [ "$mode" = "full" ]; then
  exec ./scripts/verify.sh
else
  exec ./scripts/verify.sh --fast
fi
HOOK

chmod +x "$hook_path"
echo "Installed $hook_path"
echo "Skip a single push with: git push --no-verify"
echo "Skip via env:            SKIP_VERIFY=1 git push"
echo "Run the full (slow) gate: VERIFY_MODE=full git push"
