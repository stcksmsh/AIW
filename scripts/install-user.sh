#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
skill_source="$repo_root/integrations/aiw-agent/skills/aiw-workspace"

# --force makes this command an explicit update path for a newer checked-out or
# verified source release. Cargo's lockfile pins the dependency resolution.
cargo install --path "$repo_root" --locked --force

for skill_root in "$HOME/.agents/skills" "$HOME/.claude/skills"; do
  destination="$skill_root/aiw-workspace"
  mkdir -p "$skill_root"
  temporary="$skill_root/.aiw-workspace.installing.$$"
  trap 'rm -rf "$temporary"' EXIT
  cp -R "$skill_source" "$temporary"
  if [ -e "$destination" ]; then
    backup="$destination.backup.$(date +%Y%m%d%H%M%S).$$"
    mv "$destination" "$backup"
    echo "Backed up existing skill to $backup"
  fi
  mv "$temporary" "$destination"
  trap - EXIT
done

echo "Installed aiw to $(command -v aiw)"
echo "Installed personal AIW skill for Codex and Claude Code"
echo "Enable a repository with: aiw init && aiw integrate all --enforcement strict"
