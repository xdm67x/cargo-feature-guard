---
name: pr
description: Create or update a pull request with a structured description. Use when the user says "pr", "/pr", "create pr", "update pr", or wants to open/edit a pull request.
allowed-tools: Bash
---

# Pull Request Creation/Update

Create or update a PR with a concise, structured description that explains the why and the how.

## Steps

1. **Gather context** — Run in parallel:
   - `git branch --show-current` to get current branch
   - `git log main..HEAD --oneline` to see all commits in this branch
   - `git diff main...HEAD --stat` to see changed files
   - `gh pr view --json number,title,body 2>/dev/null` to check if a PR already exists

2. **If no remote tracking branch**, push with:
   ```bash
   git push -u origin $(git branch --show-current)
   ```

3. **Read the actual diff** to understand the changes:
   ```bash
   git diff main...HEAD
   ```

4. **Draft the PR description** following this template:

   ```markdown
   ## Why

   [2-3 sentences explaining the context and motivation. Why was this change needed? What problem does it solve?]

   ## How

   [A short paragraph explaining the principle/approach behind the changes. Do not list every file or every change — focus on the overall strategy and mention only critical changes that need attention.]
   ```

5. **Present the title and description** to the user for confirmation before creating/updating.

6. **Create or update the PR**:
   - If no PR exists: `gh pr create --title "..." --body-file /tmp/pr-body.md`
   - If PR exists: `gh pr edit --title "..." --body-file /tmp/pr-body.md`

7. **Return the PR URL** to the user.

## Writing Rules

- **Why section**: Focus on context and motivation. What triggered this work? What was broken or missing?
- **How section**: Explain the approach, not the individual changes. Think "I introduced X pattern to achieve Y" rather than "I changed file A line 30". Only call out critical or non-obvious changes that a reviewer should pay attention to.
- Keep the entire description short — aim for under 15 lines total.
- PR title: short, imperative mood, under 70 characters.
- Do NOT use bullet points to list every commit or every file changed.
- Do NOT push to remote or create the PR without user confirmation.
