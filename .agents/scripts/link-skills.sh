#!/usr/bin/env bash
# Sync .agents/ (single source of truth) into agent-native directories
# via symlinks so every tool sees the same agents and skills.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

link() {
  local src=$1 dst=$2
  mkdir -p "$(dirname "$dst")"
  if [ -L "$dst" ] && [ "$(readlink "$dst")" = "$src" ]; then
    return
  fi
  rm -f "$dst"
  ln -s "$src" "$dst"
  echo "linked $dst -> $src"
}

for skill in .agents/skills/*/; do
  name=$(basename "$skill")
  link "../../.agents/skills/$name" ".claude/skills/$name"
done

for agent in .agents/agents/*.md; do
  name=$(basename "$agent" .md)
  link "../../.agents/agents/$name.md" ".claude/agents/$name.md"
done
