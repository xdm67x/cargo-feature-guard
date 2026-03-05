---
name: commit
description: Commit staged changes using conventional commits. Only feat and fix types are allowed. Use when the user says "commit", "/commit", or wants to create a git commit.
allowed-tools: Bash
---

# Conventional Commit (feat/fix only)

Create a git commit following the Conventional Commits format, restricted to `feat` and `fix` types.

## Steps

1. **Gather context** — Run these commands in parallel:
   - `git status` to see staged and unstaged changes
   - `git diff --cached` to inspect what will be committed
   - `git log --oneline -5` for recent commit style reference

2. **If nothing is staged**, check `git diff` for unstaged changes and ask the user what to stage before proceeding.

3. **Classify the change** as one of:
   - `feat`: new functionality, new behavior, new capability
   - `fix`: bug fix, correction of wrong/broken behavior

4. **Draft the commit message** and present it for confirmation:
   ```
   feat: add version bump workflow
   ```
   or with optional scope:
   ```
   fix(ci): correct merge commit detection in auto-tag
   ```

5. **Wait for user confirmation** before committing. If the user wants changes, adjust accordingly.

6. **Create the commit** using a HEREDOC:
   ```bash
   git commit -m "$(cat <<'EOF'
   feat: the message here

   Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
   EOF
   )"
   ```

7. **Run `git status`** after committing to verify success.

## Rules

- ONLY `feat` or `fix` as type prefix. No other types allowed (no chore, docs, refactor, style, test, etc.).
- If the change doesn't clearly fit either type, ask the user which to use.
- Subject line: imperative mood, lowercase, no trailing period, max 72 characters.
- Do NOT push to remote unless explicitly asked.
- Do NOT amend previous commits unless explicitly asked.
- If pre-commit hooks fail, fix the issue and create a NEW commit (never amend).
