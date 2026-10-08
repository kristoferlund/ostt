# Contributing to OSTT

Thank you for contributing. OSTT is a terminal-native speech-to-text tool for
Linux and macOS, with cloud providers, local inference, and scriptable output
and processing workflows.

## An issue is required before implementation

**You must open an issue before starting implementation and before submitting
a pull request**, unless your change qualifies for the small-change exception
below. If a relevant issue already exists, use that issue rather than creating
a duplicate.

For non-trivial changes, wait for a maintainer to agree on the scope and approach
before coding. This includes:

- New features, commands, options, providers, or models.
- Changes to existing behavior, defaults, or configuration.
- Refactors and dependency changes.
- Build, release, installer, or packaging changes.

The issue should describe:

- The problem and who it affects.
- Reproduction steps for bugs, including OSTT version and platform.
- The proposed solution and any compatibility implications.
- How you plan to verify the change.

An issue is a place to discuss the proposal, not automatic approval to implement
it. Agreement on an approach also does not guarantee that a pull request will
be merged.

### Small-change exception

You may submit a pull request without first opening an issue for:

- A small, clearly scoped bug fix that restores intended behavior without
  changing interfaces, defaults, or configuration.
- A typo, broken link, or minor documentation correction.

Explain in the pull request why the exception applies. A short diff is not
automatically a small change: features, refactors, dependency updates, and
build or packaging changes still require an issue.

If you are unsure, open an issue first. If the work grows beyond the exception,
stop and discuss it in an issue before continuing.

Non-exempt pull requests that bypass this process may be closed without review.

## Keep pull requests focused

- Address one agreed problem per pull request.
- Link the issue and explain what changed and why.
- Avoid unrelated cleanup, formatting, or refactoring.
- Follow existing code structure and conventions.
- Describe changes to CLI behavior, configuration, dependencies, and platform
  support explicitly.
- Never include API keys, credentials, private recordings, or sensitive
  transcripts in code, fixtures, logs, or screenshots.

## Development and verification

Use the stable Rust toolchain. Native dependencies vary by platform and backend.

For a baseline Linux build on Debian/Ubuntu, install:

    sudo apt-get install libasound2-dev libssl-dev cmake pkg-config

On macOS, install:

    brew install cmake pkg-config openssl@3 ffmpeg

Runtime dependencies such as ffmpeg and clipboard utilities may also be needed
to exercise the workflows affected by your change.

For code changes, run from the repository root:

    cargo fmt --check
    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings
    cargo build --profile dist --locked

Test affected optional backends separately using the relevant feature:
`whisper-cuda` or `whisper-vulkan`.

Do not use `--all-features` as the standard contributor check. GPU features
require different native toolchains and runtimes; contributors are not expected
to have every backend available.

### Verify the affected behavior

- Add or update automated tests for code changes. Bug fixes should include a
  regression test that fails without the fix where practical.
- Manually exercise the affected user workflow. Automated tests alone do not
  validate microphone capture, terminal interaction, clipboard integration,
  hotkeys, or real transcription.
- For provider changes, test request construction, response handling, parameter
  validation, and relevant failures. State whether a live API request was tested.
- For local inference or GPU changes, identify the model, feature flags,
  hardware, and runtime tested. Compilation or backend registration alone is
  not evidence that GPU inference works.
- For shared or platform-specific changes, report which Linux/macOS environments
  were tested and which remain unverified.
- For build or packaging changes, verify the affected artifacts and their
  runtime dependencies. See RELEASE.md for the release process; contributors
  are not expected to publish releases.

Report the commands run, results, and manual verification in the pull request.
Explicitly identify any failed, skipped, or unavailable checks and explain why.
Do not describe untested behavior as verified.

Documentation-only changes do not require Rust builds or tests. Review the
rendered Markdown, links, and any commands or technical claims you change.

## Documentation and changelog

Update documentation and CLI help when behavior changes. Public documentation
is maintained separately at https://ostt.ai; identify any required website
updates in the pull request.

Add a concise entry under `[Unreleased]` in CHANGELOG.md for user-visible changes.
Version bumps, release tags, and publishing are handled by maintainers.

## Before submitting

Complete the pull request template. Required CI checks must pass before merge,
and any outstanding verification gaps must be resolved with a maintainer.
