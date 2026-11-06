# Building Typerr

## Arch Linux setup

Use native Arch Linux x86_64, `base-devel`, and rustup. The repository pins
Rust **1.98.1**; keep `Cargo.lock` and use `--locked`. Initial builds need
network access or a populated Cargo cache.

```sh
sudo pacman -S --needed base-devel rustup
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
```

A graphical session and working graphics drivers are required to run the app.
See [PACKAGING.md](PACKAGING.md) for runtime libraries and packaging tools.
Fonts, icons, words, and theme colors are compiled into the executable.
SQLite is compiled from bundled C sources by `rusqlite`; the existing native C
compiler setup is required, but no system SQLite development package is needed.

## Build and check

Run from the repository root:

```sh
export CARGO_BUILD_JOBS=2
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
cargo build --locked --release
```

The binaries are `target/debug/typerr` and `target/release/typerr`.
`target/` is disposable build output. Use `cargo fmt --all` only when you
intend to format source.

Development builds optimize the existing image, JPEG/PNG codec, and tiny-skia
dependencies so picture loading and modal rendering stay responsive in `cargo
run`. Application code retains normal debug settings. Release optimization is
unchanged.

## Run and package

```sh
cargo run --locked
# Or run the optimized executable:
./target/release/typerr
# Check, build, and create both Arch packages:
bash packaging/build-arch.sh
```

`settings.json` sets `font-size` (8–48px). Restart after editing; no rebuild
is needed. The working directory takes precedence over the executable's
directory. Missing/invalid settings fall back to 16px. Theme JSON changes
require rebuilding.

Completed tests are saved to the user data directory, independently of the
working directory and executable location. See [STORAGE.md](STORAGE.md) for the
database paths, SQL queries, backup guidance, and error log. Tests use temporary
databases and never write to your real history.

Package outputs and usage are documented in [PACKAGING.md](PACKAGING.md).
Record dated validation results in [PLAN.md](PLAN.md), not in this guide.

## Windows x64 build from Arch

Use [cargo-xwin](https://github.com/rust-cross/cargo-xwin) with Clang/LLD:

```sh
sudo pacman -S --needed clang lld llvm python
cargo install --locked --version 0.23.1 cargo-xwin
rustup target add --toolchain 1.98.1 x86_64-pc-windows-msvc
bash packaging/build-windows.sh
```

The script runs native formatting/lint/tests and Windows-target Clippy, then
cross-compiles the MSVC release with a static C runtime and packages the current
settings/notices. Release debug symbols are omitted.
The first run downloads Microsoft's SDK/CRT and Windows-specific Cargo crates;
SDK downloads are cached under `target/xwin/`. Rerun after transient download failures.
This checkout also has project-local tools under `target/windows-tools/`, which
the script detects automatically; setup is unnecessary while those remain available.

Outputs: `target/x86_64-pc-windows-msvc/release/typerr.exe` and
`dist/typerr-2.0.0-windows-x86_64.zip`, plus updated `dist/SHA256SUMS`.
The static-runtime flags apply only to the Windows target. No Wine or GitHub
Actions is needed to build. Test the extracted application on Windows before
claiming native Windows runtime support.

## App logo

`assets/icons/typerr-logo.svg` is the logo source. The checked-in RGBA window
icon and multi-size Windows ICO require no image tools during ordinary builds.
After changing the SVG, regenerate them with `bash packaging/generate-icons.sh`
(requires `librsvg` and `imagemagick`). Windows builds also need `llvm-rc` from
LLVM on `PATH`, or its path in `LLVM_RC`, to embed the executable icon.
Run `bash packaging/build-windows-installer.sh` after the Windows ZIP build to
create the per-user setup executable. The script accepts `MAKENSIS`; otherwise
it detects native `makensis` or project-local Windows NSIS tools under
`target/windows-tools/nsis/` and runs them with Wine.

## Troubleshooting

- Missing compiler/components: run `rustup show` and repeat the setup command.
- Locked dependency mismatch: intentionally update the lockfile with dependency
  changes; do not delete it or remove `--locked` to bypass an error.
- Fetch failure: restore network access or populate the Cargo cache before retrying.
- Linker/library error: install the package providing the named missing library.
- Display/renderer error: check session access and graphics drivers. To test
  XWayland specifically, run `env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET ./target/release/typerr`.

Unit tests and builds do not verify native GUI behavior. Verify each display
backend separately; cross-compilation does not verify native Windows behavior.

## Updater checks

`./target/debug/typerr --version` prints the Cargo version and exits without
opening a window, database, or network connection. Updater unit tests use
fixtures and temporary directories. Native UI checks should use isolated
XDG_DATA_HOME/LOCALAPPDATA values to avoid real score history. See
[UPDATING.md](UPDATING.md) before testing file replacement or package upgrades.

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

PNG/JPG decoding uses the locked `image` codecs; `rfd` uses the Windows native
file dialog or Linux desktop portal (with Zenity fallback). On Arch, enable your
desktop's `xdg-desktop-portal` backend, or install `zenity` for the fallback.
Avatar persistence uses the user settings directory described above.
