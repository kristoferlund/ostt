# Feature Specs Template

This folder is a template for structuring the specification and implementation plan for a new feature. Copy this entire folder, rename it to match your feature (e.g., `specs/my-new-feature/`), and fill in the details.

## How to Use This Template

### 1. Copy the template

```bash
cp -r specs/_template specs/<your-feature-name>
```

Use a short, kebab-case name for the feature folder (e.g., `process-command`, `audio-normalization`, `cloud-sync`).

### 2. Write a feature overview

Replace the contents of the copied `README.md` with a high-level description of the feature:

- What the feature does
- Why it exists (problem it solves)
- A "roadmap" table of phases with links to each spec and its PLAN.md (fill this in as you go)

See `specs/process-command/README.md` for a concrete example.

### 3. Write the specs

Create one markdown file per atomic unit of work. Group related specs into "Phase" folders when the feature is large enough to warrant phasing.

Recommended naming:

```
<feature-name>/
├── README.md
├── Phase 1/
│   ├── 1.1 — <Spec Title>.md
│   └── 1.2 — <Spec Title>.md
├── Phase 2/
│   └── 2.1 — <Spec Title>.md
└── ...
```

For small features, phases may be unnecessary — spec files can sit directly in the feature folder.

Each spec file should include:

- **Problem** — what's broken or missing
- **Objective** — what the spec achieves
- **Specification** — concrete types, function signatures, behaviors, file paths
- **Files Modified** — explicit list of paths the spec touches
- **Acceptance Criteria** — how to know it's done

See `specs/process-command/Phase 1/1.1 — Action Config Types.md` for a concrete example.

### 4. Generate the implementation plan

Once all spec files are written, use the reusable prompt in `_template/Generate Implementation Plan — Prompt.md` to have a coding agent convert the specs into a `PLAN.md` (one per phase) plus a ready-to-use session prompt.

The plan decomposes each spec into small checklist tasks and defines session boundaries so the implementation can be done incrementally with `./loop.sh`.

For larger or tightly coupled work, include small review sections after major clusters. Review sessions do not add new feature scope; they inspect prior session notes and recent commits, align existing helpers/patterns, make only small consistency fixes, and stop to ask before any larger redesign.

### 5. Execute the plan

Each session:

1. Reads the current phase's `PLAN.md` and `SESSION.md`.
2. Finds the first incomplete section (at most 10 tasks).
3. Reads recent commits and identifies helpers/patterns already introduced for the feature.
4. Implements those tasks, verifying after each.
5. Commits, appends a handoff summary to `SESSION.md`, and stops.

Repeat until all checkboxes are `[x]`.

## Folder Structure at a Glance

```
specs/
├── README.md               # Overview of all features
├── _template/              # This template (do not modify when starting a feature)
│   ├── README.md           # (this file)
│   └── Generate Implementation Plan — Prompt.md
├── process-command/        # First implemented feature (reference example)
│   ├── README.md
│   ├── Generate Implementation Plan — Prompt.md
│   ├── Phase 1/
│   │   ├── PLAN.md
│   │   ├── SESSION.md
│   │   └── 1.1 — <Spec>.md
│   └── ...
└── <your-feature>/         # Your new feature, based on _template
    └── ...
```

## Conventions

- **Do not create markdown files** outside of the `specs/` folder during implementation. Specs, plans, and session notes live here; code lives in `src/`.
- **SESSION.md is append-only.** Each session adds a new `## Session N: ...` heading with a summary.
- **SESSION.md is the handoff layer.** Summaries should include decisions made, files changed, helpers/APIs introduced or reused, verification results, constraints for later sessions, obstacles, open questions, and out-of-scope observations.
- **PLAN.md tracks progress** via `- [ ]` / `- [x]` checkboxes. Agents must update checkboxes immediately after finishing each task, not in batches.
- **One section per session.** A section or sub-section has at most 10 tasks. Agents stop and commit after completing one section.
- **Do not fork patterns.** If a previous session introduced a helper/API for the same concern, later sessions should reuse it unless they document why it is unsuitable.
