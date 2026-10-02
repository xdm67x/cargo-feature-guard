# .agents/

Agent-agnostic configuration following the [Agent Skills](https://agentskills.io) open standard and [AGENTS.md](https://agents.md) conventions.

## Layout

- `agents/` — Sub-agent definitions (`<name>.md` with `name` + `description` frontmatter)
- `skills/<skill-name>/SKILL.md` — Shared skills (portable, any compatible agent)
- `scripts/link-skills.sh` — Symlinks everything into agent-native directories

## Usage

```bash
.agents/scripts/link-skills.sh
```

This creates symlinks in `.claude/agents/` and `.claude/skills/` so Claude Code
picks up the same definitions without duplicated content. `.agents/` is the
single source of truth; never edit the symlinks' targets elsewhere.

## Adding a skill

1. Create `.agents/skills/<name>/SKILL.md` with `name` + `description` frontmatter
2. Run `.agents/scripts/link-skills.sh`
3. Commit the skill directory and the new symlink

## Adding an agent

1. Create `.agents/agents/<name>.md` with `name`, `description`, and `tools` frontmatter
2. Run `.agents/scripts/link-skills.sh`
3. Commit the agent file and the new symlink
