# Typerr for Windows x64

Extract the entire ZIP, then open `typerr.exe`. Keep `settings.json` and the
`notices` directory with the application. Fonts, icons, words, and theme
colors are embedded; typing does not require network access.

Type to begin. Backspace removes a character; Tab starts a fresh session.
Right-click Words to choose 10, 25, 50, or 100 words; right-click Time to choose
15, 30, 60, or 120 seconds. Left-click a mode to use its last selected amount.
Defaults are 50 words and 30 seconds; selections last until the app closes.
Reaching the target opens Score with measured WPM, accuracy, raw WPM, consistency,
elapsed time, and a performance graph. Corrected mistakes still affect accuracy.
Completed results are saved automatically in `%LOCALAPPDATA%\Typerr\scores.db`.
Abandoned tests are not saved. SQLite is bundled; no separate install is needed.
Save failures are recorded in `scores-errors.log` beside the database when writable.
Close Typerr before copying the database for a backup. Click your name to view Dashboard statistics and all-time WPM ranges. Click the Typerr logo or press Tab to start a new test. The title and joined date remain placeholders.
Next starts fresh text; Retry repeats the same text. Tab restarts either mode.
The punctuation and number buttons independently change the generated text.

The supplied settings use 24px text. Edit `font-size` to a number from 8 to 48,
then restart. A `settings.json` in the launch working directory takes precedence
over the file beside the executable. Missing or invalid settings use 16px.
The application saves your name in settings; other values are preserved.

This is an unsigned x64 MSVC release build cross-compiled on Arch Linux.
The MSVC C runtime is linked statically. Windows system libraries and working
graphics drivers are still required. Native Windows GUI behavior has not yet
been verified; cross-compilation and archive checks do not establish that.

Third-party licenses and notices are in `notices/`.

## Updates

At startup, Typerr checks the public GitHub repository for a newer stable release.
The app remains usable offline. An offer waits until your current test finishes
or you return to a fresh session/Dashboard. Choose **Upgrade and restart** to
install, or **Later** to keep this version until the next launch. Tab selects a
dialog button, Enter activates it, and Escape dismisses/cancels when available.
Downloads can be cancelled before installation starts. Close other Typerr windows
before upgrading. Settings and score history are preserved.

Portable updates need write access to the application folder. Downloads are
verified before files change, and backups remain until the new app starts.
If an update is interrupted, start Typerr again to recover. Do not remove the
`.typerr-update` folder while an update or recovery is running. Updater errors
are logged to `updates-errors.log` beside the score database. The first version
with this feature (2.0.0) must be installed manually.

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

Windows profile pictures are saved to `%LOCALAPPDATA%/typerr/avatar.png`.
The image chooser uses the Windows native file dialog.
