# Alpha 0.1 implementation plan

## Version and scope

Current target: **Alpha 0.1**, crate version `0.1.0`.

Deliver native Rust/Iced Main Typing and Score pages with 500-word buffers, responsive input/layout, smooth 100 ms caret movement, and a 300 ms completion crossfade. The Score page uses fixed mock statistics and a native Canvas graph; real scoring is deferred.

The 2026-10-25 mode follow-up adds functional Words/Time buttons, 500-word sessions, and a 30-second countdown. The initial Alpha 0.1 milestones below are historical. The 2026-10-24 typing-page follow-up expands it to the current Figma frame, branding, inert controls, live word counter and footer links; see the follow-up record below. The 2026-10-27 score follow-up supersedes the earlier no-navigation rules; Dashboard and Settings remain excluded.

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
   - [x] Keep rendered caret coordinates independent; the smooth-caret follow-up now animates them over 100 ms.
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
- Startup generates 500 bundled English words with punctuation and numbers off. Both independent toggles can be enabled together in either mode, regenerating 500 tokens while preserving the mode; duplicate selections are allowed.
- At 700 × 400, the centered typing panel is 640 × 110 logical pixels, with 30px horizontal / 145px vertical exterior margins. Its background is transparent and it has no decorative border: the colorful rectangles in the Outline reference indicate general areas only.
- The visible text block is always centered horizontally and vertically, with shared left alignment within the block. Restart invalidates cached text layout and resets caret/scroll state.
- Resizing grows/reflows the centered panel up to a maximum width of 700 logical pixels and a height of four configured line heights plus inner padding (110px at the default 16px font), retaining whole visible rows. Startup/minimum window height grows as needed to preserve four rows and exterior clearance. No text row is partially clipped at the bottom.
- Correct input uses `#F8F8F2`; pending text uses Dracula Comment `#6272A4`; mistakes show the expected glyph in Dracula Red `#FF5555`. Incorrect expected spaces remain visibly marked.
- Each printable input consumes one position regardless of correctness. Backspace corrects prior input and crosses word/line boundaries safely.
- Completing a visual line includes consuming its trailing separator position, then makes the next active line the top visible line. Wrong input at that separator also advances.
- The bottom visible row is a 50%-opacity preview only when more text exists below the viewport. It becomes opaque when moving upward. The active row is never faded.
- Wheel scrolling permits inspection; accepted typing, successful Backspace, and resize restore the active line at the top.
- Words finishes at target word 50; Time finishes at its monotonic 30-second deadline; full-buffer exhaustion also finishes either mode. Freeze edits and crossfade to Score over 300 ms. Tab starts fresh at any time, including during the fade. On Score, Next generates new text and Retry reuses the target; both reset progress/timer/presentation while preserving mode/options.
- The engine has no Iced dependency, and logical progress is independent from rendered caret coordinates.
- The caret uses always-on 100 ms `inOut(1.25)` movement, including line returns and Backspace. Rapid input retargets from the current interpolated position; resets, reflow, and focus loss cancel movement immediately. Animation and countdown redraws preserve manual inspection and edit revisions.
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
- Smooth caret interpolation and line returns are implemented. User-configurable movement, blinking, and animated text scrolling remain future work; duration/easing are centralized for later adjustments.
- Real score calculations/history, Dashboard, Settings, and configurable durations require later version plans. Mock Score and its Next/Retry controls are implemented by the score follow-up below.
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

## Typing-page follow-up — 2026-10-24

- Created `codex/typing-page-update` before implementation; existing untracked icons and reference image were retained.
- Read Figma frame `54:22` and control groups `54:40` / `54:43`, comparing the local `Typing Page.png`. Adopted the dark palette, 16px/20px typing text, 640 × 150 typing area, branding, four inert top-right controls, and footer. Word mode is blue because the current session is 50 words; time mode remains inactive.
- Embedded all ten supplied SVGs and Jomhuria Regular with its OFL license. Iced's `svg` feature adds its required transitive rendering dependencies; there is no new direct dependency.
- Added a completed-target-word counter above the measured text origin. It counts each entered final letter regardless of correctness, reverses with Backspace, resets on Tab, and reaches 50/50 without navigating away.
- Github, Twitter, and Discord launch their respective URLs using the OS browser (updated on 2026-10-25). Browser launch runs off the input thread; the process is reaped. Version/theme labels are informational.
- Preserved synchronous native input, engine/layout boundaries, terminal completion, wrapping, centering, and viewport behavior. No score page or mode switching was implemented.
- Validation completed on 2026-10-25: formatting, strict Clippy, all 19 automated tests (13 library, 4 input policy, 2 integration), and locked debug/release builds passed on Arch Linux. `git diff --check` passed. The native debug window was launched outside the checkout on XWayland and its startup screenshot inspected at 125% scaling: bundled branding/icons, dark palette, blue word mode, 0/50 counter, and footer rendered correctly. Synthetic input attempts did not establish native counter/click behavior, so those runtime checks remain unverified; counter boundaries, mistakes, Backspace, and terminal completion are covered by automated tests. Native Windows, native Wayland, browser handoff, IME, and physical input remain unverified for this follow-up. No score page or packaged release was added.

## Dracula palette extraction — 2026-10-25

- Created `themes/dracula.json` with all 12 colors exactly as shown in the supplied palette image, including Current Line `#6272A4`. Kept the required blue accent separate from the reference palette.
- Added build-time JSON validation and generation of native Iced constants. All UI color roles and SVG tints now use the JSON; the footer shows Dracula. Pending text uses Comment, errors use Red, and the caret uses Cyan.
- Added `serde_json` as a build-only dependency; no runtime JSON parser, theme switching, or filesystem access was introduced. Rebuild after changing the JSON.
- Validation: formatting, strict Clippy, all 19 existing automated tests, and locked debug/release builds passed on Arch Linux. An isolated build-script check confirmed that changed JSON colors propagate to generated constants, while malformed hex and unknown color references fail. The source JSON was unchanged by those checks. No new native GUI or Windows verification was performed for this palette-only change.

## Typing area maximum width — 2026-10-25

The typing panel now stays horizontally centered and grows to at most 700 logical pixels wide, including its inner padding. Smaller windows retain the 30px exterior margins (640px panel at the 700px minimum window width). Height and measured text wrapping retain their existing behavior.

Validation: formatting, strict Clippy, all 19 automated tests, and locked debug/release builds passed on Arch Linux. No new GUI screenshot or native Windows verification was performed for this change. Existing platform limitations and planned next-alpha work remain unchanged.

## Words and Time modes — 2026-10-25

- Both modes sample 500 words with replacement from the bundled 200-word list. Words remains the default; the counter reads `0/500` initially.
- Selecting a different mode resets text, progress, caret/viewport, and timer. Clicking the active mode preserves progress. Tab restarts the current mode. Active mode icons use the existing JSON blue accent; punctuation/number controls remain inert.
- Time mode displays `30s` in the counter position and starts on the first accepted printable character (including mistakes or spaces). Backspace, shortcuts, controls, and focus changes do not start or restart it.
- A monotonic deadline avoids accumulated tick drift. Whole seconds round upward; delayed redraws catch up, and focus loss does not pause time. At `0s`, scheduled ticks stop, text and input remain available, and no score page opens. Timer ticks do not disturb manual scrolling.
- The page owns mode/countdown policy and supplies monitor text and the next redraw deadline to the widget. Synchronous keyboard delivery, pure engine/layout boundaries, and the 700px panel cap remain intact. No dependency was added.
- Next alpha: score/results behavior and configurable durations remain deferred. Existing platform and packaging limitations still apply.

Validation: formatting, strict Clippy, all 22 automated tests (13 library, 7 app policy, 2 integration), and locked debug/release builds passed on Arch Linux. The new tests cover 500-word generation, mode changes and active-mode no-ops, Tab resets, countdown startup and elapsed-time catch-up, focus loss, expiry, and continued editing after zero. `git diff --check` passed.

Native XWayland at 125% scaling: launched the debug executable outside the checkout and inspected `0/500`, Time mode's blue clock icon and `30s` monitor, automatic countdown to `28s` and `0s`, unchanged text at expiry, further input after expiry, and Tab resetting Time mode to `30s` with fresh text. Synthetic pointer positioning was unreliable; returning to Words through a native click was not conclusively verified, though both mode transitions passed app-state tests and button-message wiring was reviewed. Native Windows, native Wayland, physical input, and packaging remain unverified.

## Four-row viewport and 50-word target — 2026-10-25

- Cap the centered panel at 700 × 110 logical pixels. Four 20px text rows plus 15px inner padding on each vertical edge fit within 110px; the faded preview is included in the four-row limit. The cap applies in both modes and at maximized sizes.
- Both modes still generate 500 words. Words displays completed words capped at `50/50`, independently of the buffer size, and remains editable after reaching the target. Backspace below 50 completed words lowers the count again. Tab and mode changes reset progress. Exhausting the entire 500-word buffer retains the existing terminal engine behavior.
- Preserve native input delivery, measured wrapping, viewport following, preview opacity, and the Time countdown. Score/results behavior and configurable targets remain deferred.

Validation: formatting, strict Clippy, all 23 automated tests (13 library, 8 app policy, 2 integration), and locked debug/release builds passed on Arch Linux. The new boundary test confirms the 500-word buffer remains active after the 50-word target, the monitor caps at `50/50` even with errors, Backspace below the target lowers it, and Tab resets it to `0/50`. `git diff --check` passed.

Native XWayland screenshots at 125% scale confirmed four centered rows, including the faded fourth preview row, at the 875 × 500 physical-pixel minimum and a maximized 1920 × 1050 client area. After 500 targeted keypresses, the counter displayed `50/50` with further words available and the four-row viewport intact. Native Windows and native Wayland remain unverified; earlier input/platform limitations still apply.

## Punctuation and number toggles — 2026-10-25

- The existing punctuation and number buttons now independently toggle generation options, defaulting off. Enabled icons use the JSON accent blue; both can be enabled together in either mode.
- Changing either option generates a fresh 500-token buffer and resets progress, viewport, and countdown. The other option and current mode remain selected. Tab and mode changes preserve both options for the current app run; no disk persistence was added.
- Numbers replaces one random position per ten-token group with an integer from 0 to 9999. Punctuation adds a trailing comma, period, exclamation mark, question mark, semicolon, or colon at one random position per four-token group. These small groups ensure enabled features appear near the start as well as throughout the buffer.
- Numeric entries count as one word, and a punctuated token completes after its punctuation is entered. Both features retain the existing correctness, Backspace, 50-word display cap, 30-second countdown, 700px width, and four-row viewport behavior.
- Generation remains seeded and Iced-independent. No dependencies or new pages were added; score/results, configurable targets/durations, and packaging remain future work.

Validation: formatting, strict Clippy, all 26 automated tests (14 library, 10 app policy, 2 integration), and locked debug/release builds passed on Arch Linux. Seeded tests exercise all four feature combinations, exact token/separator counts, early feature presence, and valid generated content. App tests cover independent toggles, session/timer resets in both modes, preserving settings across Tab/mode changes, and committed punctuation/digit input with word counting and Backspace. `git diff --check` passed. Native feature-toggle clicks and their visual states were not conclusively verified for this follow-up; earlier XWayland timer/viewport evidence remains recorded above. Native Windows, native Wayland, and packaged releases remain unverified.

## Connection URLs — 2026-10-25

- Footer buttons now pass their individual destination to the existing OS browser launcher: Github `https://github.com/Arik-Ahnaf/Typerr`, Twitter `https://x.com/ArikAhnaf1`, and Discord `https://discord.gg/FYehF2KCw4`.
- Validation: formatting, strict Clippy, all 26 existing automated tests, locked debug/release builds, and `git diff --check` passed on Arch Linux. Button-to-URL wiring was reviewed; native browser handoff and Windows were not exercised. Existing limitations and next-alpha scope remain unchanged.

## Startup font configuration — 2026-10-25

- Added `settings.json` with only `font-size`, default 16 logical pixels. `src/settings.rs` loads once at startup from the working directory, falling back to beside the executable. Missing/invalid settings use defaults; invalid/unreadable files emit diagnostics. Valid sizes are 8–48, including fractions. Edits require restart, not rebuild.
- Reused locked `serde_json` as a runtime dependency. Application state passes settings into the native typing widget; engine and pure layout retain their boundaries. Tests use defaults without reading user configuration.
- Font measurement and glyph rendering share the selected size. Line spacing, caret height, error positions, pixel scrolling, and clipping use scaled metrics. Cache invalidation includes font size. Panel width remains capped at 700px; height scales with font size to retain four whole rows. Startup/minimum window height grows when needed to preserve exterior clearance. Header/footer and monitor typography remain unchanged.
- Validation: formatting, strict Clippy, all 28 tests (including defaults, fractional sizes, malformed JSON, wrong types, and range boundaries), locked debug/release builds, and `git diff --check` passed on Arch Linux. No native GUI or Windows verification was performed for this change. Existing platform limitations and next-alpha scope remain unchanged.

## Four rows at configurable font sizes — 2026-10-25

Removed the fixed 110px panel height cap. Panel height now fits four configured line heights plus 30px inner padding, rounded upward for fractional sizes. At 24px the panel is 150px high and still fits a 700 × 400 window. Larger fonts increase startup/minimum window height as needed to retain the existing exterior clearance; 48px uses 520px window height. Settings are loaded once and shared between startup geometry and app state. The user's 24px setting was preserved.

Validation: formatting, strict Clippy, all 29 tests, locked debug/release builds, and `git diff --check` passed. A regression test checks four whole viewport rows and minimum-window space at 4,001 font sizes from 8–48px, including the reported 24px case. No native GUI or Windows check was performed; existing platform limitations and next-alpha scope remain unchanged.

## Smooth caret movement — 2026-10-25

- Read Monkeytype's official [`test/caret.ts`](https://github.com/monkeytypegame/monkeytype/blob/master/frontend/src/ts/test/caret.ts) and delegated [`elements/caret.ts`](https://github.com/monkeytypegame/monkeytype/blob/master/frontend/src/ts/elements/caret.ts) before implementation. Adopted its medium 100 ms duration and `inOut(1.25)` power easing in native Rust; no JavaScript runtime or animation dependency was added.
- Caret state holds panel-relative start, rendered, and target positions with an injected monotonic timestamp. New edits sample the current movement before retargeting; unchanged layout does not restart it. Native redraws advance only presentation state and stop scheduling animation frames at the exact target.
- Shared measured geometry supplies glyphs and caret destinations. Immediate text scrolling/recentering is retained, with smooth caret returns across wraps in both directions. Separate clipping keeps the moving caret within the padded panel without restricting it to the new text origin.
- A separate generation counter distinguishes session resets even for identical target text. Startup, Tab/mode/toggle resets, geometry changes, focus loss, wheel inspection, and the first accepted edit after inspection settle immediately. Terminal completion lets its final movement finish; ignored input preserves inspection. Window resize/scale events restore following even when a capped panel's size is unchanged.
- Preserved the engine/layout interfaces, synchronous keyboard delivery, countdown semantics, crate version `0.1.0`, solid cyan caret, and the user's 24px setting. No new configuration, controls, dependencies, packaging, push, or publication.
- Validation: all 41 tests passed (14 library, 25 binary, 2 integration), including 12 new animation/presentation/input tests. Formatting, strict Clippy, locked debug/release builds, and whitespace review are recorded in `BUILD.md`.
- Native Arch/XWayland captures at 125% scale and 16px/24px fonts recorded intermediate movement, rapid retargeting, forward/backward wraps, restart during movement, resize, and terminal completion. Matching Backspaces restored the exact initial caret after rapid input and 100-event bursts; completed-window pixels were unchanged by further input/Backspace. Native wheel/focus interactions, Windows, native Wayland, physical held-key repeat, and IME remain unverified for this follow-up; deterministic tests cover wheel/focus/countdown policy.

## Score page follow-up — 2026-10-27

User-authorized scope supersedes earlier typing-only completion rules. Implemented native Score from Figma `59:3` and graph template `94:66`: fixed 700 × 400 composition, embedded fonts and the supplied next/restart SVGs (plus hash.svg for the numbers toggle), hardcoded WPM/ACC/Raw/Consistency/Time, and a Canvas plot with monotone cubic WPM/raw curves, error markers/axis, grid, and PB guide. The green placeholder is removed, tick 3 restored, and template colors adapted to the existing theme. Canvas enables six locked transitive geometry dependencies, with no additional direct dependency.

Completion freezes input at 50 words or 30 seconds (or buffer exhaustion) and performs a smoothstep 300 ms crossfade. Expiry precedes input even after a missed redraw, and committed text stops at the exact completion character. App presentation state is explicit; the native page host forwards keyboard/focus once and schedules countdown/transition redraws while the typing widget retains caret scheduling. Shared branding stays steady.

Next/Tab generate new text; Retry reconstructs the identical target. All preserve mode/options, reset progress/timer/caret/viewport, and increment session generation. Pointer actions are disabled during the fade; Tab may cancel it. Fixed score data and typography are independent of session duration and typing font size. Existing packaging work is preserved; package regeneration, version changes, publication, real statistics, and persistence are excluded.

Validation on Arch Linux:

- All 47 automated tests pass (14 library, 31 app/widget, 2 integration). Coverage includes the exact 50th-word boundary with mistakes/punctuation, discarded surplus committed text, expiry before input, delayed/focus-loss catch-up, elapsed-time fade endpoints/midpoint/cancellation, fresh/identical-target resets, native event publication and scheduling, graph mapping, and bounded interpolation.
- Formatting, strict all-target Clippy, locked debug/release builds, and `git diff --check` pass. No version change, package rebuild, push, or release was performed.
- Native XWayland rendering at 125% scale was exercised by an isolated temporary harness that drives the production app messages and captures Iced's own renderer. Captures cover 16/24/48px typing settings (875 × 500, 875 × 500, and 875 × 650 physical startup sizes), intermediate fades, frozen results, Next/Retry/Tab, and a larger 1000 × 650 logical window. The complete 30-second timer was allowed to expire. Final time-mode output is pixel-identical to the corresponding words result.
- Pixel comparisons confirm successful edits, Backspace/Retry restoring the exact initial text, Next producing new text, ignored result edits, intermediate crossfade frames, and unchanged branding throughout the fade. Visually inspected the graph, all labels/axes, replacement next/restart/hash assets, larger score layout, and pre-expiry timer. Score remains independent of the typing font size.
- Initial external X11 automation was unreliable: core pointer coordinates differed from compositor coordinates, focus changed, and drawable captures returned stale content. Some Next/Retry native messages were observed, but the final evidence uses deterministic app-message injection and native renderer screenshots; it is not a claim of comprehensive physical input/click verification. The temporary harness was removed from `src/` after capture; no debug hooks remain in production code.
- Evidence: `target/score-verification/render-{16,24,48}/`, `render-results.json`, the preserved temporary `score_verify.rs`, and check/build logs. These are generated local artifacts. Native Windows, native Wayland, physical held-key/IME behavior, and comprehensive real-pointer control checks remain unverified. Real score calculation/history is the next feature work, subject to separate authorization.
