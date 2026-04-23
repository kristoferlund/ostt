# OSTT — Specs

Feature specifications and implementation plans for OSTT.

Each feature lives in its own subfolder. Inside a feature folder, the work is broken down into phases and specs, with a `PLAN.md` + `SESSION.md` for each phase that an agent uses to implement the feature incrementally.

## Features

| Feature | Status | Description |
|---------|--------|-------------|
| [process-command](process-command/README.md) | Complete | Post-processing pipeline — `process` subcommand and `-p` flag for transforming transcriptions with bash or AI actions. |

## Starting a New Feature

1. Copy the template folder:

   ```bash
   cp -r specs/_template specs/<your-feature-name>
   ```

2. Follow the instructions in `specs/_template/README.md` to write specs and generate an implementation plan.

3. Add an entry to the **Features** table above once the feature folder exists.

## Folder Layout

```
specs/
├── README.md                # This file — index of all features
├── _template/               # Template for new features (see its README)
└── <feature-name>/          # One folder per feature
    ├── README.md            # Feature overview and phase roadmap
    ├── Phase N/
    │   ├── PLAN.md          # Implementation plan (checklist)
    │   ├── SESSION.md       # Append-only session notes
    │   └── N.M — <Spec>.md  # Individual spec files
    └── ...
```
