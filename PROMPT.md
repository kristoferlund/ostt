You are implementing Phase 1 local model support for OSTT.

Paths:
- Plan: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/PLAN.md
- Session notes: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/SESSION.md
- Spec folder: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing
- Target codebase: /Users/kristoferlund/gh/ostt

Read PLAN.md first. Find the next incomplete section or sub-section: the first section/sub-section containing unchecked tasks. That section/sub-section is your entire scope for this session. STOP after completing that one section/sub-section. Do not continue to the next section/sub-section.

Read the spec file for the selected section:
- Spec 1.1: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/1.1 - Local Provider Variant + Feature Flag.md
- Spec 1.2: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/1.2 - whisper-rs Integration & Model Loading.md

Read SESSION.md if it exists. Use previous notes as context. Append to SESSION.md at the end; do not overwrite it.

Study the relevant source files in /Users/kristoferlund/gh/ostt before editing. Read exports, immediate callers, and shared utilities needed for the current section. Do not do unrelated refactors.

Implement tasks in exact checklist order. No skipping tasks. No reordering. SCOPE is one section/sub-section only.

CRITICAL crash-recovery rule: update PLAN.md IMMEDIATELY after completing each task by changing that task from `- [ ]` to `- [x]` before starting the next task. Do not batch PLAN.md updates.

Verification:
- Run the verification command tasks in order when reached.
- If a verification command fails, make a focused fix within the current section scope and rerun it once.
- If the same verification command fails twice, change that task marker in PLAN.md from `- [ ]` to `- [!]`, append a SESSION.md summary describing the failure, git commit partial work, and stop.

File modification restrictions:
- You may modify files only under /Users/kristoferlund/gh/ostt, PLAN.md, and SESSION.md.
- Do not modify files outside the target codebase except PLAN.md and SESSION.md.
- Do not implement deferred items: GPU acceleration, config-driven whisper parameters, progress callbacks, or streaming.

Session notes:
- Append a summary to /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/SESSION.md.
- Use heading `## Session N: Spec X.Y — <title>` and increment N from existing headings.
- Include: what was accomplished, obstacles encountered, out-of-scope observations.

Git:
- Before stopping, git commit all changes from this session.
- Use a concise commit message matching repository style.
- Do not push.

Stop condition:
- Stop after one section/sub-section is complete and committed, or after the failure protocol is committed.
