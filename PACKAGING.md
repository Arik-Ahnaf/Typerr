# Packaging Typerr

## Build both formats

Run on native Arch Linux x86_64 with the pinned Rust toolchain from
[BUILD.md](BUILD.md). Packaging also needs `base-devel` (makepkg/fakeroot),
`python`, `desktop-file-utils`, `file`, `tar`, `gzip`, and `zstd`.

```sh
sudo pacman -S --needed base-devel python desktop-file-utils file tar gzip zstd \
  libx11 libxcursor libxi libxrandr libxkbcommon libxkbcommon-x11 \
  wayland libglvnd vulkan-icd-loader libxcb
bash packaging/build-arch.sh
```

The script runs formatting, strict Clippy, tests, and locked debug/release
builds with two Cargo jobs by default. It stages the current release binary
and settings, creates the portable archive, and feeds that archive and its
SHA-256 checksum into makepkg. It does not install packages or publish files.
It requires a native host build configuration; remove cross-target overrides
from Cargo config before running it.

Outputs for the current `Cargo.toml` version:

- `dist/typerr-2.0.0-arch-linux-x86_64.tar.gz`
- `dist/typerr-2.0.0-1-x86_64.pkg.tar.zst`
- `dist/SHA256SUMS`

Check downloaded/copied artifacts with:

```sh
cd dist
sha256sum --check SHA256SUMS
```

`packaging/` holds maintained scripts, the PKGBUILD template, launcher, usage
text, and supplemental upstream notices. `target/packaging/arch.*` holds each
run's staging files, resolved metadata, generated checksum-bound PKGBUILD,
and build evidence. `dist/` and `target/` are generated output. The script
replaces matching distribution filenames only after both formats build.

## Run or install

Portable archive:

```sh
tar -xzf dist/typerr-2.0.0-arch-linux-x86_64.tar.gz
./typerr-2.0.0-arch-linux-x86_64/typerr
```

Pacman package (optional; packaging does not run this):

```sh
sudo pacman -U dist/typerr-2.0.0-1-x86_64.pkg.tar.zst
typerr
```

The package adds an application-menu entry and `/usr/bin/typerr` symlink.
The supplied logo is installed under the hicolor icon theme for the launcher;
the same logo is embedded as the app's window icon.
The executable and configuration live together under `/usr/lib/typerr/`.
`settings.json` is registered in pacman's backup list to preserve user edits
on upgrades. Documentation is under `/usr/share/doc/typerr/`; notices are
under `/usr/share/licenses/typerr/`.

Both artifacts copy the checkout's current settings, presently 24px. Edit
`font-size` (8–48) and restart; a settings file in the launch working directory
takes precedence over the executable-adjacent file. Editing installed
settings requires administrator access. The application saves your name in settings; other values are preserved.

Completed scores are written to the user's data directory, not `/usr/lib/typerr`
or the extracted application folder. See [STORAGE.md](STORAGE.md) for exact paths.
Do not package `scores.db`, its SQLite sidecars, or diagnostic logs. Bundled
SQLite adds no system SQLite runtime dependency; retain the collected rusqlite,
libsqlite3-sys, and embedded SQLite notices when generating either package.

## Runtime and included notices

Both formats need a working graphical session and graphics drivers. Dependencies
include `glibc`, `gcc-libs`, `libx11`, `libxcursor`, `libxi`, `libxrandr`,
`libxkbcommon`, `libxkbcommon-x11`, `wayland`, `libglvnd`, `vulkan-icd-loader`,
and `libxcb`. These cover direct linkage and dynamically loaded Iced/winit,
X11/Wayland, OpenGL/EGL, Vulkan, and software-renderer paths. The portable
archive does not bundle these system libraries. A suitable Vulkan driver is
optional for hardware Vulkan rendering; `xdg-utils` and a browser enable links.

Fonts, icons, words, and theme colors are embedded. Both formats include font
OFL notices, word-list provenance, Rust standard-library notices, and the locked
Linux Cargo dependency notices (including build dependencies). Missing crate
archive notices use checked-in supplemental files with upstream provenance in
`packaging/notices.json`. New dependencies without notices fail packaging until
reviewed. Typerr has no declared application-code license; the package records
that fact without assigning one.

Native XWayland package checks and build results belong in [PLAN.md](PLAN.md).
Archive extraction/launch checks do not prove a clean-machine installation or
pacman upgrade behavior. Native Wayland and Windows require separate validation.
The Arch workflow includes no AUR upload, CI release, signing, or publication.

## Windows x64 ZIP from Arch

Set up cargo-xwin and the Windows Rust target using [BUILD.md](BUILD.md), then run:

```sh
bash packaging/build-windows.sh
```

This cross-compiles the MSVC release, checks the x64 PE GUI subsystem and
runtime imports, and creates `dist/typerr-2.0.0-windows-x86_64.zip`.
Extract the whole ZIP on Windows and open `typerr.exe`. The ZIP contains the
current settings (24px), usage instructions, and Windows dependency/font/Rust
notices. The executable includes the supplied logo at multiple icon sizes for
Explorer and shortcuts. The C runtime is linked statically; Windows system DLLs and graphics
drivers are still required. The artifact is unsigned.

Windows staging, target-filtered Cargo metadata, and PE inspection are under
`target/packaging/windows.*`; build dependencies are cached in `target/xwin/`.
Both packaging scripts retain checksums for existing Arch and Windows archives
in `dist/SHA256SUMS`. No GitHub Actions, host installation, or upload is involved.
Cross-compilation and ZIP checks are separate from native Windows GUI testing;
the latter remains required before claiming runtime support.

Create the Windows installer from that same ZIP payload:

```sh
bash packaging/build-windows-installer.sh
```

The resulting `dist/typerr-2.0.0-setup.exe` installs for the current user under
Local AppData Programs, creates Start Menu and desktop shortcuts, and registers
an uninstaller. Reinstalling preserves an existing adjacent `settings.json`;
uninstalling also leaves it in place. The installer is unsigned. See BUILD.md
for the supported local NSIS tool discovery.

## Updater release contract

Build/package version 2.0.0 and later with the matching `v<version>` GitHub tag.
Set `TYPERR_RELEASE_TAG=v2.0.0` while packaging to enforce the intended tag.
Native packaging executes `typerr --version`; Windows PE version resources are
generated from Cargo's package version. Never reuse the historical v1.0.0 /
0.1.0 mismatch. Keep all platform assets and `SHA256SUMS` attached to the same
stable release, with exactly the generated names. Upload all assets before
making a release latest. The updater rejects incomplete releases.

See [UPDATING.md](UPDATING.md) for installation, integrity checks, and recovery.
Polkit plus a desktop authentication agent are optional runtime requirements
for in-app pacman upgrades. Portable replacement requires a writable application
folder. New HTTPS/archive/checksum dependency notices are collected by the
existing locked-graph notice collector. No updater service or startup task is
installed; update checks run only when the user launches Typerr.

On first launch, enter a name (1–16 characters) and choose Save or press Enter.
The name is saved as `name` in `settings.json` and used on the account button
and Dashboard. The prompt blocks the app until saving succeeds. Existing valid
names skip the prompt on subsequent launches.

User settings take precedence over the working-directory and executable-adjacent
files described above. When the original file cannot be written, Typerr preserves
its other values in a user-owned `settings.json`: `$XDG_CONFIG_HOME/typerr/`
(or `~/.config/typerr/`) on Linux, `%LOCALAPPDATA%/typerr/` on Windows. No administrator
access is needed to save a name for a packaged installation. Invalid JSON is
reported without overwriting it. Packages strip the builder's saved name.

On Dashboard, hover over your profile picture to see the pencil, then click to
choose a PNG or JPG. Drag the image to position it and use the slider or mouse
wheel to zoom. Save applies the crop; Discard, ×, or Escape keeps your previous
picture. Tab selects zoom/Discard/Save; arrows adjust zoom, Shift+arrows moves the
image, and Enter activates a selected button. The saved picture is a separate
512px `avatar.png` in the user configuration folder and survives moving/deleting
the source image. Failed reads/saves show an error without changing your picture.

Do not stage user `avatar.png` files. The default JPEG and modal SVGs are embedded;
the added Bold font is covered by the existing Google Sans Code OFL notice.
Retain the locked image-codec and native-dialog dependency notices. Linux file
selection uses the desktop's XDG portal/FileChooser backend, falling back to
Zenity; these are optional runtime dependencies for picture selection.
