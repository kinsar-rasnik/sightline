# Synthetic Workforce Blueprint

> **Purpose.** This document defines the operating model for a human-led, AI-assisted software organization running inside a Claude Code project. It is the single reference that every subagent, every skill, and every contributor is expected to have read. It is deliberately kept short. If you need more detail on any topic, follow the cross-references into the specific artifact.

> **Status.** Canonical. Changes require an ADR.

---

## 1. Operating Model

Four explicit roles, each with a narrow remit:

| Role              | Who       | Primary responsibility                                                                                     |
| ----------------- | --------- | ---------------------------------------------------------------------------------------------------------- |
| **CEO**           | Human     | Vision, priorities, external relationships, final call on scope and direction.                             |
| **CTO**           | Human     | Technical strategy, architecture guardrails, quality bar, phase gating, delegation to the Senior Engineer. |
| **Senior Engineer** | Claude  | End-to-end delivery of a phase: implementation, tests, docs, CI, review. Owns the code until handoff.      |
| **Subagents**     | Claude    | Narrow, task-scoped specialists invoked by the Senior Engineer for parallelizable or high-focus work.      |

The CEO talks to the CTO. The CTO writes the prompt. The Senior Engineer executes, using subagents as force multipliers. No subagent talks to the CEO directly.

**Why this hierarchy.** Every layer has a different cost of mistakes and a different context window. Keeping the layers explicit means a mistake at the subagent level never propagates upward silently — the Senior Engineer catches it, the CTO reviews the Senior Engineer's output, and the CEO sees only vetted work.

---

## 2. Governance Principles

These are non-negotiable. They override any local optimization.

1. **Reversible by default.** Any change must be undoable with `git revert` or a documented downgrade. Irreversible operations (data migrations, deletions, external API writes, force-pushes) require explicit human approval at the time of action, not in advance.
2. **Deterministic builds.** Same inputs, same outputs. Lockfiles committed. Bundled binaries pinned by version and checksum. CI must be reproducible on a fresh clone.
3. **Local first, network second.** The app functions offline. Network is a capability, not a dependency. Telemetry is opt-in and local-only unless the CEO explicitly approves a remote endpoint in an ADR.
4. **Compile-time over run-time.** Prefer types, schema checks, and static validation over runtime assertions. If the compiler can catch it, don't write a test for it.
5. **One source of truth per concern.** Configuration has one canonical path. Data has one canonical table. Types cross the IPC boundary once, generated. Don't hand-duplicate contracts.
6. **Small, observable commits.** Conventional Commits. One logical change per commit. Every commit passes the quality gate. No "fix earlier commit" at the end of a branch — amend or rebase.
7. **The quality gate is the API.** Code is not done until `cargo fmt --check && cargo clippy -D warnings && cargo test && pnpm typecheck && pnpm lint && pnpm test` is green. The Senior Engineer runs this before declaring a phase complete.

---

## 3. Repository Layout Principles

The repo is organized for **discoverability** and **isolation**, not for technical neatness. Someone arriving cold should find what they need in three clicks.

- `CLAUDE.md` — the orientation doc. Short. Points out.
- `.claude/` — the workforce definition. Agents, hooks, skills, rules, settings.
- `docs/` — everything a human or an agent should read before touching the code.
- `src-tauri/` — Rust backend. Layered: `commands` (thin), `services` (orchestration), `domain` (pure), `infra` (external).
- `src/` — React frontend. Feature-folder organization, not type-folder.
- `scripts/` — one-shot bash/ps1 automation. Documented in README.
- `.github/` — CI, CD, issue templates.

Anti-pattern: a `utils/` folder at the root. Utilities belong to a feature or a layer. If nothing owns it, it probably doesn't need to exist.

---

## 4. The CLAUDE.md Hierarchy

Claude Code loads CLAUDE.md from every ancestor directory up to the project root, plus the user-level one at `~/.claude/CLAUDE.md`. Use this intentionally.

- **`CLAUDE.md` (repo root)** — stack, commands, conventions, active decisions with links to ADRs, key file paths, escalation. **≤120 lines.** If you need more, split it into a topic-scoped rule under `.claude/rules/`.
- **`CLAUDE.local.md`** — per-developer overrides, untracked. Committed as `.example`.
- **Nested `CLAUDE.md`** — use sparingly, for directories with rules that are truly local (e.g., generated code that must not be hand-edited). Do not use as a dumping ground.

Rules in `.claude/rules/` with a `glob:` frontmatter target specific file patterns and load only when relevant files are in play. Prefer these over piling onto the root CLAUDE.md.

---

## 5. Synthetic Workforce Patterns

### 5.1 CLAUDE.md template (root)

```markdown
# CLAUDE.md

## Project
<one-line pitch>. <one-line status>.

## Stack
<bullets: primary languages, frameworks, runtime>

## Commands
- `<cmd>` — <what it does>
...

## Conventions
- <rule>
...

## Active decisions
- [ADR-0001](docs/adr/0001-*.md) — <title>
...

## Key paths
- <path> — <purpose>
...

## When in doubt
<where to look, whom to escalate to>
```

### 5.2 Subagent definitions (`.claude/agents/<name>.md`)

Each subagent is a markdown file with YAML frontmatter. Keep the body under 60 lines — agents are invoked for narrow tasks.

```markdown
---
name: <slug>
description: <one sentence — the router uses this to decide when to invoke>
model: haiku | sonnet | opus
tools: [Read, Grep, Glob, Bash, Edit, Write]   # minimal set
---

# <Title>

## Responsibility
<2–4 sentences>

## Invocation signal
<when the Senior Engineer should delegate to you>

## Process
1. ...
2. ...

## Output format
<what you return — structured, predictable>

## Out of scope
<what you must not do>
```

Patterns:
- **Reviewers** (code-reviewer, security-reviewer) — read-only; return structured findings.
- **Specialists** (rust-specialist, frontend-specialist) — opus-tier, deep reasoning on narrow stacks.
- **Workers** (qa-tester, docs-writer, researcher) — haiku or sonnet, high-throughput.

### 5.3 Hooks (`.claude/hooks/*.sh` + `.claude/settings.json`)

Three categories:

1. **PreToolUse guards** — block destructive operations before they happen. Bash firewall, protected-path guard, env-file guard. Exit non-zero to deny.
2. **PostToolUse formatters** — auto-format on write. `rustfmt` for `.rs`, `prettier` for `.ts`/`.tsx`/`.md`/`.json`. Silent on success, stderr on failure.
3. **Stop gate** — run the fast part of the quality gate at session end (`cargo check`, `pnpm typecheck`). Use `stop_hook_active` to avoid infinite loops.

Hook scripts must be idempotent, fast (<2s where possible), and never modify files silently outside their documented scope.

### 5.4 MCP servers (`.mcp.json`)

Keep lean. More servers means more tool-name collisions and more context cost. Three is plenty for most phases:

- `filesystem` — scoped to the repo root. Read/write inside the repo only.
- `git` — inspect history. Pair with direct `gh` via Bash for GitHub ops (cheaper than an MCP).
- `github` — only when actively managing issues/PRs. Gate on `GITHUB_TOKEN`.

Anti-pattern: plugging in every available MCP "just in case." Each unused tool eats context on every turn.

### 5.5 Skills (`.claude/skills/<name>/SKILL.md`)

Skills encode a repeatable playbook. The Senior Engineer invokes them by name (`/skill-name`) rather than remembering the steps.

A good skill has:
- A **clear trigger** in the frontmatter `description`.
- **Inputs** it expects (file paths, identifiers).
- A **deterministic process** (numbered steps).
- **Validation** at the end (what "done" looks like).
- **Examples** of correct use.

Skills are not documentation. If you find yourself writing prose, move it to `docs/` and link.

### 5.6 Rules (`.claude/rules/*.md`)

Topic-scoped guidance that loads only when matching files are touched. Frontmatter:

```markdown
---
description: <one line>
glob: "src-tauri/**/*.rs"
---
```

Keep each file under 80 lines. If a rule needs more, it's a design doc and belongs in `docs/`.

---

## 6. Delegation Patterns

The Senior Engineer is the orchestrator. Use subagents when:

- **Parallelism helps.** Two independent doc writes, or research on three libraries — spawn in parallel.
- **Context isolation helps.** Deep code review of 2000 lines in a subagent keeps the main session lean.
- **Specialization helps.** A tricky async-Rust question → `rust-specialist`. A11y audit → `frontend-specialist`.

Do **not** use subagents for:

- A task you've already done the lookup for (duplicates work, wastes tokens).
- Short, single-file edits where launch overhead exceeds the edit.
- Anything where you need to think step-by-step with tool results in your own context.

### Prompting a subagent

Treat it like a new senior hire: three lines of context, one concrete ask, one output shape. Include:
- The problem in plain language.
- What you've ruled out.
- The exact output format (structured if you'll machine-read it).

Bad: "Review this code."
Good: "Review `src-tauri/src/services/poll.rs` for async correctness. I'm concerned about the select! macro around line 120 holding a mutex across an await. Return: list of issues with line numbers, each tagged P0/P1/P2."

---

## 7. Phase Gating

A phase ends when:

1. Every item in the phase's acceptance criteria is checked.
2. The quality gate is green on a clean checkout.
3. A session report exists in `docs/session-reports/phase-NN.md` covering deliverables, deviations, and open questions.
4. The CTO has reviewed the session report.

If any item fails, the phase is not complete. The Senior Engineer surfaces the blocker; the CTO decides whether to extend, re-scope, or cut.

---

## 8. Anti-patterns

Observed in the wild; explicitly banned here:

- **Prompt sprawl.** CLAUDE.md grows past 200 lines because someone kept appending. Split into rules.
- **Subagent theater.** Invoking a subagent to do a one-line grep. You have Grep.
- **Silent formatters.** A PostToolUse hook that rewrites unrelated files. Format only what was written.
- **Lockfile churn.** Regenerating `Cargo.lock` / `pnpm-lock.yaml` on every commit. Lockfiles are committed and stable.
- **Unpinned sidecars.** yt-dlp auto-updates are allowed behind a version-pin fallback; they are **not** allowed as unbounded `latest`.
- **Hand-duplicated types across IPC.** Rust and TS types for the same command must be generated from one source.
- **`unwrap()` in production paths.** Tests only. Elsewhere, `?` or explicit `match`.
- **TODO comments as a backlog.** Open an issue. The comment rots; the issue gets triaged.

---

## 9. Communication Discipline

- **Commits** are Conventional Commits: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `build:`, `ci:`.
- **ADRs** are numbered, immutable once merged, superseded rather than edited.
- **Session reports** are short (under 200 lines) and follow the template: Delivered / Deviated / Deferred / Open Questions / Next-Phase Readiness.
- **Open questions** go to the session report. Don't leave them as TODOs in code.

---

## 10. When this blueprint is wrong

It's a living document. If a pattern here is slowing you down or producing bad outcomes, open an ADR proposing a change. Do **not** silently deviate. Silent deviation is how operating models rot.

— End of blueprint —
