# Typerr: instructions for coding agents

## Vision and current boundary

Typerr is a native Rust typing application inspired by Monkeytype. Responsive input, accurate text layout, and eventually smooth caret movement take priority over feature count. Target platforms are Windows and Arch Linux. Develop through small, independently testable alpha versions.

The current typing-page follow-up (`0.1.0`) contains the Main Typing page with Words and Time modes, each generating 500 words. Words is the startup mode, with a 50-word displayed target independent of the 500-word buffer. The counter caps at `50/50`; reaching 50 does not stop input or navigate. Clicking a different mode starts a fresh session; clicking the current mode does nothing. Tab restarts the current mode, including after completion. The selected mode icon and enabled punctuation/number icons use the blue accent. Punctuation and numbers are independent, default-off toggles that work together in either mode. Changing a toggle resets the session and timer while preserving the mode and the other toggle; selections survive Tab and mode changes for the current app run. Time mode replaces the blue word counter with a 30-second countdown, starting on the first accepted printable input. It uses monotonic elapsed time, continues while unfocused, and stays at `0s` after expiry without stopping input or navigating. Do not implement Score, Dashboard, Settings, on-screen restart controls, persistence, or application networking. Keep future pages easy to add by separating page state and view responsibilities; do not create empty pages or a generic routing framework.

The UI must remain native. Do not add HTML, CSS, JavaScript, webviews, or embedded browsers.

## Technical decisions

- Rust edition 2024, pinned toolchain `1.98.1`.
- Iced `0.14.0`, native default render backends plus the `advanced` and `svg` features for a custom typing widget.
- `rand` `0.10.3` for word selection, with injectable seeded randomness for tests.
- Bundle an original curated list of approximately 200 common lowercase ASCII English words and the existing static Google Sans Code Regular font. Load both from compiled assets; do not require the source checkout or network at runtime.
- Commit `Cargo.lock`. Add another dependency only for a concrete need; no additional shaping crate is planned.
- Measure the embedded font through Iced's renderer/paragraph facilities. Do not estimate character advance from font size. SVG support brings Iced's transitive SVG dependencies; no extra application shaping crate is used. The current English target is monospaced ASCII; future general Unicode layout will require a deliberate extension.

## Responsibilities

Keep modules small and cohesive. The initial structure is:

| Module | Responsibility |
| --- | --- |
| `src/main.rs` | Startup, embedded font registration, window settings, and application launch |
| `src/lib.rs` | Expose independently testable library modules |
| `src/engine.rs` | Target characters, entered characters, logical position, correctness, and terminal completion; no Iced dependency |
| `src/words.rs` | Embedded list validation and 500-token generation and punctuation/number options using an injected RNG |
| `src/layout.rs` | Pure measured-cell wrapping, visual lines, character/caret positions, and separator ownership; no Iced dependency |
| `src/app.rs` | Application state, native events, focus/input policy, mode/countdown/generation-option state, and Main Typing page composition |
| `src/ui/typing_panel.rs` | Native widget, measured font metrics, glyph/error drawing, viewport, clipping, and scroll interaction |
| `src/ui/caret.rs` | Rendered caret coordinates separate from logical input position and layout target |
| `src/ui/theme.rs` | Centralized visual dimensions, font, and colors |

The implementation also uses `tests/session_layout.rs` to exercise engine/layout/viewport boundaries together. The Main Typing page owns an edit revision that changes only after successful input, Backspace, an explicit Tab restart, a mode change, or a generation-option toggle; the widget uses it to restore automatic following without disturbing manual inspection after no-op events. Countdown redraws do not change the edit revision or reset manual scrolling. The widget schedules the next countdown second with `Shell::request_redraw_at` and forwards due redraw timestamps to the page; no async keyboard subscription or extra timer dependency is used. Widget tree state retains viewport and caret presentation state across views. The widget forwards keyboard and window-focus events synchronously with `Shell::publish`; do not route typing through `iced::event::listen`, whose bounded asynchronous queue dropped keystrokes during a native burst-input check.

Small adjustments to this structure are acceptable when documented, but preserve the engine/layout/UI boundaries. Pure layout takes measured cell advance and geometry as inputs; it does not create Iced paragraphs itself. Text, error markers, and the caret must use the same layout result.

## Typing invariants

- Generate exactly 500 space-separated tokens in either mode. Plain words are sampled with replacement from the bundled list; duplicates are allowed. Numbers replaces one randomly positioned word in each group of ten with an integer from 0 to 9999. Punctuation appends one of `, . ! ? ; :` to one randomly positioned token in each group of four. Both options can apply to the same token. Join tokens with exactly 499 ASCII spaces and no trailing space. Each token, including its trailing punctuation, counts as one word for progress.
- Represent logical positions as character indices, not UTF-8 byte offsets. Store entered characters; derive progress from their count rather than maintaining redundant counters.
- Every printable input scalar consumes one target position, including incorrect input. Spaces are ordinary input positions. A wrong character at an expected space and a space at an expected letter both advance and are incorrect.
- Continue to display the expected glyph after mistakes; color it red. Give incorrect expected spaces a visible red cell treatment or underline.
- Backspace removes exactly one entered character, including across word boundaries. At position zero it is a no-op.
- Entering the last target character completes the test, even when incorrect. Completion is terminal: further typing and Backspace do nothing. Preserve the text; do not show a dialog, change pages, or automatically restart. A fresh unmodified Tab keypress explicitly replaces the session and resets progress. Ignore held-Tab repeats and modified Tab shortcuts.
- Process native committed keyboard text exactly once. Handle Backspace separately, ignore control characters and shortcut combinations, and use native repeat events without artificial delays. Do not synthesize text from physical key codes.

## Layout and viewport invariants

- Startup and minimum window size are 700 × 400 logical pixels. The panel is centered on both axes, 640 × 110 at the minimum window size, and grows to at most 700 × 110 logical pixels. Its height includes four 20px text rows and 15px top/bottom inner padding. Keep at least 30px horizontal and 125px vertical exterior clearance; retain a fixed 16px font and 20px line height.
- Always center the visible text block horizontally and vertically within the available area, including after scrolling, resizing, restart, and completion. Keep lines left-aligned within the centered block; glyphs, underlines, and caret share its origin. Rebuild cached layout when target text changes on restart.
- Use 20px horizontal and 15px vertical inner padding and word wrapping based on measured monospaced advance. Reserve space for the caret and separator/error cells. Retain a safe character-wrap fallback for unexpectedly long words.
- Each visual line owns the separator after its final word. Stay on that line until the separator position is consumed; then put the next active line at the viewport top. Incorrect input at the separator also consumes it under the engine rules.
- Show at most four whole text rows (including the preview row): use the floor of available text height divided by line height. Distribute spare vertical space around the centered block and clip overflow. Never expose a partially clipped bottom row.
- After every completed visual line, its successor becomes the top row and completed text exits above. This is not conventional scroll-only-when-the-caret-reaches-the-bottom behavior.
- If more text exists below the viewport, render its bottom visible preview row at 50% opacity. Multiply the base glyph/error alpha by 0.5; do not halve row height. A line becomes opaque as it moves upward. Do not fade the active line, and do not fade the final visible line when there is no additional text below the viewport.
- Wheel scrolling temporarily permits inspecting previous or later text in whole-line increments. The next accepted input, successful Backspace, or resize restores the active line to the top. No-op input leaves manual inspection unchanged.
- On resize, recompute wrapping from the same logical position. Backspace across a wrap restores the preceding active line to the top.
- At completion, retain the final character's line as the terminal anchor and place the caret after its last glyph. Do not scroll to an empty sentinel line. Manual scrolling remains available after completion.
- Keep logical position in the engine, target caret coordinates in the layout result, and rendered coordinates in UI state. Alpha 0.1 snaps the rendered caret to its target. Later easing, duration, frame scheduling, and line-transition policy must be addable without changing typing rules or wrapping.

## Visual reference

Primary reference: [Typerr Figma](https://www.figma.com/design/80x3SZRgeqDp8xOz7FzGdU/Typerr?node-id=0-1), current frame `54:22`, text `54:28`, counter `71:5`. `design_references/Typing Page.png` is the current rendered reference; the older Outline file marks layout areas only.

References whose frame or image name ends in “Outline” use colorful rectangles only to mark general layout areas. Preserve their placement and spacing, but do not render those rectangle fills or borders as decoration. The typing panel is transparent and borderless over background `#282A36`, using Google Sans Code Regular 16px and reference foreground `#F8F8F2`. Use the bundled Jomhuria Regular at 48px for branding, `#44475A` control groups, and `#0088FF` for the selected mode, enabled features, and counter/timer. The counter counts target words whose final character has been entered, regardless of correctness, and decreases when Backspace removes that character. In Words mode the display is capped at 50/50 while typing continues into the remaining buffer, and resets on Tab. Backspace reduces it only once fewer than 50 words are completed. In Time mode the same location displays the remaining seconds, from `30s` to `0s`. Footer connections open the OS browser at https://google.com; version/theme labels are informational. This user clarification supersedes the initial interpretation of the cyan panel rectangle.

Interaction defaults, not separately specified by the Figma placeholder: pending `#6272A4`, correct `#F8F8F2`, incorrect `#FF5555`, and thin caret `#8BE9FD`. Base colors are opaque; preview opacity is an additional multiplier. Use `assets/fonts/Google_Sans_Code/static/GoogleSansCode-Regular.ttf` and preserve its OFL notice. Bundle `assets/fonts/Jomhuria/Jomhuria-Regular.ttf` with its OFL notice and embed the supplied SVGs. `src/ui/chrome.rs` owns the native header/footer composition.

## Workflow and quality

Read this file, `PLAN.md`, `BUILD.md`, and `PACKAGING.md` before implementation. Follow the current alpha's plan; record changed decisions in all affected documentation. Do not expand scope to future versions without explicit authorization.

Use idiomatic Rust, explicit ownership, standard formatting, and focused tests. Avoid speculative abstractions and needless dependencies. Keep engine and layout tests deterministic. Treat renderer metrics as the shared source of truth rather than maintaining separate caret arithmetic.

Run formatting checks, strict Clippy, tests, and builds using `BUILD.md`. Fix issues and update `PLAN.md` with actual outcomes. Clearly distinguish a compile check, automated behavior test, GUI launch, screenshot inspection, and native-platform verification. Do not call Windows supported because Linux compiled, or call visual behavior verified because unit tests passed.

Before delivery, review all acceptance criteria, architecture boundaries, test results, and documentation. Record remaining limitations and recommended next alpha work. Do not push or publish releases unless separately requested.

## Dracula palette source

`themes/dracula.json` is the color source of truth, copied from `design_references/Dracula Color Pallet.png`. Preserve the image's Current Line value `#6272A4`. Its separate blue accent retains the explicitly requested mode/counter color. The JSON maps UI roles to palette names; `build.rs` validates the hex values and mappings using build-only `serde_json` and generates native color constants imported by `src/ui/theme.rs`. JSON edits require rebuilding; there is no runtime filesystem dependency or theme selector. SVG tinting uses the same constants. The footer displays the theme's JSON name.
