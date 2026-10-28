# Typerr for Arch Linux

For the portable archive, extract it and run `./typerr` inside its directory.
For the pacman package, install with `sudo pacman -U <package>.pkg.tar.zst`,
then run `typerr` or open Typerr from the application menu.

Type to begin. Backspace removes a character; Tab starts a new session.
Right-click Words to choose 10, 25, 50, or 100 words; right-click Time to choose
15, 30, 60, or 120 seconds. Left-click a mode to use its last selected amount.
Defaults are 50 words and 30 seconds; selections last until the app closes.
Reaching the target opens Score with measured WPM, accuracy, raw WPM, consistency,
elapsed time, and a performance graph. Corrected mistakes still affect accuracy.
Results are not saved between sessions.
Next starts fresh text; Retry repeats the same text. Tab restarts either mode.
The punctuation and number buttons independently change the generated text.

The supplied `settings.json` uses 24px text. Set `font-size` to a number from
8 to 48 and restart to apply it. A settings file in the launch working
directory takes precedence over the file beside the executable. Missing or
invalid settings use 16px. The application never writes settings.

Portable settings are beside `typerr`. The installed package uses
`/usr/lib/typerr/settings.json` (editing it requires administrator access);
pacman preserves modified settings during upgrades. You can instead launch
from a directory containing your own `settings.json`.

Requires Arch Linux x86_64 with an X11/XWayland or Wayland graphical session,
working graphics drivers, and the display/renderer libraries declared by the
pacman package. The archive does not bundle system libraries. `xdg-utils`
and a configured browser enable the footer links. Fonts, icons, words and
theme colors are embedded; no source checkout or network is needed for typing.

Third-party notices are in `notices/` in the portable archive, or
`/usr/share/licenses/typerr/` after installation.
