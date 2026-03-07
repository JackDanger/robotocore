---
role: assistant
model: claude-opus-4-6
timestamp: "2026-03-07T19:16:00Z"
session: "142fa5"
sequence: 14
---

Improvements made to the promptlog spec (`/Users/jackdanger/www/jackdanger.com/static/promptlog.md`) based on first real-world use:

1. **Added `tools` field** — records when subagents or git worktrees were used, important for understanding how parallelized work was orchestrated
2. **Added `sequence` field** — orders prompts within a session so readers can follow conversation flow without relying on timestamp sort
3. **Added "Retroactive logging" section** — the original spec assumed real-time logging, but this project needed reconstruction from session transcripts. Added `reconstructed: true` field and guidance.
4. **Added "Linking prompts to commits" section** — addressed the circular dependency: a commit SHA can't appear inside the commit. Documented three approaches (amend after, next-commit backfill, don't link at all) and recommended option 2 or 3 for shared branches.
5. **Expanded assistant entry guidance** — for long autonomous sessions, one entry per logical phase is enough. Log plans and decisions, not every tool call.
