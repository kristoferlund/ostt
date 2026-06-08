# ostt Distribution and Installation Guide

This document explains how ostt is distributed and installed across different platforms and package managers.

## Repository Structure

```
ostt/
├── src/
│   ├── setup/mod.rs             # Setup module with embedded files
│   └── ...                      # Other source code
├── environments/                 # Platform-specific setup documentation
│   └── ostt.toml                # Configuration template embedded into binary
├── Cargo.toml                   # Rust package manifest
├── dist-workspace.toml          # cargo-dist configuration
└── README.md                    # Main documentation
```

**Note:** AUR packages are maintained in four separate AUR repositories (one per variant), not in this repository.

**Note:** The default configuration template is embedded into the binary at compile time using `include_str!()` and is automatically extracted on first run.

## Installation Methods

### 1. Shell Installer (Linux/macOS) - Recommended

```bash
curl -fsSL https://ostt.ai/install | bash
```

The website installer is the primary install path. It detects the platform, installs missing runtime dependencies where possible, downloads a release artifact, verifies its checksum, and installs `ostt`.

Then configure:
```bash
ostt auth  # Set up API credentials
ostt       # Start recording
```

### 2. Homebrew (macOS/Linux)

```bash
brew tap kristoferlund/ostt
brew install ostt
ostt auth  # Set up API credentials
```

### 3. AUR (Arch/Manjaro Linux)

Prebuilt binary packages (recommended — no compilation, no Rust toolchain). Pick the one that matches your hardware:

```bash
yay -S ostt-bin          # CPU build (x86_64, aarch64)
yay -S ostt-cuda-bin     # NVIDIA CUDA build (x86_64)
yay -S ostt-vulkan-bin   # AMD / Intel Vulkan build (x86_64)
```

These packages download the official release binary and install it directly. They conflict with each other and with the source package, so only one can be installed at a time.

To build from source instead:

```bash
yay -S ostt
```

`paru` works in place of `yay` for any of the above.

### 4. Debian/Ubuntu (.deb)

```bash
curl -sLO https://github.com/kristoferlund/ostt/releases/latest/download/ostt_latest_amd64.deb && sudo apt install ./ostt_latest_amd64.deb
```

Native packages are release artifacts for users who prefer OS package manager installation. They also give the website installer a distro-native artifact to use on supported Linux systems instead of manually installing the tarball.

### 5. Fedora/RHEL (.rpm)

```bash
sudo dnf install https://github.com/kristoferlund/ostt/releases/latest/download/ostt-latest.x86_64.rpm
```

### 6. openSUSE (.rpm)

```bash
sudo zypper install https://github.com/kristoferlund/ostt/releases/latest/download/ostt-latest.x86_64.rpm
```

### 7. Direct Binary Download (Linux/macOS)

Download pre-compiled binaries from [GitHub Releases](https://github.com/kristoferlund/ostt/releases):

```bash
# Available platforms:
# - ostt-x86_64-unknown-linux-gnu.tar.gz
# - ostt-aarch64-unknown-linux-gnu.tar.gz  
# - ostt-x86_64-apple-darwin.tar.gz
# - ostt-aarch64-apple-darwin.tar.gz

tar -xzf ostt-<platform>.tar.gz
cd ostt-<platform>
sudo cp ostt /usr/local/bin/
```

### 8. Compile from Source

```bash
git clone https://github.com/kristoferlund/ostt.git
cd ostt
cargo build --profile dist
sudo cp target/dist/ostt /usr/local/bin/
```

## Runtime Dependencies

ostt requires the following external tools:

### All Platforms
- **ffmpeg** - Audio format conversion

### macOS
- **pbcopy** - Clipboard support (built-in)

### Linux
- **wl-clipboard** (Wayland) OR **xclip** (X11) - Clipboard support
- **alsa-lib** - Audio capture

### Installation

**macOS (Homebrew):**
```bash
brew install ffmpeg  # pbcopy is built-in
```

**Linux (Debian/Ubuntu):**
```bash
sudo apt install ffmpeg wl-clipboard  # For Wayland
# OR
sudo apt install ffmpeg xclip          # For X11
```

**Linux (Arch):**
```bash
sudo pacman -S ffmpeg wl-clipboard     # For Wayland
# OR
sudo pacman -S ffmpeg xclip            # For X11
```

**Linux (Fedora):**
```bash
sudo dnf install ffmpeg wl-clipboard   # For Wayland
# OR
sudo dnf install ffmpeg xclip          # For X11
```

**Note:** Package managers (Homebrew, AUR) will automatically install most dependencies.

## Getting Started

After installation, configure your API credentials:

```bash
ostt auth
```

Then start recording and transcribing:

```bash
ostt
```

View command history:

```bash
ostt history
```

## Configuration

ostt follows the XDG Base Directory Specification:

- **Config:** `~/.config/ostt/ostt.toml` (auto-created)
- **Data:** `~/.local/share/ostt/` (credentials)
- **Logs:** `~/.local/state/ostt/ostt.log.*` (daily rotated)

## Automatic Setup

On first run, ostt automatically:
1. Creates `~/.config/ostt/ostt.toml` with default configuration

### Hyprland Integration

If you're using Hyprland, add a keybinding and window rules to your Hyprland config after first run:

```hyprland
# Keybinding for ostt (clipboard output)
bindd = ALT, SPACE, ostt, exec, ostt launch -c

# OSTT window rules
windowrule = float on, match:title ostt
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) (monitor_h*0.85), match:title ostt
```

Then reload your Hyprland configuration: `hyprctl reload`

This will launch ostt in a floating popup terminal window, centered horizontally and positioned near the bottom of the screen.

## Distribution Details

### cargo-dist Configuration

ostt uses [cargo-dist](https://github.com/axodotdev/cargo-dist) for building and distributing releases.

**Configuration files:**
- `dist-workspace.toml` - Main cargo-dist configuration
- `Cargo.toml` - Build profile and file inclusion settings
- `.github/workflows/release.yml` - Auto-generated CI workflow

**Build profile (`[profile.dist]` in Cargo.toml):**
- `lto = true` - Full link-time optimization
- `codegen-units = 1` - Better optimization
- `opt-level = "s"` - Optimize for binary size
- `strip = true` - Remove debug symbols

**Target platforms:**
- `aarch64-apple-darwin` (macOS ARM64/M-series)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-unknown-linux-gnu` (Linux ARM64)
- `x86_64-unknown-linux-gnu` (Linux x86_64)

### cargo-deb (.deb for Debian/Ubuntu)

ostt uses [cargo-deb](https://github.com/kornelski/cargo-deb) to build `.deb` packages automatically in CI.

**Configuration:** `[package.metadata.deb]` in `Cargo.toml`

**How it works:**
- `cargo build --profile dist --locked` compiles the binary
- `cargo deb --no-build` packages the pre-built binary into a `.deb`
- Output goes to `target/debian/`
- The package and checksum are uploaded as GitHub Release assets on every tagged release

**Dependencies declared in package:**
- `ffmpeg` (required)
- `wl-clipboard | xclip` (recommended)

### cargo-generate-rpm (.rpm for Fedora/RHEL/openSUSE)

ostt uses [cargo-generate-rpm](https://github.com/cat-in-136/cargo-generate-rpm) to build `.rpm` packages automatically in CI.

**Configuration:** `[package.metadata.generate-rpm]` in `Cargo.toml`

**How it works:**
- `cargo build --profile dist --locked` compiles the binary
- `cargo generate-rpm` packages the pre-built binary into an `.rpm`
- Output goes to `target/generate-rpm/`
- The package and checksum are uploaded as GitHub Release assets on every tagged release

**Dependencies declared in package:**
- `ffmpeg` (required)
- `wl-clipboard` (recommended)

### PKGBUILD (AUR)

Four AUR packages are published for each release:

#### `ostt` (source build)

Compiles from source using `cargo`. Slower install but does not depend on pre-built binaries.

**Dependencies:**
- `alsa-lib` - Audio capture
- `openssl` - TLS for API calls
- `ffmpeg` - Audio format conversion

**Optional dependencies:**
- `wl-clipboard` - Clipboard support on Wayland
- `xclip` - Clipboard support on X11

#### `ostt-bin` (prebuilt CPU binary)

Downloads the official CPU release binary from GitHub. Supported on `x86_64` and `aarch64`.

**Dependencies:** `glibc`, `gcc-libs`, `openssl`, `alsa-lib`, `ffmpeg`
**Provides/Conflicts:** `ostt`

#### `ostt-cuda-bin` (prebuilt NVIDIA CUDA binary)

Downloads the official CUDA release binary from GitHub. Supported on `x86_64` only.

**Dependencies:** `glibc`, `gcc-libs`, `openssl`, `alsa-lib`, `ffmpeg`, `cuda`, `nvidia-utils`
**Provides/Conflicts:** `ostt`

#### `ostt-vulkan-bin` (prebuilt AMD/Intel Vulkan binary)

Downloads the official Vulkan release binary from GitHub. Supported on `x86_64` only.

**Dependencies:** `glibc`, `gcc-libs`, `openssl`, `alsa-lib`, `ffmpeg`, `vulkan-icd-loader`
**Provides/Conflicts:** `ostt`

### Homebrew Formula (ostt.rb)

Downloads pre-built binaries and installs:
- Binary: `$(brew --prefix)/bin/ostt` (with embedded config files)
- Documentation: `$(brew --prefix)/share/doc/ostt/`

**Dependencies:**
- `openssl` - TLS for API calls
- `ffmpeg` - Audio format conversion
- `alsa-lib` - Audio capture (Linux only)

**Note:** macOS users get `pbcopy` built-in. Linux users need to manually install `wl-clipboard` or `xclip`.

## Homebrew Distribution

ostt uses a **multi-tier Homebrew strategy**:

### Tier 1: Personal Tap (Current) ✓

Users install via:
```bash
brew tap kristoferlund/ostt
brew install ostt
```

**Setup (one-time):**
1. Create GitHub repo: `kristoferlund/homebrew-ostt`
2. Add GitHub Actions secret: `HOMEBREW_TAP_TOKEN` with permissions to push
3. On release, cargo-dist automatically updates the tap

**How it works:**
- cargo-dist generates a Homebrew formula
- Automatically pushes to `kristoferlund/homebrew-ostt`
- Users get updates via `brew upgrade`

### Tier 2: Homebrew Core (Future Goal) 🎯

Once ostt is established (75+ GitHub stars, stable release history):

1. Submit PR to `homebrew/homebrew-core`
2. Homebrew maintainers review
3. Once merged, users can: `brew install ostt` (no tap needed)

**Requirements for Homebrew Core:**
- Project notability (stars/forks/watchers)
- Clean release history
- Meets Homebrew quality guidelines
- Active maintenance

### Current Status

- ✓ Manual formula: `ostt.rb` (maintained in this repo)
- ✓ Auto-publishing configured: `dist-workspace.toml`
- ⏳ Tap repo: Create `kristoferlund/homebrew-ostt` when ready
- ⏳ Homebrew Core: Submit when project gains traction

## AUR (Arch User Repository) Distribution

Unlike Homebrew Core, **anyone can publish to the AUR**. ostt publishes **four AUR packages** per release:

| Package | Build | Architecture | GPU Support |
|---------|-------|-------------|-------------|
| `ostt` | Source (cargo) | x86_64, aarch64 | CPU only |
| `ostt-bin` | Prebuilt binary | x86_64, aarch64 | CPU only |
| `ostt-cuda-bin` | Prebuilt binary | x86_64 | NVIDIA CUDA |
| `ostt-vulkan-bin` | Prebuilt binary | x86_64 | AMD/Intel Vulkan |

All binary packages (`*-bin`) conflict with each other and with the source `ostt` package — only one can be installed at a time.

### Local Repository Structure

The AUR packages are maintained in four sibling directories:

```
..
├── ostt/                    # Main ostt repository
├── aur-ostt/                # AUR: ostt (source build)
├── aur-ostt-bin/            # AUR: ostt-bin (prebuilt CPU)
├── aur-ostt-cuda-bin/       # AUR: ostt-cuda-bin (prebuilt CUDA)
└── aur-ostt-vulkan-bin/     # AUR: ostt-vulkan-bin (prebuilt Vulkan)
```

### Initial AUR Setup (One-Time Per Package)

1. **Create AUR account:**
   - Go to https://aur.archlinux.org/register
   - Add your SSH public key to your account

2. **Request each package name** on the AUR (e.g., `ostt-bin`, `ostt-cuda-bin`, `ostt-vulkan-bin`).

3. **Clone each AUR repository:**
   ```bash
   git clone ssh://aur@aur.archlinux.org/ostt.git aur-ostt
   git clone ssh://aur@aur.archlinux.org/ostt-bin.git aur-ostt-bin
   git clone ssh://aur@aur.archlinux.org/ostt-cuda-bin.git aur-ostt-cuda-bin
   git clone ssh://aur@aur.archlinux.org/ostt-vulkan-bin.git aur-ostt-vulkan-bin
   ```

4. **Create a PKGBUILD file in each** (see examples in each repo).

5. **Publish to AUR:**
   ```bash
   git add PKGBUILD .SRCINFO
   git commit -m "Initial release: ostt-bin 0.0.20"
   git push
   ```

### Updating AUR Packages

After each release, all four packages must be updated. Manual steps for each package:

#### Manual Update (per package)

```bash
cd aur-ostt-bin

# Update version in PKGBUILD
sed -i 's/pkgver=.*/pkgver=0.0.21/' PKGBUILD

# Update checksums (or use sha256sums=('SKIP') for simplicity)
updpkgsums

# Regenerate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Update to 0.0.21"
git push
```

Repeat for `aur-ostt`, `aur-ostt-cuda-bin`, and `aur-ostt-vulkan-bin`.

### Scripted Publishing (Planned)

A helper script at `scripts/update-aur.sh` can update all four AUR packages in one pass. It expects the AUR repositories to be cloned as sibling directories (`../aur-ostt`, `../aur-ostt-bin`, `../aur-ostt-cuda-bin`, `../aur-ostt-vulkan-bin`). This script does not exist yet — manual updates are required for now.

## Release Process

1. **Update version** in `Cargo.toml`
2. **Build and test locally:**
   ```bash
   cargo build --profile dist
   cargo test
   cargo clippy
   ```
4. **Create and push tag:**
   ```bash
   git tag v0.0.2
   git push origin v0.0.2
   ```
5. **GitHub Actions automatically:**
   - Builds for all target platforms (CPU, CUDA, Vulkan)
   - Creates GitHub release
   - Uploads binaries and installer script
   - Generates Homebrew formula
   - Publishes to `kristoferlund/homebrew-ostt` (if tap repo exists)
   - Builds and uploads `.deb` (Debian/Ubuntu) and `.rpm` (Fedora/RHEL/openSUSE) packages for CPU, CUDA, and Vulkan variants
6. **Manually update all four AUR packages** (until automated):
   ```bash
   # Update source package
   cd ../aur-ostt
   sed -i 's/pkgver=.*/pkgver=0.0.21/' PKGBUILD
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO
   git commit -m "Update to 0.0.21"
   git push

   # Update prebuilt CPU binary package
   cd ../aur-ostt-bin
   sed -i 's/pkgver=.*/pkgver=0.0.21/' PKGBUILD
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO
   git commit -m "Update to 0.0.21"
   git push

   # Update prebuilt CUDA binary package
   cd ../aur-ostt-cuda-bin
   sed -i 's/pkgver=.*/pkgver=0.0.21/' PKGBUILD
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO
   git commit -m "Update to 0.0.21"
   git push

   # Update prebuilt Vulkan binary package
   cd ../aur-ostt-vulkan-bin
   sed -i 's/pkgver=.*/pkgver=0.0.21/' PKGBUILD
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO
   git commit -m "Update to 0.0.21"
   git push
   ```

### Setting Up Your Homebrew Tap (Optional)

If you want automatic Homebrew publishing:

1. **Create the tap repository:**
   ```bash
   # On GitHub, create repo: kristoferlund/homebrew-ostt
   # Initialize with README
   ```

2. **Create GitHub token:**
   - Go to Settings → Developer settings → Personal access tokens
   - Create token with `repo` permissions
   - Copy the token

3. **Add secret to ostt repo:**
   - Go to ostt repo → Settings → Secrets → Actions
   - Add new secret: `HOMEBREW_TAP_TOKEN`
   - Paste your token

4. **Release:**
   - Next release will automatically update the tap
   - cargo-dist creates/updates `Formula/ostt.rb` in the tap repo

**Without the tap repo:** Releases still work, but Homebrew formula won't auto-publish. Users can still use the manual `ostt.rb` in this repo.

## Adding New Window Manager Integrations

To add support for a new window manager:

1. **Create directory:** `environments/<wm-name>/`
2. **Add configuration files** (will be embedded in binary via `include_str!()`)
3. **Update `src/setup/mod.rs`:**
   - Add `const` for each embedded file
   - Add detection function (e.g., `is_sway()`)
   - Add setup function (e.g., `setup_sway()`)
   - Call from `run_setup()` when detected
4. **Test and submit PR**

## Troubleshooting

### Binary not found after installation

**Shell installer:** Ensure `~/.cargo/bin` is in your PATH:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

**Homebrew:** Ensure Homebrew bin is in PATH:
```bash
export PATH="$(brew --prefix)/bin:$PATH"
```

### Unsupported architecture

Pre-built binaries support:
- x86_64 (Intel/AMD 64-bit)
- aarch64 (ARM 64-bit, Apple Silicon, Raspberry Pi 4+)

For other architectures, compile from source:
```bash
git clone https://github.com/kristoferlund/ostt.git
cd ostt
cargo build --profile dist
sudo cp target/dist/ostt /usr/local/bin/
```

### Permission denied

If you get permission errors, the binary may need to be made executable:
```bash
chmod +x /usr/local/bin/ostt
```

### Missing clipboard tool

If clipboard functionality doesn't work:

**macOS:** `pbcopy` should be built-in. If missing, your system may be corrupted.

**Linux Wayland:**
```bash
sudo apt install wl-clipboard      # Debian/Ubuntu
sudo pacman -S wl-clipboard        # Arch
sudo dnf install wl-clipboard      # Fedora
```

**Linux X11:**
```bash
sudo apt install xclip             # Debian/Ubuntu
sudo pacman -S xclip               # Arch
sudo dnf install xclip             # Fedora
```

### Missing ffmpeg

```bash
brew install ffmpeg                # macOS
sudo apt install ffmpeg            # Debian/Ubuntu
sudo pacman -S ffmpeg              # Arch
sudo dnf install ffmpeg            # Fedora
```
