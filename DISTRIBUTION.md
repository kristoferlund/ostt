# ostt Distribution and Installation Guide

This document explains how ostt is distributed and installed across different platforms and package managers.

## Repository Structure

```
ostt/
├── src/
│   ├── setup/mod.rs             # Setup module with embedded files
│   └── ...                      # Other source code
├── environments/                 # Embedded into binary at compile time
│   ├── ostt.toml                # Configuration template
│   └── hyprland/                # Hyprland window manager support
│       ├── ostt-float.sh        # Launch script for floating window
│       └── alacritty-float.toml # Terminal configuration
├── Cargo.toml                   # Rust package manifest
├── dist-workspace.toml          # cargo-dist configuration
└── README.md                    # Main documentation
```

**Note:** The AUR PKGBUILD is maintained in the separate [AUR repository](https://aur.archlinux.org/packages/ostt), not in this repository.

**Note:** Configuration files in `environments/` are embedded into the binary at compile time using `include_str!()`. They are automatically extracted on first run.

## Installation Methods

### 1. Shell Installer (Linux/macOS) - Recommended

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kristoferlund/ostt/releases/latest/download/ostt-installer.sh | sh
```

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

```bash
# Using yay
yay -S ostt

# Or using makepkg
git clone https://aur.archlinux.org/ostt.git
cd ostt
makepkg -si
```

### 4. Direct Binary Download (Linux/macOS)

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

### 5. Compile from Source

```bash
git clone https://github.com/kristoferlund/ostt.git
cd ostt
cargo build --release --profile dist
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
2. Detects Hyprland environment and sets up integration files:
   - `~/.config/ostt/alacritty-float.toml` - Terminal configuration
   - `~/.local/bin/ostt-float` - Launcher script (automatically made executable)

### Hyprland Integration

If you're using Hyprland, add a keybinding and window rules to your Hyprland config after first run:

```hyprland
# Keybinding for ostt (clipboard output)
bindd = SUPER, R, ostt, exec, bash ~/.local/bin/ostt-float -c

# OSTT window rules
windowrule = float on, match:title ostt
windowrule = size (monitor_w*0.14) (monitor_h*0.08), match:title ostt
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) (monitor_h*0.9), match:title ostt
```

Then reload your Hyprland configuration: `hyprctl reload`

This will launch ostt in a floating Alacritty terminal window, centered horizontally and positioned at the bottom of the screen.

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

### PKGBUILD (AUR)

Compiles from source and installs:
- Binary: `/usr/bin/ostt` (with embedded config files)
- Documentation: `/usr/share/doc/ostt/`

**Dependencies:**
- `alsa-lib` - Audio capture
- `openssl` - TLS for API calls
- `ffmpeg` - Audio format conversion

**Optional dependencies:**
- `wl-clipboard` - Clipboard support on Wayland
- `xclip` - Clipboard support on X11

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

Unlike Homebrew Core, **anyone can publish to the AUR**. It's a community-maintained collection of build scripts.

### Initial AUR Setup (One-Time)

1. **Create AUR account:**
   - Go to https://aur.archlinux.org/register
   - Add your SSH public key to your account

2. **Clone the AUR repository:**
   ```bash
   git clone ssh://aur@aur.archlinux.org/ostt.git aur-ostt
   cd aur-ostt
   ```

3. **Create PKGBUILD file:**
   ```bash
   # Create PKGBUILD with the following content
   # (See example below)
   
   # Generate .SRCINFO (required by AUR)
   makepkg --printsrcinfo > .SRCINFO
   ```

   Example PKGBUILD:
   ```bash
   # Maintainer: Kristofer Lund <kristoferlund@users.noreply.github.com>
   pkgname=ostt
   pkgver=0.0.3
   pkgrel=1
   pkgdesc="Open Speech-to-Text: Terminal application for recording and transcribing audio"
   arch=('x86_64' 'aarch64')
   url="https://github.com/kristoferlund/ostt"
   license=('MIT')
   depends=('alsa-lib' 'openssl' 'ffmpeg')
   optdepends=('wl-clipboard: Clipboard support on Wayland'
               'xclip: Clipboard support on X11')
   makedepends=('cargo' 'rust' 'git' 'pkgconf')
   options=('!lto')
   source=("${pkgname}-${pkgver}.tar.gz::https://github.com/kristoferlund/ostt/archive/refs/tags/v${pkgver}.tar.gz")
   sha256sums=('SKIP')
   
   build() {
     cd "ostt-${pkgver}"
     cargo build --release --locked
   }
   
   package() {
     cd "ostt-${pkgver}"
     install -Dm755 target/release/ostt "${pkgdir}/usr/bin/ostt"
     install -Dm644 README.md "${pkgdir}/usr/share/doc/ostt/README.md"
   }
   
   check() {
     cd "ostt-${pkgver}"
     cargo test --release --locked
   }
   ```

4. **Publish to AUR:**
   ```bash
   git add PKGBUILD .SRCINFO
   git commit -m "Initial release: ostt 0.0.1"
   git push
   ```

5. **Done!** Package is live at: https://aur.archlinux.org/packages/ostt

### Updating AUR Package

After each release:

```bash
cd aur-ostt

# Update version in PKGBUILD
sed -i 's/pkgver=.*/pkgver=0.0.2/' PKGBUILD

# Update checksums (or use sha256sums=('SKIP') for simplicity)
updpkgsums

# Regenerate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Update to 0.0.2"
git push
```

### Optional: Automate AUR Publishing

You can automate AUR updates with a GitHub Action. This requires:
- AUR SSH key added as GitHub secret
- Action that updates PKGBUILD and pushes to AUR on release

Example workflow snippet:
```yaml
- name: Publish to AUR
  run: |
    # Clone AUR repo
    git clone ssh://aur@aur.archlinux.org/ostt.git
    cd ostt
    
    # Update PKGBUILD version
    sed -i 's/pkgver=.*/pkgver=${{ github.ref_name }}/' PKGBUILD
    
    # Generate .SRCINFO
    makepkg --printsrcinfo > .SRCINFO
    
    # Push to AUR
    git add PKGBUILD .SRCINFO
    git commit -m "Update to ${{ github.ref_name }}"
    git push
```

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
   - Builds for all target platforms
   - Creates GitHub release
   - Uploads binaries and installer script
   - Generates Homebrew formula
   - Publishes to `kristoferlund/homebrew-ostt` (if tap repo exists)
6. **Manually update AUR** (until automated):
   ```bash
   cd aur-ostt
   sed -i 's/pkgver=.*/pkgver=0.0.2/' PKGBUILD
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO
   git commit -m "Update to 0.0.2"
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
