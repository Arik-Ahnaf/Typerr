# Building and verifying Typerr

## Toolchain and dependencies

Alpha 0.1 targets Rust **1.98.1**, edition 2024; Iced **0.14.0** with default native render backends and `advanced`; and rand **0.10.3**. The `rust-toolchain.toml` pin and `Cargo.lock` retain the toolchain and resolved dependency versions for reproducible builds.

Planning-stage local observations:

```text
rustc 1.98.1 (48a229cea 2026-09-01)
cargo 1.98.1 (797e8a9bc 2026-08-05)
```

These versions were inspected and the named 1.98.1 toolchain was subsequently installed successfully. Native Arch debug compilation and automated tests now pass; runtime checks are recorded separately below.

Both the stable alias and pinned `1.98.1` toolchain are installed locally, with rustfmt and Clippy. Approximately 8GiB RAM is available; use `CARGO_BUILD_JOBS=2` for initial Iced builds if necessary to limit concurrent compiler memory use. Verification commands used `CARGO_BUILD_JOBS=2` to limit concurrent compiler memory use.

Use rustup with the pinned toolchain and its `rustfmt` and `clippy` components. Initial dependency resolution requires network access or a populated Cargo cache; the built application does not require network access. The font and word list are embedded, so no system installation of Google Sans Code is required.

## Setup and development commands

Run project commands from the repository root. Setup and debug build/test commands have been executed on Arch Linux; the table below distinguishes each verification result.

```sh
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
cargo run --locked
```

Run formatting without modifying source during verification:

```sh
cargo fmt --all -- --check
```

Check linting, automated behavior, and builds:

```sh
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
cargo build --locked --release
```

When intentionally formatting changed source, use `cargo fmt --all`, then repeat the non-mutating check. The manifest and lockfile already resolve the selected dependencies; keep `--locked` in reproducibility checks.

Development output normally appears under `target/debug/`; release output under `target/release/`. `target/` is regenerable build output, not packaging source. Expected native binaries are `target/release/typerr` on Linux and `target/release/typerr.exe` on Windows; verify existence before using or documenting a packaged artifact.

## Arch Linux

Build natively on Arch with a Rust toolchain, a native C/C++ linker/build toolchain, and the runtime libraries required by Iced's selected windowing/render backends. A graphical session is required for meaningful native GUI verification. Keep the default native renderers unless evidence justifies a documented change.

Determine and record actual package/runtime requirements from the resolved dependencies, build errors, and binary linkage. Do not assume a generic package list has been validated. X11 and Wayland are separate runtime paths; document which was exercised. A compile or headless test does not establish display behavior on either path.

Completed runtime checks are listed below. Physical wheel/held-key behavior, native Wayland, and full IME input remain unverified. Preview opacity and viewport scroll policy were tested automatically. No input latency benchmark was performed.

The current desktop is KDE Wayland with XWayland. The sandbox blocks display access by default; an approved elevated `xdpyinfo` inspection succeeded. The release executable was subsequently launched outside the checkout and visually inspected over XWayland. Use the available approval mechanism for necessary native GUI checks rather than interpreting sandbox display denial as an application defect.

## Windows

Use a native Windows environment with the MSVC Rust toolchain and Microsoft C++ build/linker tools plus an appropriate Windows SDK. The same Cargo commands are intended for a native build.

Windows compilation, runtime behavior, display scaling, keyboard input, and packaging must be tested on Windows before being reported as verified. A Linux compile, cross-compile, or Wine launch is not native Windows verification. Native `windows-latest` CI may be introduced later if authorized; no working pipeline is claimed by this guide.

## Troubleshooting

- **Locked dependency mismatch:** implementation must intentionally resolve the changed manifest and commit the resulting lockfile. Do not silently discard `--locked` in reproducibility checks or delete the lockfile to conceal drift.
- **Missing toolchain/components:** inspect `rustup show` and install the pinned toolchain, rustfmt, and Clippy. The setup command succeeded on the current Arch host; it still requires network access and permission to write rustup directories on a new machine.
- **Network/cache failure:** report the actual dependency-fetch error. A fetch failure is not a source compilation failure; do not claim tests ran if compilation never started.
- **Linux linker or shared-library errors:** identify the named missing dependency, install the corresponding native development/runtime package, and record what was required.
- **No display or renderer initialization failure:** confirm a usable native graphical session and graphics drivers. Headless unit tests can still verify the pure engine/layout, but cannot establish GUI correctness.
- **Windows linker errors:** check that the MSVC C++ build tools and SDK are installed for the selected target.
- **Wrong font, caret drift, or clipping:** verify embedded font registration and that glyphs/caret share measured layout coordinates. Do not compensate with guessed width constants or independent caret math.

## Verification record

| Check | Current status |
| --- | --- |
| Rust/Cargo version inspection | Executed locally; both 1.98.1 |
| Bundled font family inspection | Executed with `fc-scan`; Google Sans Code Regular |
| Toolchain installation command | Passed on Arch Linux x86_64 |
| `cargo fmt --all -- --check` for Alpha 0.1 | Passed |
| `CARGO_BUILD_JOBS=2 cargo clippy --locked --all-targets -- -D warnings` | Passed without warnings |
| Automated tests | Passed: 18 tests including engine, layout, generation, input policy, and integration |
| Development build | Passed on Arch Linux x86_64 |
| `CARGO_BUILD_JOBS=2 cargo build --locked --release` | Passed on Arch Linux x86_64 |
| Native Arch GUI and visual review | Passed the XWayland checks below at 125% desktop scaling |
| Native Windows build/runtime | Not verified |

Final checks ran after the transparent-panel clarification and synchronous keyboard-delivery fix. All 15 tests passed; formatting and strict Clippy passed; both debug and release binaries built successfully.

### Native validation performed

The release executable was launched from `/tmp` using the following command, with the absolute path adjusted to this checkout:

```sh
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET /home/eric/Codes/Rust/typerr/target/release/typerr
```

This explicitly selected XWayland under KDE; it does not verify native Wayland. The renderer backend was not instrumented, so no specific GPU/software-renderer claim is made.

- Captured and inspected the transparent/borderless typing area, text, and caret. The Outline reference's colored rectangles are layout guides, per the user's clarification.
- Inspected minimum-size hints: 875 × 500 physical pixels at 125% scale equals 700 × 400 logical pixels. Inspected settled reflow at 1250 × 750 physical pixels, preserving progress.
- Sent targeted X11 key events to the application: correct text, red space underline, wrapped separator scrolling, and Backspace across the boundary behaved as specified.
- Verified 100 queued inputs against the exact expected caret position, then 100 Backspaces restored the start. A 1,000-input burst reached completion; extra typing and Backspace left the screenshot unchanged (ImageMagick pixel absolute-error count 0).
- The initial event subscription dropped burst input; keyboard/focus events now use the widget's synchronous `Shell::publish` path. Do not regress this to an asynchronous subscription.

The test harness used window-targeted X11 events because desktop-wide XTest injection was ineffective in this session. These checks do not establish physical held-key repeat, manual wheel behavior, full IME input, or Windows compatibility. Those remain separate checks. Unit tests cover focus gating, viewport following/manual-scroll policy, and conditional preview opacity.

## Observed Linux dependencies

The debug executable was inspected with `ldd`. Direct shared-library linkage resolved `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and the native ELF loader. Iced also loads display/graphics libraries dynamically; this linkage list is **not** a complete portable-package dependency declaration. No extra Linux packages were needed for the build on this configured Arch host. A graphics driver and the libraries for the chosen X11/Wayland renderer path remain runtime requirements.

### Tab restart and centering follow-up

After this change, `cargo fmt --all -- --check`, `CARGO_BUILD_JOBS=2 cargo test --locked` (18 tests), `CARGO_BUILD_JOBS=2 cargo clippy --locked --all-targets -- -D warnings`, and `CARGO_BUILD_JOBS=2 cargo build --locked --release` passed on Arch Linux. Prior native evidence above belongs to the earlier implementation; follow-up visual results are recorded separately.

Native XWayland follow-up at 125% scaling: captured and inspected centered text/caret at startup and completion. Targeted Tab events restarted both active and completed sessions with fresh content and reset progress.

The follow-up also visually verified centering at 1000 × 600 logical pixels and the completed single-line block; the final native session was restarted with Tab and returned to 700 × 400 logical pixels.
