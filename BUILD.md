# Building and verifying Typerr

## Toolchain and dependencies

Alpha 0.1 targets Rust **1.98.1**, edition 2024; Iced **0.14.0** with default native render backends and `advanced`/`svg`; and rand **0.10.3**. The `rust-toolchain.toml` pin and `Cargo.lock` retain the toolchain and resolved dependency versions for reproducible builds.

Planning-stage local observations:

```text
rustc 1.98.1 (48a229cea 2026-09-01)
cargo 1.98.1 (797e8a9bc 2026-08-05)
```

These versions were inspected and the named 1.98.1 toolchain was subsequently installed successfully. Native Arch debug compilation and automated tests now pass; runtime checks are recorded separately below.

Both the stable alias and pinned `1.98.1` toolchain are installed locally, with rustfmt and Clippy. Approximately 8GiB RAM is available; use `CARGO_BUILD_JOBS=2` for initial Iced builds if necessary to limit concurrent compiler memory use. Verification commands used `CARGO_BUILD_JOBS=2` to limit concurrent compiler memory use.

Use rustup with the pinned toolchain and its `rustfmt` and `clippy` components. Initial dependency resolution requires network access or a populated Cargo cache; the built application does not require network access. The Google Sans Code and Jomhuria fonts, SVG icons and word list are embedded, so no system installation of Google Sans Code is required.

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
| Automated tests | Passed: 26 tests including engine, layout, generation, input/mode/timer policy, and integration |
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

### Typing-page follow-up — 2026-10-24

The updated native page uses Iced's `svg` feature; `Cargo.lock` records the additional transitive SVG rendering dependencies. Connection links use `xdg-open` on Arch and `cmd /C start` on Windows. Building and typing remain offline; clicking a connection delegates its configured Github, Twitter, or Discord URL to the default browser. Jomhuria and its OFL license are bundled alongside the existing Google Sans Code font.

Validation completed on 2026-10-25: formatting, strict Clippy, all 19 automated tests (13 library, 4 input policy, 2 integration), and locked debug/release builds passed on Arch Linux. `git diff --check` passed. The native debug window was launched outside the checkout on XWayland and its startup screenshot inspected at 125% scaling: bundled branding/icons, dark palette, blue word mode, 0/50 counter, and footer rendered correctly. Synthetic input attempts did not establish native counter/click behavior, so those runtime checks remain unverified; counter boundaries, mistakes, Backspace, and terminal completion are covered by automated tests. Native Windows, native Wayland, browser handoff, IME, and physical input remain unverified for this follow-up. No score page or packaged release was added.

### Dracula palette build — 2026-10-25

`build.rs` reads `themes/dracula.json`, validates `#RRGGBB` colors and role references with build-only `serde_json`, and generates color constants under Cargo's `OUT_DIR`. Cargo rebuilds when the JSON changes. The executable includes the resulting colors and theme name; no external theme file is needed at runtime. Unknown role references or malformed colors fail the build.

Validation: formatting, strict Clippy, all 19 existing automated tests, and locked debug/release builds passed on Arch Linux. An isolated build-script check confirmed that changed JSON colors propagate to generated constants, while malformed hex and unknown color references fail. The source JSON was unchanged by those checks. No new native GUI or Windows verification was performed for this palette-only change.

## Typing area maximum width — 2026-10-25

The typing panel now stays horizontally centered and grows to at most 700 logical pixels wide, including its inner padding. Smaller windows retain the 30px exterior margins (640px panel at the 700px minimum window width). Height and measured text wrapping retain their existing behavior.

Validation: formatting, strict Clippy, all 19 automated tests, and locked debug/release builds passed on Arch Linux. No new GUI screenshot or native Windows verification was performed for this change. Existing platform limitations and planned next-alpha work remain unchanged.

### Words and Time modes — 2026-10-25

Both modes generate 500 words. The countdown uses `std::time::Instant` and native scheduled redraws, adding no dependencies. Keyboard/focus delivery remains synchronous.

Validation: formatting, strict Clippy, all 22 automated tests (13 library, 7 app policy, 2 integration), and locked debug/release builds passed on Arch Linux. The new tests cover 500-word generation, mode changes and active-mode no-ops, Tab resets, countdown startup and elapsed-time catch-up, focus loss, expiry, and continued editing after zero. `git diff --check` passed.

Native XWayland at 125% scaling: launched the debug executable outside the checkout and inspected `0/500`, Time mode's blue clock icon and `30s` monitor, automatic countdown to `28s` and `0s`, unchanged text at expiry, further input after expiry, and Tab resetting Time mode to `30s` with fresh text. Synthetic pointer positioning was unreliable; returning to Words through a native click was not conclusively verified, though both mode transitions passed app-state tests and button-message wiring was reviewed. Native Windows, native Wayland, physical input, and packaging remain unverified.

### Four-row viewport and 50-word target — 2026-10-25

The centered panel caps at 700 × 110 logical pixels. Words mode displays a 50-word target while retaining the 500-word buffer; reaching 50 does not end input.

Validation: formatting, strict Clippy, all 23 automated tests (13 library, 8 app policy, 2 integration), and locked debug/release builds passed on Arch Linux. The new boundary test confirms the 500-word buffer remains active after the 50-word target, the monitor caps at `50/50` even with errors, Backspace below the target lowers it, and Tab resets it to `0/50`. `git diff --check` passed.

Native XWayland screenshots at 125% scale confirmed four centered rows, including the faded fourth preview row, at the 875 × 500 physical-pixel minimum and a maximized 1920 × 1050 client area. After 500 targeted keypresses, the counter displayed `50/50` with further words available and the four-row viewport intact. Native Windows and native Wayland remain unverified; earlier input/platform limitations still apply.

### Punctuation and number toggles — 2026-10-25

Generation options are in-memory state and use the existing seeded RNG. No dependency or runtime asset changes were needed.

Validation: formatting, strict Clippy, all 26 automated tests (14 library, 10 app policy, 2 integration), and locked debug/release builds passed on Arch Linux. Seeded tests exercise all four feature combinations, exact token/separator counts, early feature presence, and valid generated content. App tests cover independent toggles, session/timer resets in both modes, preserving settings across Tab/mode changes, and committed punctuation/digit input with word counting and Backspace. `git diff --check` passed. Native feature-toggle clicks and their visual states were not conclusively verified for this follow-up; earlier XWayland timer/viewport evidence remains recorded above. Native Windows, native Wayland, and packaged releases remain unverified.

## Typing font settings — 2026-10-25

Edit the repository's `settings.json` before running `cargo run --locked`:

```json
{
  "font-size": 16
}
```

`font-size` accepts numbers from 8 through 48 in logical pixels, including fractions. Restart the app to apply changes; rebuilding is unnecessary. The app first looks in its working directory, then beside its executable. Missing files or invalid values fall back to 16px; malformed/unreadable files emit a diagnostic. It never rewrites the file. For a portable launch from another directory, place the file beside the executable.

Typing line height is 1.25 times font size. Panel height scales to fit four whole rows plus 30px padding: 110px at 16px font, 150px at 24px font. Startup/minimum window height increases above 400px as needed to preserve 125px top/bottom clearance (520px at 48px font). Header/footer and counter/timer sizes stay unchanged. Runtime JSON parsing uses the existing locked `serde_json` version.

Validation: formatting, strict Clippy, all 28 automated tests, and locked debug/release builds passed on Arch Linux. Configuration tests cover defaults, fractional sizes, malformed JSON, wrong types, and range boundaries. No native GUI or Windows verification was performed for this change.

## Four rows at configurable font sizes — 2026-10-25

Removed the fixed 110px panel height cap. Panel height now fits four configured line heights plus 30px inner padding, rounded upward for fractional sizes. At 24px the panel is 150px high and still fits a 700 × 400 window. Larger fonts increase startup/minimum window height as needed to retain the existing exterior clearance; 48px uses 520px window height. Settings are loaded once and shared between startup geometry and app state. The user's 24px setting was preserved.

Validation: formatting, strict Clippy, all 29 tests, locked debug/release builds, and `git diff --check` passed. A regression test checks four whole viewport rows and minimum-window space at 4,001 font sizes from 8–48px, including the reported 24px case. No native GUI or Windows check was performed; existing platform limitations and next-alpha scope remain unchanged.
