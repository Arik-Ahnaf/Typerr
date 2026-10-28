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
`dist/typerr-0.1.0-windows-x86_64.zip`, plus updated `dist/SHA256SUMS`.
The static-runtime flags apply only to the Windows target. No Wine or GitHub
Actions is needed to build. Test the extracted application on Windows before
claiming native Windows runtime support.

## App logo

`assets/icons/typerr-logo.svg` is the logo source. The checked-in RGBA window
icon and multi-size Windows ICO require no image tools during ordinary builds.
After changing the SVG, regenerate them with `bash packaging/generate-icons.sh`
(requires `librsvg` and `imagemagick`). Windows builds also need `llvm-rc` from
LLVM on `PATH`, or its path in `LLVM_RC`, to embed the executable icon.

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
