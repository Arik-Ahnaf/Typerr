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
Results are not saved between sessions.
Next starts fresh text; Retry repeats the same text. Tab restarts either mode.
The punctuation and number buttons independently change the generated text.

The supplied settings use 24px text. Edit `font-size` to a number from 8 to 48,
then restart. A `settings.json` in the launch working directory takes precedence
over the file beside the executable. Missing or invalid settings use 16px.
The application never writes settings.

This is an unsigned x64 MSVC release build cross-compiled on Arch Linux.
The MSVC C runtime is linked statically. Windows system libraries and working
graphics drivers are still required. Native Windows GUI behavior has not yet
been verified; cross-compilation and archive checks do not establish that.

Third-party licenses and notices are in `notices/`.
