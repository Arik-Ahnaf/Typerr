# Alpha 0.1 implementation plan

## Version and scope

Current target: **Alpha 0.1**, crate version `0.1.0`.

Deliver a native Rust/Iced Main Typing page with 50 randomly selected English words, responsive reflow, immediate input, expected-glyph error highlighting, Backspace, terminal completion, and a caret architecture ready for future animation.

Only the typing panel is in scope. Do not implement secondary pages, navigation, branding, icons, mode controls, scores, statistics, timers, on-screen restart controls, persistence, networking, or full caret animation.

## Planning decisions

- [x] Inspect the fresh Rust repository and existing font assets.
- [x] Inspect the Figma frame and relevant child nodes; compare the local reference.
- [x] Separate the typing engine, pure measured layout, application state, and native UI.
- [x] Define typing, wrapping, separator ownership, viewport, preview, and completion behavior in `AGENTS.md`.
- [x] Select Rust 1.98.1, Iced 0.14.0 with `advanced` and default native backends, and rand 0.10.3.
- [x] Create the four project guides before application implementation.

The implementation agent must read all four guides first. The coordinating agent performs integration review after implementation and verification.

## Implementation milestones

1. **Foundation**
   - [x] Pin toolchain and dependency versions; retain `Cargo.lock` for source control.
   - [x] Embed the existing static Google Sans Code Regular font.
   - [x] Add an original curated list of approximately 200 common lowercase ASCII words, with provenance recorded alongside it.
   - [x] Establish the small modules and boundaries documented in `AGENTS.md`.
2. **Pure typing and word generation**
   - [x] Generate exactly 50 words with replacement, separated by 49 spaces, without a trailing space.
   - [x] Implement correct/incorrect/pending character state, input, Backspace, and immutable completion.
   - [x] Cover generation and engine edge cases with deterministic unit tests.
3. **Measured layout and viewport**
   - [x] Measure the embedded monospaced font using Iced's text renderer.
   - [x] Implement pure wrapping and shared glyph/caret coordinates, including separator ownership.
   - [x] Implement active-line-at-top advancement, whole-line clipping, conditional 50% preview opacity, and manual scrolling.
   - [x] Reflow correctly after resizing and restore the proper line after Backspace.
   - [x] Keep rendered caret coordinates independent and snap them for this alpha.
4. **Native GUI and input**
   - [x] Open a 700 × 400 window with the same minimum size.
   - [x] Render the in-scope Figma panel, colors, and embedded font.
   - [x] Handle committed keyboard text once, native repeats, Backspace, and focus safely.
   - [x] Preserve completed text and permit inspection without further engine edits.
5. **Verification and integration**
   - [x] Run formatting, Clippy, unit tests, and development/release builds where available.
   - [x] Exercise available native GUI behavior, resizing, scrolling, and completion.
   - [x] Compare in-scope rendered geometry and typography with the Figma reference.
   - [x] Record exact validation evidence and outstanding platform limitations below and in `BUILD.md`.
   - [x] Review architectural consistency and update all documentation to reflect the implemented state.

## Acceptance criteria

- The application is a native Iced program with no web interface.
- Startup displays exactly 50 bundled English words; duplicate selections are allowed.
- At 700 × 400, the centered typing panel matches the reference's 640 × 250 dimensions, 30px horizontal / 75px vertical exterior margins. Its background is transparent and it has no decorative border: the colorful rectangles in the Outline reference indicate general areas only.
- The visible text block is always centered horizontally and vertically, with shared left alignment within the block. Restart invalidates cached text layout and resets caret/scroll state.
- Resizing grows/reflows the panel while retaining 20px type, 25px line height, and whole visible rows. No text row is partially clipped at the bottom.
- Correct input uses `#F2F2F2`; pending text uses `#B4B4B4`; mistakes show the expected glyph in `#FF6B6B`. Incorrect expected spaces remain visibly marked.
- Each printable input consumes one position regardless of correctness. Backspace corrects prior input and crosses word/line boundaries safely.
- Completing a visual line includes consuming its trailing separator position, then makes the next active line the top visible line. Wrong input at that separator also advances.
- The bottom visible row is a 50%-opacity preview only when more text exists below the viewport. It becomes opaque when moving upward. The active row is never faded.
- Wheel scrolling permits inspection; accepted typing, successful Backspace, and resize restore the active line at the top.
- The final input completes the session with no dialog, page change, or automatic restart. An unmodified Tab keypress explicitly starts a fresh 50-word race at any time, including after completion. While completed, further typing and Backspace are ignored until Tab restarts. The final line remains visible instead of advancing to an empty row; wheel inspection remains possible.
- The engine has no Iced dependency, and logical progress is independent from rendered caret coordinates.
- Embedded assets work when the binary is launched outside the repository directory.
- Tests and verification evidence support the claimed behavior; Windows and Arch runtime status are separately qualified.

## Test scenarios

**Engine and words:** list validity; exact word/separator counts; seeded generation; membership in the bundled list; correct and wrong letters; uppercase/non-ASCII scalar mistakes; wrong spaces; a space at a letter; Backspace at zero and across separators; final correct/incorrect input; no advancement or Backspace after completion.

**Layout and viewport:** narrow/wide widths; whole-word wrapping; long-word fallback; expected spaces preserved at wraps; caret positions before/after separators; one line advance per completed visual line; Backspace restoring the previous line; resize preserving progress; whole bottom row; preview alpha only when more content follows; manual-scroll restoration; terminal caret without a blank line.

**Native integration:** startup/minimum size, immediate input without duplication, key repeat, focus loss/regain, visible space errors, minimum and larger sizes, wheel inspection, completed-state lock, font loading, and launch from another working directory. Verify screenshots only when actually captured and inspected.

## Verification record

Planning evidence: the original application was a Rust edition-2024 hello-world binary; all four documentation files were initially empty. Figma and local references were inspected. Local toolchain version commands reported Rust/Cargo 1.98.1. The static font family was inspected with `fc-scan`.

Stage 2 implementation is complete. On the native Arch Linux x86_64 host with Rust 1.98.1:

- `rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy`: succeeded after granting required toolchain-directory/network access.
- `cargo fetch`: succeeded; pinned direct dependencies and the resolved transitive graph are in `Cargo.lock`.
- `CARGO_BUILD_JOBS=2 cargo test --locked`: passed 15 tests (11 library, 2 native-event-policy, 2 engine/layout integration).
- `CARGO_BUILD_JOBS=2 cargo build --locked`: succeeded; executable is `target/debug/typerr`.
- `cargo fmt --all -- --check`: passed.
- `CARGO_BUILD_JOBS=2 cargo clippy --locked --all-targets -- -D warnings`: passed without warnings.
- `CARGO_BUILD_JOBS=2 cargo build --locked --release`: succeeded after the final integration changes; native x86_64 ELF inspected with `file` and `ldd`.
- Coordinating-agent review completed. Changed keyboard/focus delivery from the bounded asynchronous event subscription to synchronous widget publication after native burst testing exposed dropped inputs. All 15 tests, strict Clippy, formatting, debug build, and release build passed again after this fix.
- Native release launched from `/tmp` on Arch KDE/XWayland at 125% scaling. Inspected transparent, borderless rendering and embedded typography; minimum-size hints are 875 × 500 physical pixels (700 × 400 logical). Inspected reflow at 1250 × 750 physical pixels (1000 × 600 logical), preserving logical progress.
- Window-targeted X11 events verified correct glyphs, red expected-space underline, a full-line input burst, separator-triggered upward scrolling, and Backspace restoring the previous line. After the input-delivery fix, 100 queued keystrokes reached the exact expected position, 100 Backspaces restored the start, and 1,000 queued keystrokes completed the session. Further Backspace and typing produced a pixel-identical completed screenshot (ImageMagick absolute-error count: 0).
- Preview opacity and manual-wheel viewport policies have automated test coverage; physical wheel/held-key behavior, native Wayland, IME, and Windows were not conclusively exercised. Native checks are functional evidence, not latency benchmarks. Packaging remains unimplemented.

The word list contains exactly 200 original curated words. Keyboard/focus events are published synchronously by the widget and update the Main Typing state; successful edits increment a presentation revision so no-op input preserves a manually inspected viewport. Widget-local state owns font measurement, wrapping, viewport, and separate target/rendered caret positions.

## Known limitations and future considerations

- Arch Linux release launch and the native XWayland checks above passed. Native Wayland and Windows build/runtime verification remain pending.
- Alpha 0.1 uses a monospaced ASCII English target. Full IME/composition, international word sets, and advanced Unicode shaping are future work unless separately verified.
- Smooth caret interpolation, configurable easing/duration, and animated line transitions are deferred; retain the independent target/rendered coordinate boundary now.
- Score, Dashboard, Settings, additional modes, and on-screen restart controls require later version plans.
- Installer creation, distribution automation, signing, and publication remain unimplemented; see `PACKAGING.md`.

## Follow-up: Tab restart and centering

- Implemented explicit unmodified Tab restart with new 50-word content, clearing typing, caret, and viewport state. Held-Tab repeats, modified shortcuts, and unfocused key events do not restart.
- Centered the visible block on both axes while retaining left-aligned lines and shared text/error/caret geometry. New target text invalidates the layout cache, even when the window size is unchanged.
- Passed 18 automated tests (12 library, 4 input-policy, 2 integration), formatting, strict Clippy, and the native release build. New tests cover active/completed restart, shortcut/focus/repeat exclusions, and centered geometry after scrolling/resizing.
- Native XWayland follow-up: visually inspected centered text and caret; targeted Tab restarted both in-progress and completed races with fresh words and cleared styling. The completed single-row block also remained centered.

The follow-up also visually verified centering at 1000 × 600 logical pixels and the completed single-line block; the final native session was restarted with Tab and returned to 700 × 400 logical pixels.

## Source upload verification — 2026-10-22

- Re-ran formatting, strict Clippy, all 18 automated tests, and locked debug/release builds successfully on Arch Linux before the initial Alpha 0.1 source upload.
- Expanded `.gitignore` to exclude generated build/distribution output, editor/OS files, local environment secrets, logs, and unused font variants. The embedded Regular font, OFL notice, lockfile, tests, and design reference remain included.
- No new GUI or Windows verification was performed for this source upload; the native evidence and limitations above still apply. Packaged releases remain future work.
