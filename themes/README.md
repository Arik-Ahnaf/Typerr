# Bundled themes

`dracula.json`, `gruvbox.json`, and `vscode.json` share the same name, colors, accents, and
roles schema. Build-time validation embeds all palettes; no theme files are
needed at runtime. Edit `settings.json` to select `"theme": "Gruvbox"` or
`"theme": "Dracula"` or `"theme": "VSCode"`, or use Settings and Save, then restart. Names are
case-insensitive; missing, unknown, or non-string selections use Dracula.

Gruvbox matches Monkeytype's
[official gruvbox_dark entry](https://github.com/monkeytypegame/monkeytype/blob/master/frontend/src/ts/constants/themes.ts):
background = bg (#282828), current_line/selection = subAlt (#212121),
foreground = text (#EBDBB2), comment = sub (#665C54), red = error (#FB4934),
cyan = caret (#FABD2F), blue/orange = main (#D79921). The existing role names
are preserved even where they now refer to a yellow caret or gold accent.
Unused yellow/green/purple/pink keys retain the original Gruvbox colors.
Typerr has no separate extra-error or colorful-error roles.
Dracula retains its existing palette and blue accent unchanged.

VSCode matches Monkeytype's
[official vscode theme entry](https://github.com/monkeytypegame/monkeytype/blob/master/frontend/src/ts/constants/themes.ts):
background = bg (#1E1E1E), current_line/selection = subAlt (#191919),
foreground = text (#D4D4D4), comment = sub (#4D4D4D), red = error (#F44747),
cyan = caret (#569CD6), blue/orange = main (#007ACC). To retain the existing
schema, unused yellow/green keys alias text and purple/pink alias main;
Monkeytype does not define separate colors for these keys. Orange maps to
main so the existing footer icons also use the theme's blue accent.
