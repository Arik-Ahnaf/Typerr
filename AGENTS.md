# Typerr: instructions for coding agents

## Vision and current boundary

Typerr is a native Rust typing application inspired by Monkeytype. Responsive input, accurate text layout, and smooth caret movement take priority over feature count. Target platforms are Windows and Arch Linux. Develop through small, independently testable alpha versions.

The current updater follow-up (`1.1.0`) contains Main Typing, Score, and Dashboard pages with Words and Time modes, each generating 500 words. Words is the startup mode, with a selectable target (default 50) independent of the 500-word buffer. Words finishes when the final character of the selected final word is entered (including punctuation, regardless of correctness). Input freezes and the page crossfades to Score over 300 ms. Clicking a different mode starts a fresh session; clicking the current mode does nothing. Tab restarts the current mode, including after completion. The selected mode icon and enabled punctuation/number icons use the foreground color; inactive icons use Comment. Punctuation and numbers are independent, default-off toggles that work together in either mode. Changing a toggle resets the session and timer while preserving the mode and the other toggle; selections survive Tab and mode changes for the current app run. Time mode replaces the blue word counter with a selectable countdown (default 30 seconds), starting on the first accepted printable input. It uses monotonic elapsed time, continues while unfocused, and finishes at expiry, freezing input and crossfading to Score. Expiry is checked before keyboard edits as well as on redraws; exhausting the entire buffer also finishes either mode. Score uses measured session statistics and graph samples as authorized on 2026-10-28; completed-test SQLite history and Dashboard read queries are authorized by the 2026-11-03 follow-up. The 2026-11-04 follow-up authorizes the native Dashboard with hardcoded profile data; the Name Setter follow-up authorizes saving the user name in settings.json. The 2026-11-05 Profile Picture Chooser follow-up authorizes selecting, cropping, and saving an avatar. The 2026-11-06 follow-up authorizes a native Settings page and explicit Save to settings.json; live application of its parameters and remaining profile fields stay excluded. The startup updater follow-up authorizes GitHub release checks, accepted downloads/installations, and temporary updater state. Score has Next (new random target) and Retry (identical target) controls; both and Tab return immediately to Typing, preserving mode/options and clearing input/timer/presentation state. Ignore pointer actions during the fade; unmodified non-repeated Tab may cancel it. Startup-only reading of `settings.json` is authorized for typing font size. Keep future pages easy to add by separating page state and view responsibilities; do not create empty pages or a generic routing framework.

The UI must remain native. Do not add HTML, CSS, JavaScript, webviews, or embedded browsers.

## Technical decisions

- Rust edition 2024, pinned toolchain `1.98.1`.
- Iced `0.14.0`, native default render backends plus the `advanced`, `svg`, `canvas`, and `image-without-codecs` features for a custom typing widget.
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
| `src/storage.rs`, `src/storage/` | Versioned SQLite schema, typed Dashboard queries, user-data paths, and a background score writer |
| `src/statistics.rs` | Pure elapsed-time measurements, frozen results, and graph samples; no Iced dependency |
| `src/ui/dashboard.rs` | Native Dashboard cards, profile placeholder, best results, and all-time WPM histogram |
| `src/ui/score.rs` | Native Canvas graph, measured score presentation and Next/Retry controls |
| `src/ui/page_host.rs` | Synchronous keyboard/focus forwarding and page-level countdown/crossfade redraw scheduling |
| `src/ui/caret.rs` | Monotonic caret animation and panel-relative presentation coordinates, separate from logical input and measured layout |
| `src/ui/theme.rs` | Centralized visual dimensions, font, and colors |

The implementation also uses `tests/session_layout.rs` to exercise engine/layout/viewport boundaries together. The Main Typing page owns an edit revision that changes only after successful input, Backspace, an explicit Tab restart, a mode change, or a generation-option toggle; the widget uses it to restore automatic following without disturbing manual inspection after no-op events. Countdown redraws do not change the edit revision or reset manual scrolling. The page host schedules the next countdown second with `Shell::request_redraw_at` and forwards due redraw timestamps to the page; no async keyboard subscription or extra timer dependency is used. Widget tree state retains viewport and caret presentation state across views. The page host forwards keyboard and window-focus events synchronously exactly once with `Shell::publish`; do not route typing through `iced::event::listen`, whose bounded asynchronous queue dropped keystrokes during a native burst-input check.

Small adjustments to this structure are acceptable when documented, but preserve the engine/layout/UI boundaries. Pure layout takes measured cell advance and geometry as inputs; it does not create Iced paragraphs itself. Text, error markers, and the caret must use the same layout result.

The page also owns a session-generation counter, incremented only when replacing a session; do not infer resets from text equality or logical position. Caret animation uses native redraw timestamps and requests another frame only while moving. These frames do not publish typing edits, change revisions, or rebuild measured layout. Countdown deadlines and movement share the shell's earliest-redraw scheduling.

## Typing invariants

- Generate exactly 500 space-separated tokens in either mode. Plain words are sampled with replacement from the bundled list; duplicates are allowed. Numbers replaces one randomly positioned word in each group of ten with an integer from 0 to 9999. Punctuation appends one of `, . ! ? ; :` to one randomly positioned token in each group of four. Both options can apply to the same token. Join tokens with exactly 499 ASCII spaces and no trailing space. Each token, including its trailing punctuation, counts as one word for progress.
- Represent logical positions as character indices, not UTF-8 byte offsets. Store entered characters; derive progress from their count rather than maintaining redundant counters.
- Every printable input scalar consumes one target position, including incorrect input. Spaces are ordinary input positions. A wrong character at an expected space and a space at an expected letter both advance and are incorrect.
- Continue to display the expected glyph after mistakes; color it red. Give incorrect expected spaces a visible red cell treatment or underline.
- Backspace removes exactly one entered character, including across word boundaries. At position zero it is a no-op.
- Entering the last target character completes the test, even when incorrect. Completion is terminal: further typing and Backspace do nothing. Preserve the target for Retry; the application freezes edits and transitions to Score on completion. The pure engine retains its terminal full-buffer semantics. A fresh unmodified Tab keypress explicitly replaces the session and resets progress. Ignore held-Tab repeats and modified Tab shortcuts.
- Process native committed keyboard text exactly once. Handle Backspace separately, ignore control characters and shortcut combinations, and use native repeat events without artificial delays. Do not synthesize text from physical key codes.

## Layout and viewport invariants

- Startup and minimum window size are 700 × 400 logical pixels. The panel is centered on both axes, 640 × 110 at the minimum window size, and grows to at most 700 × 110 logical pixels. Its height includes four 20px text rows and 15px top/bottom inner padding. Keep at least 30px horizontal and 125px vertical exterior clearance; default to a 16px font and 20px line height. `settings.json` may set `font-size` from 8 to 48 logical pixels; line height scales by 1.25. The dimensions above apply at the default font size. Panel height scales to four line heights plus 30px padding (rounded upward). Startup/minimum window height grows above 400px when needed to preserve four rows and 125px vertical exterior clearance.
- Always center the visible text block horizontally and vertically within the available area, including after scrolling, resizing, restart, and completion. Keep lines left-aligned within the centered block; glyphs, underlines, and caret share its origin. Rebuild cached layout when target text changes on restart.
- Use 20px horizontal and 15px vertical inner padding and word wrapping based on measured monospaced advance. Reserve space for the caret and separator/error cells. Retain a safe character-wrap fallback for unexpectedly long words.
- Each visual line owns the separator after its final word. Stay on that line until the separator position is consumed; then put the next active line at the viewport top. Incorrect input at the separator also consumes it under the engine rules.
- Show at most four whole text rows (including the preview row): use the floor of available text height divided by line height. Distribute spare vertical space around the centered block and clip overflow. Never expose a partially clipped bottom row.
- After every completed visual line, its successor becomes the top row and completed text exits above. This is not conventional scroll-only-when-the-caret-reaches-the-bottom behavior.
- If more text exists below the viewport, render its bottom visible preview row at 50% opacity. Multiply the base glyph/error alpha by 0.5; do not halve row height. A line becomes opaque as it moves upward. Do not fade the active line, and do not fade the final visible line when there is no additional text below the viewport.
- Wheel scrolling temporarily permits inspecting previous or later text in whole-line increments. The next accepted input, successful Backspace, or resize restores the active line to the top. No-op input leaves manual inspection unchanged.
- On resize, recompute wrapping from the same logical position. Backspace across a wrap restores the preceding active line to the top.
- At completion, retain the final character's line as the terminal anchor and place the caret after its last glyph. Do not scroll to an empty sentinel line. Manual scrolling remains available after completion.
- Keep logical position in the engine, target caret coordinates in the layout result, and rendered coordinates in UI state. Movement is always enabled at 100 ms with power `inOut(1.25)` easing, centralized in `src/ui/caret.rs`. Retarget from the current interpolated position without queuing obsolete movements or restarting on unchanged targets.
- Animate panel-relative coordinates derived from the shared measured layout and centered origin, including forward/backward line crossings; text scrolling remains immediate. Clip the caret to the padded panel separately from the text block so recentering cannot hide its return movement.
- Snap on startup, session replacement, resize/reflow, font-metric changes, and focus loss. Wheel inspection cancels motion and positions the caret against inspected text; hide it if its active line is outside the viewport. The first accepted edit after inspection snaps back to automatic following; later edits animate. Completion lets the final movement settle. Keep the solid caret style; do not add blink behavior or a motion setting without authorization.

## Visual reference

Primary reference: [Typerr Figma](https://www.figma.com/design/80x3SZRgeqDp8xOz7FzGdU/Typerr?node-id=0-1), current frame `54:22`, text `54:28`, counter `71:5`. `design_references/Typing Page.png` is the current rendered reference; the older Outline file marks layout areas only.

References whose frame or image name ends in “Outline” use colorful rectangles only to mark general layout areas. Preserve their placement and spacing, but do not render those rectangle fills or borders as decoration. The typing panel is transparent and borderless over background `#282A36`, using Google Sans Code Regular 16px and reference foreground `#F8F8F2`. Use the bundled Jomhuria Regular at 48px for branding, `#44475A` control groups, and `#0088FF` for the counter/timer; foreground for selected modes/enabled features and Comment for inactive controls. The counter counts target words whose final character has been entered, regardless of correctness, and decreases when Backspace removes that character. In Words mode the display reaches the selected target (for example, 25/25) and immediately begins the score transition; Backspace can reduce progress only before completion. Tab resets it. In Time mode the same location displays the remaining seconds, from the selected duration to `0s`. Footer connections open the OS browser at https://github.com/Arik-Ahnaf/Typerr (Github), https://x.com/ArikAhnaf1 (Twitter), and https://discord.gg/FYehF2KCw4 (Discord); version/theme labels are informational. This user clarification supersedes the initial interpretation of the cyan panel rectangle.

Interaction defaults, not separately specified by the Figma placeholder: pending `#6272A4`, correct `#F8F8F2`, incorrect `#FF5555`, and thin caret `#8BE9FD`. Base colors are opaque; preview opacity is an additional multiplier. Use `assets/fonts/Google_Sans_Code/static/GoogleSansCode-Regular.ttf` and preserve its OFL notice. Bundle `assets/fonts/Jomhuria/Jomhuria-Regular.ttf` with its OFL notice and embed the supplied SVGs. `src/ui/chrome.rs` owns the native header/footer composition.

## Workflow and quality

Read this file, `PLAN.md`, `BUILD.md`, and `PACKAGING.md` before implementation. Follow the current alpha's plan; record changed decisions in all affected documentation. Do not expand scope to future versions without explicit authorization.

Use idiomatic Rust, explicit ownership, standard formatting, and focused tests. Avoid speculative abstractions and needless dependencies. Keep engine and layout tests deterministic. Treat renderer metrics as the shared source of truth rather than maintaining separate caret arithmetic.

Run formatting checks, strict Clippy, tests, and builds using `BUILD.md`. Fix issues and update `PLAN.md` with actual outcomes. Clearly distinguish a compile check, automated behavior test, GUI launch, screenshot inspection, and native-platform verification. Do not call Windows supported because Linux compiled, or call visual behavior verified because unit tests passed.

Before delivery, review all acceptance criteria, architecture boundaries, test results, and documentation. Record remaining limitations and recommended next alpha work. Do not push or publish releases unless separately requested.

## Dracula palette source

`themes/dracula.json` is the color source of truth, copied from `design_references/Dracula Color Pallet.png`. Preserve the image's Current Line value `#6272A4`. Its separate blue accent retains the counter/timer color. The JSON maps UI roles to palette names; `build.rs` validates the hex values and mappings using build-only `serde_json` and generates native color constants imported by `src/ui/theme.rs`. JSON edits require rebuilding; there is no runtime filesystem dependency or theme selector. SVG tinting uses the same constants. The footer displays the theme's JSON name.

## Startup settings

`src/settings.rs` reads `settings.json` once at startup, preferring the working directory and then the executable directory. Missing or invalid configuration uses 16px; malformed/unreadable files emit a diagnostic. The typing option is numeric `font-size` (8–48 logical pixels); `name` stores the first-launch profile name. Restart to apply edits. Renderer measurement, wrapping, glyphs, caret, clipping, and scroll increments share the configured metrics; header/footer and counter/timer sizes remain unchanged. Runtime `serde_json` reuses the existing locked build dependency. Tests use defaults independently of local settings.

## Score page and crossfade

Figma Score frame `59:3` and graph template `94:66`, plus the two corresponding PNG references, define the score UI. Keep the 700 × 400 composition centered in larger windows; common branding stays at its existing window position throughout the fade. Score typography is independent of typing `font-size`. Hide typing controls/footer on Score. Replace the green placeholder with a transparent native Canvas plot, reserving the right edge for the Errors axis. Use Google Sans Code and existing centralized theme colors; use the supplied `next.svg` (12 × 20) and `restart.svg` (25 × 25) inside separate 36 × 40 Next/Retry buttons. The numbers toggle uses `hash.svg` (20 × 20).

`Score` in `src/statistics.rs` is the completed-session source of truth. Start monotonic timing on the first accepted printable scalar in either mode, keep idle/unfocused time, and freeze at the completion character or exact Time deadline (before the fade). WPM = retained correct scalars / 5 / elapsed minutes; Raw = all retained scalars / 5 / elapsed minutes, displayed as WPM rather than a percentage. Include spaces, punctuation, and numbers. Backspace removes the deleted scalar from these retained counts. Accuracy = correct printable attempts / all printable attempts * 100; deleting mistakes does not erase their accuracy/error history, and Backspace/control/shortcut/no-op events are not attempts. Consistency = clamp(100 * (1 - population standard deviation / mean), 0, 100) of attempted characters/second over one-second intervals, including idle intervals; weight the final partial interval by its duration. The first interval includes time zero and all intervals include their right endpoint. A single interval has 100% consistency; zero elapsed time has zero speeds/consistency and an empty graph, avoiding division by zero. Every session replacement clears all measurements.

The graph plots cumulative retained correct/raw WPM and mistake counts per interval. Normally sample each second plus the actual final partial interval; cap long Words tests at 300 graph samples by using wider intervals, while consistency still uses one-second intervals. Use actual elapsed X coordinates, adaptive speed/error axes, shape-preserving curves, and no fabricated PB guide. Completed-test history and personal-best queries are persisted through the storage module; historical graph samples remain out of scope. Score typography stays independent of typing font size; fit wider measured values within the existing composition. The scoring calculations require no new dependency; persistence uses bundled `rusqlite`.

App owns explicit Typing/Transition/Score state. The 300 ms crossfade uses monotonic elapsed time and smoothstep easing; multiply all outgoing/incoming primitive opacity, including typing preview alpha, while keeping the background/brand steady. Retain the outgoing typing widget through the fade and disable its edits/wheel interaction. PageHost forwards keyboard/focus synchronously and requests frames only during transitions or at countdown deadlines; the typing widget still handles caret redraws and local focus/resize events. Delayed redraws catch up. Stop processing a committed-text event as soon as its accepted character completes the test. Next, Retry, and Tab increment session generation even when the target is unchanged.

## Mode amount selectors

Figma Words `103:10` and Times `7:65` under Typing Page `54:22` define native 170 × 30 bars with 8px corners, control-background fill, and Google Sans Code 16px / 20px labels. At 700px width, Words begins at (325, 65) and Times at (295, 65); both follow the centered mode controls when resizing. Both reuse existing compiled theme/font assets.

Right-clicking a mode toggles its bar without changing the active course. Words offers 10/25/50/100; Time offers 15/30/60/120 seconds. Choosing a value activates that mode and starts a fresh session even if the value is unchanged. Each mode remembers its latest value for the current app run, including through Tab, generation toggles, mode changes, Next, and Retry. Left-clicking another mode starts it with its remembered amount; left-clicking the active mode retains the session. No settings writes or persistence across app launches.

Only one bar is open at a time. Selection, outside click, Escape, accepted typing/Backspace, session replacement, focus loss, or completion closes it. Background/footer clicks dismiss without activating underlying controls; header controls remain available. Opening/dismissing preserves input, timer, edit revision, and session generation; a running timer continues. Pointer actions remain disabled during the score transition. The 500-token generation buffer remains unchanged; selected amounts determine the measured test duration and completion boundary.

## Refreshed page chrome — 2026-10-29

Figma frames `54:22` and `59:3` now place a shared 70 × 30 account button 50px from the right and 30px from the top. Display the saved user name with bundled Google Sans Code SemiBoldItalic at 20px, an 8px corner radius, control background, and Comment text changing to foreground on hover/press. Clicking opens Dashboard (authorized 2026-11-04). Disable pointer actions during the fade while keeping the account visible with the shared brand.

The typing control groups form a centered 170 × 30 row at y=30 (x=265 at 700px); amount menus retain their 170 × 30 size at y=65. Score metrics begin at x=50, the speed-axis title at x=125, tick labels at x=150, and the plot at x=170; retain room for the Errors axis at the right. Lower statistics begin at x=180/264/410. Score also shows the centered 50 × 25 `Tab` hint at (252, 37), followed by `to start a new test`; it is presentation only and uses the existing fresh-Tab behavior. Preserve measured data, the four-row typing viewport, configured font sizing, and all existing completion/input policies.


## Completed score history — 2026-11-03

Read `STORAGE.md` before changing persistence. `scores.db` lives under the user's
Linux XDG data directory (HOME fallback) or Windows LocalAppData, never beside an
installed executable. `App::new` initializes the store; `App::default` stays free
of filesystem side effects. Keep `statistics.rs` pure. Save one frozen summary
at the guarded Typing-to-Transition boundary; use one background writer and drain
its queue on normal shutdown. Do not save abandoned tests, profile fields, target
text, or graph samples. Retry completion creates a distinct test.

Use the checked-in parameterized SQL, schema version 1, and transactional
initialization. Never overwrite corrupt/unsupported databases. Report storage
failures to stderr and the writable local log while preserving typing behavior.
Dashboard averages are per completed test, duration sums only completed tests,
and personal bests combine all options within each mode/amount; accuracy comes
from the winning WPM row. Keep tests isolated from actual user history. SQLite is
bundled through `rusqlite`; `tempfile` is also used for private updater staging. Preserve dependency notices
and Windows cross-build checks when changing storage dependencies.


## Dashboard — 2026-11-04

Figma `86:50` and `design_references/Dashboard Page.png` define the
700 × 400 reference layout. Dashboard fills the available window, up to the
entire screen: scale card/chart geometry and positions with each window axis,
and scale body text and profile artwork uniformly by the smaller ratio.
Keep the avatar circular. Header branding stays at its original fixed size,
50px from the left and 30px from the top, with a fixed 124 × 40 Home hit target.
The greeting stays 20px at top=37/right=70. Neither header element scales.
Do not cap Dashboard at 700 × 400 or add outer centering margins. The pink 610 × 100 area at (45, 287) is only chart placement; the centered 500 × 2 foreground rule at (100, 275) is visible chart chrome. The four cards begin at x=45/355. Use the renderer-measured top heading as the fixed left anchor for both headings and values in each lifetime-summary column; keep best-result columns centered. Render a transparent native Canvas histogram from `All time wpm bar chart.png`.
Reuse the shared brand and compiled palette. Bundle the supplied default profile JPEG and existing Google Sans Code Italic/Medium/SemiBold/Light font faces with
OFL coverage. The profile name and greeting use the saved name; title and joined date remain hardcoded.
Dashboard typography is independent of typing font settings.

Account clicks from Typing or Score open Dashboard; ignore them during fades.
Entering closes amount menus, abandons an unfinished test without saving it, and
stops its countdown. The brand button or an unmodified fresh Tab returns to a
new typing session with remembered mode, amounts, and options. Dashboard ignores
printable text and Backspace. Do not silently resume an expired hidden timer.

Read the lifetime summary, eight mode/amount bests, and histogram in one read
transaction on the existing storage worker, ordered after queued saves. Never
block the UI on SQLite. Poll only while a reply is pending; replace stale data on
each opening. Read failures are logged through the existing storage diagnostics.
Show N/A for absent metrics/categories, empty history, or failed reads, while
preserving real zero values. Best accuracy belongs to its winning WPM row.
The histogram counts all completed tests in twelve ranges, initially 0–9 through
110–119, widening in multiples of ten to include higher speeds. No history rows
or profile values are fabricated. No new dependency or schema migration.

## Startup updater — 2026-11-04

Read `UPDATING.md` before changing update behavior. Version 1.1.0 aligns Cargo,
visible version labels, package filenames, and future `v<version>` GitHub tags;
the historical v1.0.0 release contains 0.1.0 packages and must never be offered
as an upgrade. Networking is limited to a once-per-launch stable GitHub release
check and downloads explicitly accepted through the native dialog. Check errors
are quiet; defer offers until Typing has not started, Score, or Dashboard.

`src/update.rs` owns worker communication; `src/update/` owns bounded HTTPS,
release validation, extraction, and installation/recovery. No updater work in
engine, layout, statistics, or SQLite. `App::default` remains side-effect-free.
Keep synchronous PageHost input forwarding. Modal keys must never edit/restart
underlying sessions. Poll only while worker results or startup confirmation are
pending. Wait for GUI acknowledgment before entering non-cancellable installation.
Drain score writes before releasing the helper handoff.

Require the exact platform/version archive, SHA256SUMS, and any GitHub asset
digest; reject unsafe archive paths, links, duplicate entries, wrong architecture,
and mismatched embedded versions. Portable replacement uses a durable journal,
backups, process locks, and a copied helper. Preserve existing settings and user
history. Package-owned installations must use authenticated pacman, never direct
file replacement or weakened signature policy. Temporary downloads, lock files,
recovery journals, and updater diagnostics are authorized; no settings UI,
automatic publication, or application data migration is authorized.

## First-launch name — 2026-11-04

Figma Name Setter Modal `122:10` is a centered 300 × 150 native card. Require
1–16 Unicode scalar characters after trimming, excluding control characters.
No name means onboarding on launch; valid saved names skip it. Save or Enter
persists `name` before unlocking Typing. Escape, Tab, background clicks, and
text must not operate underlying pages. Tab returns focus to the field.
Defer update offers while onboarding; retain startup acknowledgment.

`settings::Profile` preserves unrelated JSON keys and atomically replaces the
settings file. User settings take priority, then working-directory settings,
then executable-adjacent settings. If the original location is unwritable,
save a merged copy to `$XDG_CONFIG_HOME/typerr/settings.json` (HOME/.config
fallback) or `%LOCALAPPDATA%/typerr/settings.json`. Both font settings and name
load from the same selected file. Show write/parse failures in the modal and
keep it open. Do not overwrite malformed configuration or store names in SQLite.

The backdrop uses a cached native software-rendered page with a three-pass
separable blur and a 25% black overlay. Render the backdrop at half resolution,
capped at 512px per axis, using an explicit renderer transform. Reuse the cached
image during live resizing and rebuild after 120ms of settled geometry; keep
modal layout and text sharp and immediate. Image rendering is enabled without image codecs.
Account and Dashboard labels use the saved name and accommodate 16 characters.
Packaging removes `name` from staged settings; never ship a builder's profile.

## Profile picture chooser — 2026-11-05

Figma `143:15` and `design_references/Pfp Chooser Modal.png` define the centered
native Edit Image card. Uniformly fit the entire card and controls inside the
window with 16px exterior margins, growing up to 400 × 460 (2× the reference).
At 700 × 400 the card is 320 × 368. Reserve space for scrollable errors when
present. Never resize the app window to fit the modal. Preview image bounds,
clipping, crop ring, and drag coordinates share the actual display scale.
Dashboard uses the bundled
`assets/icons/profile-placeholder.jpg` by default. Hover darkens the circular
avatar and shows the supplied pencil; click opens the OS PNG/JPG chooser.
After selection, drag to position, use the slider or wheel to zoom 1–4×, and
choose Save or Discard. Close/Escape discards; outside clicks do nothing.
Tab/Shift+Tab cycles zoom, Discard, Save; arrows adjust focused zoom and Shift
with arrows pans. No modal keys or pointer events navigate/edit underlying pages.
Defer update offers until the chooser/editor closes. Keep this UI native.

`src/profile_picture.rs` owns bounded decoding, EXIF orientation, crop geometry,
and atomic avatar persistence; `src/ui/picture_prompt.rs` owns native interaction
and rendering. The PNG/JPEG `image` codecs and `rfd` native dialog are concrete
new dependencies. Decode/save run on background threads; poll only while pending.
Keep the crop inside source bounds, preserve aspect ratio, and display a circular
mask. Save a 512 × 512 PNG to the user configuration folder as `avatar.png`
(Linux XDG_CONFIG_HOME/typerr or HOME/.config/typerr; Windows LOCALAPPDATA/typerr).
Keep the previous avatar until saving succeeds. Discard, picker cancellation,
and read/write failures preserve it; failures are visible and retryable. Do not
modify the original image, settings JSON, or SQLite. `App::default` performs no
filesystem access. Limit inputs to 20 MiB, 8192px per axis and 128 MiB decoder
allocation; reduce working copies to 2048px per axis. Never package user avatars.


## Native Settings UI — 2026-11-06

Figma Settings `91:26` and `design_references/Settings Page.png` define the
700 × 400 reference. Ctrl+, opens Settings from Typing, Score, or Dashboard;
ignore held repeats, extra modifiers, unfocused input, completion fades, and
blocking modals. Opening abandons an unfinished test without saving, closes
amount menus, and stops its countdown. Clicking the fixed brand returns to a
fresh test with remembered mode, amounts, and generation options. Ctrl+, while
already on Settings preserves the current view. Tab/Shift+Tab traverses settings
fields and must never restart the underlying typing session.

Keep branding fixed at (50, 30). Scale body geometry by each window axis and
fonts/control contents uniformly by the smaller ratio. At the reference size,
search is (30, 80, 640, 40), categories are (30, 130, 90, 160), and the options
viewport is (130, 130, 540, 250). Only the options panel scrolls. General, Caret,
Theme, Sound category clicks clear search and scroll their section heading to
the top; leave enough trailing space for Sound. Highlight the section at the
viewport top. Case-insensitive search matches category names or row labels,
keeps matching headings, and shows an empty-result message when appropriate.

`src/ui/settings_page.rs` owns temporary form state and native layout. Font size
starts at 16, accepts integer 8–48, and restores the last valid value on invalid
commit. Font family offers Google Sans Code; caret offers Line/Block/Underline;
theme offers Dracula; Sound offers Off/On. Edits remain drafts until Save. Load
saved values on startup, preserving valid fractional font sizes from existing
configuration until edited. Save does not change the running font/caret/theme/audio
or SQLite. The existing font-size startup setting takes effect after restart;
other saved parameters are currently UI-only. Keep `App::default` free of filesystem access. Reuse
compiled fonts/colors/branding and bundle the Figma dropdown arrow. The narrow
Settings combo-box adapter exposes Iced 0.14's existing input to focus operations;
keep typing's synchronous PageHost event path and modal isolation intact.
Settings is a safe page for existing updater offers.

The Save follow-up uses Figma `159:12`: a 100 × 33 background-colored button at
(560, 337), 8px corners, Google Sans Code Bold 20px. Keep it fixed within the scaled options
panel; reserve the bottom 43px for Save and status, leaving a 207px scrolling
viewport at reference size. Reuse the bundled Google Sans Code Bold font.
Save all parameters as `font-size` (number), `font-family`, `caret-style`, `theme`
(strings), and `sound` (boolean), preserving name and unrelated JSON keys.
Reuse the profile configuration path and atomic replacement/fallback behavior;
never overwrite malformed JSON, including an existing fallback. Show Saved only
after a successful write; show retryable errors without leaving the page or
losing drafts. No live preference application is authorized by this follow-up.

The Settings search stays at y=80 as the window grows. Its scale is
clamp(1 + (min(width/700, height/400) - 1) * 0.25, 1, 1.25), yielding 40–50px
height and 16–20px text. Keep its width aligned with the body panels. Place the
panels directly below it with a 10px gap scaled by that same search factor;
expand the options panel to retain its original scaled bottom margin. Category
scroll targets still use the body vertical scale, independently of viewport
height. Save has a 10px scaled inset from the panel's right edge.

## Gruvbox theme — 2026-11-07

The theme follow-up authorizes startup selection of Dracula or Gruvbox through
settings.json and the existing Settings dropdown/Save flow. Save remains deferred
until restart. Both JSON palettes share Dracula's keys and role mappings and are
validated/embedded by build.rs. Gruvbox uses Monkeytype's gruvbox_dark palette;
see themes/README.md for source and mappings. The selected immutable palette is
initialized before UI creation and supplies all page colors, SVG tints, graphs,
modal backdrops, and the footer name. Unknown/missing theme values use Dracula.
This supersedes earlier statements that theme is UI-only or Dracula-only.

The VSCode follow-up also authorizes Monkeytype's VSCode palette using the
same JSON schema, startup selection, and Settings Save/restart behavior.
The current selection is preserved when adding a theme. See themes/README.md
for the official source and role mapping.
