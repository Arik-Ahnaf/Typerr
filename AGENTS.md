# Typerr: instructions for coding agents

## Vision and current boundary

Typerr is a native Rust typing application inspired by Monkeytype. Responsive input, accurate text layout, and eventually smooth caret movement take priority over feature count. Target platforms are Windows and Arch Linux. Develop through small, independently testable alpha versions.

Alpha 0.1 (`0.1.0`) contains only the Main Typing page and a 50-word typing test. Tab restarts the race with 50 newly sampled words, including after completion. Do not implement navigation, Score, Dashboard, Settings, mode controls, icons, branding, decorative assets, statistics, timers, on-screen restart controls, persistence, or networking. Keep future pages easy to add by separating page state and view responsibilities; do not create empty pages or a generic routing framework.

The UI must remain native. Do not add HTML, CSS, JavaScript, webviews, or embedded browsers.

## Technical decisions

- Rust edition 2024, pinned toolchain `1.98.1`.
- Iced `0.14.0`, native default render backends plus the `advanced` feature for a custom typing widget.
- `rand` `0.10.3` for word selection, with injectable seeded randomness for tests.
- Bundle an original curated list of approximately 200 common lowercase ASCII English words and the existing static Google Sans Code Regular font. Load both from compiled assets; do not require the source checkout or network at runtime.
- Commit `Cargo.lock`. Add another dependency only for a concrete need; no additional shaping crate is planned.
- Measure the embedded font through Iced's renderer/paragraph facilities. Do not estimate character advance from font size. The current English target is monospaced ASCII; future general Unicode layout will require a deliberate extension.

## Responsibilities

Keep modules small and cohesive. The initial structure is:

| Module | Responsibility |
| --- | --- |
| `src/main.rs` | Startup, embedded font registration, window settings, and application launch |
| `src/lib.rs` | Expose independently testable library modules |
| `src/engine.rs` | Target characters, entered characters, logical position, correctness, and terminal completion; no Iced dependency |
| `src/words.rs` | Embedded list validation and 50-word generation using an injected RNG |
| `src/layout.rs` | Pure measured-cell wrapping, visual lines, character/caret positions, and separator ownership; no Iced dependency |
| `src/app.rs` | Application state, native events, focus/input policy, and Main Typing page composition |
| `src/ui/typing_panel.rs` | Native widget, measured font metrics, glyph/error drawing, viewport, clipping, and scroll interaction |
| `src/ui/caret.rs` | Rendered caret coordinates separate from logical input position and layout target |
| `src/ui/theme.rs` | Centralized visual dimensions, font, and colors |

The implementation also uses `tests/session_layout.rs` to exercise engine/layout/viewport boundaries together. The Main Typing page owns an edit revision that changes only after successful input, Backspace, or an explicit Tab restart; the widget uses it to restore automatic following without disturbing manual inspection after no-op events. Widget tree state retains viewport and caret presentation state across views. The widget forwards keyboard and window-focus events synchronously with `Shell::publish`; do not route typing through `iced::event::listen`, whose bounded asynchronous queue dropped keystrokes during a native burst-input check.

Small adjustments to this structure are acceptable when documented, but preserve the engine/layout/UI boundaries. Pure layout takes measured cell advance and geometry as inputs; it does not create Iced paragraphs itself. Text, error markers, and the caret must use the same layout result.

## Typing invariants

- Generate exactly 50 words, sampled independently with replacement; duplicates are allowed. Join them with exactly 49 ASCII spaces and no trailing space.
- Represent logical positions as character indices, not UTF-8 byte offsets. Store entered characters; derive progress from their count rather than maintaining redundant counters.
- Every printable input scalar consumes one target position, including incorrect input. Spaces are ordinary input positions. A wrong character at an expected space and a space at an expected letter both advance and are incorrect.
- Continue to display the expected glyph after mistakes; color it red. Give incorrect expected spaces a visible red cell treatment or underline.
- Backspace removes exactly one entered character, including across word boundaries. At position zero it is a no-op.
- Entering the last target character completes the test, even when incorrect. Completion is terminal: further typing and Backspace do nothing. Preserve the text; do not show a dialog, change pages, or automatically restart. A fresh unmodified Tab keypress explicitly replaces the session and resets progress. Ignore held-Tab repeats and modified Tab shortcuts.
- Process native committed keyboard text exactly once. Handle Backspace separately, ignore control characters and shortcut combinations, and use native repeat events without artificial delays. Do not synthesize text from physical key codes.

## Layout and viewport invariants

- Startup and minimum window size are 700 × 400 logical pixels. At that size reproduce the reference's centered 640 × 250 panel with 30px horizontal and 75px vertical exterior margins. Let the panel grow with the window; retain a fixed 20px font and 25px line height.
- Always center the visible text block horizontally and vertically within the available area, including after scrolling, resizing, restart, and completion. Keep lines left-aligned within the centered block; glyphs, underlines, and caret share its origin. Rebuild cached layout when target text changes on restart.
- Use approximately 20px inner padding and word wrapping based on measured monospaced advance. Reserve space for the caret and separator/error cells. Retain a safe character-wrap fallback for unexpectedly long words.
- Each visual line owns the separator after its final word. Stay on that line until the separator position is consumed; then put the next active line at the viewport top. Incorrect input at the separator also consumes it under the engine rules.
- Show only whole text rows: use the floor of available text height divided by line height. Distribute spare vertical space around the centered block and clip overflow. Never expose a partially clipped bottom row.
- After every completed visual line, its successor becomes the top row and completed text exits above. This is not conventional scroll-only-when-the-caret-reaches-the-bottom behavior.
- If more text exists below the viewport, render its bottom visible preview row at 50% opacity. Multiply the base glyph/error alpha by 0.5; do not halve row height. A line becomes opaque as it moves upward. Do not fade the active line, and do not fade the final visible line when there is no additional text below the viewport.
- Wheel scrolling temporarily permits inspecting previous or later text in whole-line increments. The next accepted input, successful Backspace, or resize restores the active line to the top. No-op input leaves manual inspection unchanged.
- On resize, recompute wrapping from the same logical position. Backspace across a wrap restores the preceding active line to the top.
- At completion, retain the final character's line as the terminal anchor and place the caret after its last glyph. Do not scroll to an empty sentinel line. Manual scrolling remains available after completion.
- Keep logical position in the engine, target caret coordinates in the layout result, and rendered coordinates in UI state. Alpha 0.1 snaps the rendered caret to its target. Later easing, duration, frame scheduling, and line-transition policy must be addable without changing typing rules or wrapping.

## Visual reference

Primary reference: [Typerr Figma](https://www.figma.com/design/80x3SZRgeqDp8xOz7FzGdU/Typerr?node-id=0-1), frame `1:2`, typing panel `7:60`, text `7:67`. Figma context and screenshots were inspected during planning; `design_references/Typing Page Outline.png` matches them.

References whose frame or image name ends in “Outline” use colorful rectangles only to mark general layout areas. Preserve their placement and spacing, but do not render those rectangle fills or borders as decoration. The typing panel is transparent and borderless over background `#333333`, using Google Sans Code Regular 20px and reference foreground `#F2F2F2`. Exclude the reference's branding, modes, and bottom controls because they are outside Alpha 0.1. This user clarification supersedes the initial interpretation of the cyan panel rectangle.

Interaction defaults, not separately specified by the Figma placeholder: pending `#B4B4B4`, correct `#F2F2F2`, incorrect `#FF6B6B`, and thin caret `#00FDE8`. Base colors are opaque; preview opacity is an additional multiplier. Use `assets/fonts/Google_Sans_Code/static/GoogleSansCode-Regular.ttf` and preserve its OFL notice.

## Workflow and quality

Read this file, `PLAN.md`, `BUILD.md`, and `PACKAGING.md` before implementation. Follow the current alpha's plan; record changed decisions in all affected documentation. Do not expand scope to future versions without explicit authorization.

Use idiomatic Rust, explicit ownership, standard formatting, and focused tests. Avoid speculative abstractions and needless dependencies. Keep engine and layout tests deterministic. Treat renderer metrics as the shared source of truth rather than maintaining separate caret arithmetic.

Run formatting checks, strict Clippy, tests, and builds using `BUILD.md`. Fix issues and update `PLAN.md` with actual outcomes. Clearly distinguish a compile check, automated behavior test, GUI launch, screenshot inspection, and native-platform verification. Do not call Windows supported because Linux compiled, or call visual behavior verified because unit tests passed.

Before delivery, review all acceptance criteria, architecture boundaries, test results, and documentation. Record remaining limitations and recommended next alpha work. Do not push or publish releases unless separately requested.
