#!/usr/bin/env bash
# Install local git hooks (no git config changes).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
hooks_src="$root/.githooks"
hooks_dst="$root/.git/hooks"

mkdir -p "$hooks_dst"
for hook in commit-msg pre-commit; do
  cp "$hooks_src/$hook" "$hooks_dst/$hook"
  chmod +x "$hooks_dst/$hook"
  echo "Installed $hook"
done
echo "Done. Commits will reject Cursor/agent attribution."
