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

- `dist/typerr-0.1.0-arch-linux-x86_64.tar.gz`
- `dist/typerr-0.1.0-1-x86_64.pkg.tar.zst`
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
tar -xzf dist/typerr-0.1.0-arch-linux-x86_64.tar.gz
./typerr-0.1.0-arch-linux-x86_64/typerr
```

Pacman package (optional; packaging does not run this):

```sh
sudo pacman -U dist/typerr-0.1.0-1-x86_64.pkg.tar.zst
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
settings requires administrator access. The application never writes settings.

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
runtime imports, and creates `dist/typerr-0.1.0-windows-x86_64.zip`.
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
