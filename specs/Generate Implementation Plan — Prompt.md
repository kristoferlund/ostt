# Generate Implementation Plan — Reusable Prompt

Use this prompt with a coding agent (OpenCode, Claude Code, etc.) to convert a set of spec files into a structured implementation plan with an embedded session prompt for iterative execution.

## Prerequisites

Before using this prompt, you need:

1. **A folder of spec files** -- markdown files describing what to implement, each with acceptance criteria. They can be numbered, phased, or just a flat set.
2. **A target codebase** -- the repo/project where the implementation will happen.

## The Prompt

Copy everything between the `---` markers and paste it into a new agent session. 

---

```
I have a set of specification files that I want to turn into a structured implementation plan.

Instructions:

1. Read every spec file in the specs folder.
2. Read the target codebase to understand its current structure, files, and patterns.
3. Analyze dependencies between specs — which specs must be completed before others.
4. Determine an execution order: start with the smallest, most isolated changes first, then build toward larger changes. Specs with dependencies on other specs go after their dependencies.
5. Decompose each spec into atomic implementation tasks. Each task should be:
   - Small enough for one focused action (create a type, modify a function, add a test)
   - Ordered within its spec group (types before logic, logic before tests)
   - Verifiable (ends with a concrete check: cargo check, cargo test, cargo clippy, etc.)
6. Include verification tasks within each spec group (e.g., "Verify: cargo test passes").

Write PLAN.md with the following structure:

## Required PLAN.md Structure

### Header
- Title (e.g., "Phase 1 — Implementation Plan")
- Scope (which specs are covered)
- Target codebase path
- Status: Not started

### Dependency Order
- Text or diagram showing which specs depend on which
- Brief rationale for the recommended execution order

### Tasks
- One section per spec, in execution order (not necessarily spec number order)
- IMPORTANT: Each section (or sub-section) can have AT MOST 10 tasks. If a spec has more than 10 tasks, split it into sub-sections (e.g., 2.1.A, 2.1.B, 2.1.C). Each sub-section is the unit of work for one session — the agent completes one sub-section, commits, and stops.
- Each section/sub-section contains a checklist of atomic tasks with `- [ ]` checkboxes
- Each task has a bold ID (e.g., **1.4.1**) and a concise description
- Specs that depend on others have a "Depends on: X.X" note
- Verification tasks (cargo check, cargo test, cargo clippy, etc.) are the last items in each section/sub-section

### Verification Protocol
- What commands to run after each spec is complete
- What commands to run after all specs are complete

### Session Boundaries
Explain:
- The unit of work per session is ONE section or sub-section (at most 10 tasks). If a spec was split into sub-sections (e.g., 2.1.A, 2.1.B), each sub-section is a separate session. The agent completes one, commits, and stops.
- When to stop early (repeated verification failure)
- The agent must git commit before stopping every time

### Session Prompt Template
Include a ready-to-paste prompt block that future sessions will use. The prompt must:
- Reference PLAN.md, SESSION.md and the spec files folder by absolute path
- Tell the agent to read the plan, find the next incomplete section or sub-section (the first one with unchecked tasks), read its spec file
- Tell the agent to study the relevant source files in the target codebase
- Tell the agent to read the notes from previous sessions
- Tell the agent to implement tasks in order, running verification after each
- CRITICAL: Tell the agent to update PLAN.md IMMEDIATELY after completing each task (change `- [ ]` to `- [x]`) BEFORE starting the next task. This must not be batched — it is essential for crash recovery so interrupted sessions don't redo completed work.
- Tell the agent to git commit all changes before stopping
- Tell the agent to STOP after one section/sub-section (at most 10 tasks) — do NOT continue to the next
- Include rules: no skipping tasks, no reordering, SCOPE is one section/sub-section only
- Include the failure protocol: if verification fails twice, mark task with [!], commit partial work, stop
- Restrict file modifications to the target codebase, PLAN.md, and SESSION.md only
- Tell the agent to APPEND (not overwrite) a session summary to the end of SESSION.md (in the same directory as PLAN.md). Use heading `## Session N: Spec X.Y — <title>` (increment N). Include: what was accomplished, obstacles encountered, out-of-scope observations.

Verification commands for this project (Rust):
- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`

### Important Rules for Plan Generation

- Do NOT start implementing. Only produce the plan.
- Do NOT modify any files in the target codebase.
- The only file you create is PLAN.md in the spec files folder.
- If a spec has items explicitly marked as "deferred" to a later phase, exclude those tasks from the plan but note the deferral.
- If a spec's acceptance criteria are vague, decompose conservatively — fewer tasks that are clearly defined is better than many ambiguous ones.
- Task IDs should match spec numbers (spec 1.3 → tasks 1.3.1, 1.3.2, etc.).
```

---

## After Running the Prompt

1. Review the generated PLAN.md — check that dependencies make sense, task granularity is right, and the session prompt paths are correct.
2. To start implementation, open a new agent session in the target codebase directory and paste the session prompt template from PLAN.md.
3. Each session does one section/sub-section (at most 10 tasks), commits, and stops. Repeat until all tasks are checked off.
