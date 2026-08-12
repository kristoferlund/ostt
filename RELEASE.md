# OSTT Release Runbook

This file describes how to go from merged code changes to a finished OSTT release across GitHub Releases, Homebrew, the documentation site, and all AUR packages.

Assumptions:

- Run commands from the main repository at `../ostt` unless another path is shown.
- The documentation repository is checked out at sibling path `../ostt-web`.
- The AUR repositories are checked out at sibling paths `../aur-ostt`, `../aur-ostt-bin`, `../aur-ostt-cuda-bin`, and `../aur-ostt-vulkan-bin`.
- The Homebrew tap is updated by the GitHub release workflow.

## Release Targets

Every release must update or verify these targets:

- GitHub Release: `https://github.com/kristoferlund/ostt/releases/tag/v<VERSION>`
- Homebrew tap: `kristoferlund/homebrew-ostt`
- Documentation changelog: `../ostt-web/guide/changelog.md`
- Install manifest: `../ostt-web/public/latest.json`
- AUR source package: `../aur-ostt`
- AUR CPU binary package: `../aur-ostt-bin`
- AUR CUDA binary package: `../aur-ostt-cuda-bin`
- AUR Vulkan binary package: `../aur-ostt-vulkan-bin`

## 1. Prepare Main Release

Choose the next semantic version. For a patch release after `0.0.20`, use `0.0.21` and tag `v0.0.21`.

Update files in `../ostt`:

- `Cargo.toml`: set `[package].version` to `<VERSION>`
- `Cargo.lock`: update the `ostt` package version to `<VERSION>`
- `CHANGELOG.md`: move `[Unreleased]` entries into `## <VERSION> - <YYYY-MM-DD>`

Keep an empty `## [Unreleased]` section at the top of `CHANGELOG.md`.

## 2. Update Documentation Changelog

Update the public documentation changelog in the sibling website repository:

```bash
cd ../ostt-web
```

Edit:

- `guide/changelog.md`

Add the same release notes that were moved into `../ostt/CHANGELOG.md`.

If this is only a changelog edit, no new page is being added. Still do a targeted website diff review before committing:

```bash
git diff -- guide/changelog.md .vitepress/config.mts public/llms.txt public/robots.txt
```

Ask before running `pnpm docs:build`; full website builds require approval in that workspace.

Commit and push the docs changelog update:

```bash
git status --short --branch
git diff -- guide/changelog.md
git add guide/changelog.md
git commit -m "Update changelog for <VERSION>"
git push
```

## 3. Verify Main Release Locally

Run release verification in `../ostt`:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --profile dist --locked
```

Do not continue if any command fails.

## 4. Commit And Tag Main Release

Inspect before committing:

```bash
git status --short --branch
git diff -- CHANGELOG.md Cargo.toml Cargo.lock
git log --oneline -10
```

Commit and tag:

```bash
git add CHANGELOG.md Cargo.toml Cargo.lock
git commit -m "Release v<VERSION>"
git tag v<VERSION>
```

Push the release commit and tag:

```bash
git push origin main
git push origin v<VERSION>
```

The tag push starts `.github/workflows/release.yml` through cargo-dist.

## 5. Watch GitHub Release Workflow

Find and watch the release workflow:

```bash
gh run list --workflow Release --limit 5
gh run watch <RUN_ID> --exit-status
```

The workflow must complete successfully. It is expected to:

- Build CPU binaries for Linux and macOS
- Build CUDA and Vulkan Linux binaries
- Build `.deb` and `.rpm` packages
- Upload release assets
- Create the GitHub Release
- Publish the Homebrew formula to `kristoferlund/homebrew-ostt`

Verify the release exists:

```bash
gh release view v<VERSION> --json url,name,tagName,isDraft,isPrerelease,publishedAt
```

Do not update the binary AUR packages until the GitHub release assets exist.

## 6. Refresh The Install Manifest

The one-line installer at `https://ostt.ai/install` resolves download URLs and
checksums from `https://ostt.ai/latest.json`. That file is a static artifact in
the website repository and does **not** update itself when a release is
published. If this step is skipped, the installer keeps installing the previous
version.

The installer fails with an explicit error if the manifest is unreachable; it
does not silently fall back to guessing filenames. It also compares the
manifest version against the newest GitHub release and warns loudly when they
disagree, so a skipped refresh shows up in user-visible output rather than
quietly serving an old release.

Run this only after step 5 has confirmed the release assets exist.

```bash
cd ../ostt-web
node scripts/gen-manifest.mjs
```

The generator reads the latest GitHub release and derives every URL and SHA-256
from the published assets, so it never requires a new release to run. It exits
non-zero if the release is missing a target the installer knows how to request —
that means the release itself is incomplete, so fix that before continuing.

Set `GITHUB_TOKEN` if you hit the unauthenticated API rate limit:

```bash
GITHUB_TOKEN="$(gh auth token)" node scripts/gen-manifest.mjs
```

To regenerate for a specific tag instead of the latest release, pass it:

```bash
node scripts/gen-manifest.mjs v<VERSION>
```

Confirm the manifest points at the new version:

```bash
git diff -- public/latest.json
grep '"version"' public/latest.json
```

Commit and push:

```bash
git add public/latest.json
git commit -m "Update install manifest for <VERSION>"
git push
```

Wait for ostt.ai to redeploy, then verify the live file:

```bash
curl -fsSL https://ostt.ai/latest.json | grep '"version"'
```

This must report `<VERSION>`. Do not continue until it does.

## 7. Collect AUR Checksums

Create a temporary checksum directory:

```bash
mkdir -p /tmp/opencode/aur-ostt-<VERSION>
cd /tmp/opencode/aur-ostt-<VERSION>
```

Download release sources and binary archives:

```bash
curl -L -o source-archive.tar.gz \
  "https://github.com/kristoferlund/ostt/archive/refs/tags/v<VERSION>.tar.gz"

curl -L -o ostt-x86_64-unknown-linux-gnu.tar.gz \
  "https://github.com/kristoferlund/ostt/releases/download/v<VERSION>/ostt-x86_64-unknown-linux-gnu.tar.gz"

curl -L -o ostt-aarch64-unknown-linux-gnu.tar.gz \
  "https://github.com/kristoferlund/ostt/releases/download/v<VERSION>/ostt-aarch64-unknown-linux-gnu.tar.gz"

curl -L -o ostt-x86_64-unknown-linux-gnu-cuda.tar.gz \
  "https://github.com/kristoferlund/ostt/releases/download/v<VERSION>/ostt-x86_64-unknown-linux-gnu-cuda.tar.gz"

curl -L -o ostt-x86_64-unknown-linux-gnu-vulkan.tar.gz \
  "https://github.com/kristoferlund/ostt/releases/download/v<VERSION>/ostt-x86_64-unknown-linux-gnu-vulkan.tar.gz"
```

Compute checksums:

```bash
sha256sum source-archive.tar.gz
sha256sum ostt-x86_64-unknown-linux-gnu.tar.gz
sha256sum ostt-aarch64-unknown-linux-gnu.tar.gz
sha256sum ostt-x86_64-unknown-linux-gnu-cuda.tar.gz
sha256sum ostt-x86_64-unknown-linux-gnu-vulkan.tar.gz
```

## 8. Update AUR Packages

Update each sibling AUR repository.

For `../aur-ostt`:

- Set `pkgver=<VERSION>` in `PKGBUILD`
- Set `sha256sums` to the source archive checksum

For `../aur-ostt-bin`:

- Set `pkgver=<VERSION>` in `PKGBUILD`
- Set `sha256sums_x86_64` to the CPU x86_64 archive checksum
- Set `sha256sums_aarch64` to the CPU aarch64 archive checksum

For `../aur-ostt-cuda-bin`:

- Set `pkgver=<VERSION>` in `PKGBUILD`
- Set the first `sha256sums` entry to the CUDA archive checksum

For `../aur-ostt-vulkan-bin`:

- Set `pkgver=<VERSION>` in `PKGBUILD`
- Set the first `sha256sums` entry to the Vulkan archive checksum

Regenerate `.SRCINFO` in each AUR repository:

```bash
makepkg --printsrcinfo > .SRCINFO
```

## 9. Verify AUR Packages

Run source verification in each AUR repository, using the temporary source cache:

```bash
SRCDEST="/tmp/opencode/aur-ostt-<VERSION>" makepkg --verifysource
```

For `../aur-ostt-bin`, also verify the aarch64 archive explicitly on an x86_64 machine:

```bash
CARCH=aarch64 SRCDEST="/tmp/opencode/aur-ostt-<VERSION>" makepkg --verifysource
```

Inspect each AUR diff before committing:

```bash
git status --short --branch
git diff -- PKGBUILD .SRCINFO
```

## 10. Commit And Push AUR Packages

Commit in each AUR repository:

```bash
git add PKGBUILD .SRCINFO
git commit -m "Update to <VERSION>"
```

Push `../aur-ostt` to AUR:

```bash
git push origin master
```

Push the three binary package repositories to AUR and their GitHub mirrors:

```bash
git push origin master
git push github master
```

Do this for:

- `../aur-ostt-bin`
- `../aur-ostt-cuda-bin`
- `../aur-ostt-vulkan-bin`

## 11. Confirm Final State

Confirm the main release:

```bash
cd ../ostt
git status --short --branch
gh release view v<VERSION> --json url,name,tagName,publishedAt
```

Confirm the website repo is clean:

```bash
cd ../ostt-web
git status --short --branch
```

Confirm the live install manifest serves the new version:

```bash
curl -fsSL https://ostt.ai/latest.json | grep '"version"'
```

Confirm every AUR repository is clean and pushed:

```bash
cd ../aur-ostt && git status --short --branch
cd ../aur-ostt-bin && git status --short --branch
cd ../aur-ostt-cuda-bin && git status --short --branch
cd ../aur-ostt-vulkan-bin && git status --short --branch
```

The AUR web UI and RPC metadata can lag behind Git pushes. If the package page still shows the old version immediately after pushing, verify the AUR Git remote first:

```bash
git fetch origin master
git rev-parse HEAD
git rev-parse origin/master
```

If `HEAD` and `origin/master` match the new release commit, wait for AUR metadata to refresh before making any extra commits.
