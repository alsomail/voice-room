---
name: commit-with-status
description: Review git status, summarize repository changes, generate a concise commit message, then create a git commit.
argument-hint: "[optional-commit-subject]"
disable-model-invocation: true
allowed-tools: Read, Grep, Bash(git --no-pager branch --show-current), Bash(git --no-pager status *), Bash(git --no-pager diff *), Bash(git add -- *), Bash(git commit -m * -m *)
---

# Commit With Status

Use this skill when you want a safe commit workflow that always inspects the repository state first.

## Repository snapshot

- Current branch: !`git --no-pager branch --show-current`
- Git status: !`git --no-pager status --short`

## Your task

1. Run `git --no-pager status --short` again if needed and summarize the current repository state in a few bullets:
   - modified files
   - new files
   - deleted files
   - whether changes are already staged
2. Inspect relevant diffs before writing the commit message:
   - `git --no-pager diff --stat`
   - `git --no-pager diff --cached --stat`
   - `git --no-pager diff -- <path>` for important files when the summary is unclear
3. Generate a concise commit message:
   - If `$ARGUMENTS` is provided, treat it as the preferred commit subject and refine only if needed.
   - If no argument is provided, write a subject based on the actual change set.
   - Keep the message focused on the meaningful user-facing or architectural change.
4. Stage the intended files explicitly with `git add ...` only after confirming the repository state matches the summary.
5. If unrelated files are already staged, abort instead of trying to repair the index.
6. Before committing, run a final staged-only verification with `git --no-pager diff --cached --stat` and confirm it matches the summary.
7. Commit non-interactively with an explicit command pattern that includes the final message and this trailer exactly:

   `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`

   Preferred command form:

   `git commit -m "<subject>" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"`

8. After committing, report:
   - the final commit subject
   - the files included
   - the high-level summary of what changed

## Constraints

- Never skip the `git status` summary.
- Never use interactive git commands.
- Never amend unless the user explicitly asks.
- Never include unrelated dirty files in the commit.
- Never proceed if unrelated files are already staged.
- If the worktree contains unrelated changes and the correct commit scope is ambiguous, stop and ask the user instead of guessing.
