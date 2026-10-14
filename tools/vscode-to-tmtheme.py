#!/usr/bin/env python3
"""Convert a VS Code color theme (JSON) into a TextMate .tmTheme for syntect.

VS Code's `tokenColors` are TextMate scope rules, so they carry over directly;
the editor colours become the theme's global settings. Semantic token colours
have no TextMate equivalent and are dropped.

Usage: vscode-to-tmtheme.py THEME.json NAME > NAME.tmTheme
"""

import json
import plistlib
import sys

# Font styles syntect understands; others (e.g. "strikethrough") make it reject the theme.
FONT_STYLES = {"bold", "italic", "underline"}

# Global settings taken from the VS Code editor colours.
GLOBAL_COLORS = {
    "foreground": "editor.foreground",
    "background": "editor.background",
    "caret": "editorCursor.foreground",
    "selection": "editor.selectionBackground",
    "lineHighlight": "editor.lineHighlightBackground",
}


def rule_settings(settings):
    out = {}
    for key in ("foreground", "background"):
        if settings.get(key):
            out[key] = settings[key]
    if "fontStyle" in settings:
        styles = [s for s in settings["fontStyle"].split() if s in FONT_STYLES]
        out["fontStyle"] = " ".join(styles)
    return out


def convert(theme, name):
    colors = theme.get("colors", {})
    global_settings = {key: colors[src] for key, src in GLOBAL_COLORS.items() if colors.get(src)}
    rules = [{"settings": global_settings}]
    for rule in theme.get("tokenColors", []):
        settings = rule_settings(rule.get("settings", {}))
        scope = rule.get("scope")
        if not scope:
            # A rule without scope sets defaults.
            global_settings.update(settings)
            continue
        if isinstance(scope, list):
            scope = ", ".join(scope)
        if settings:
            rules.append({"name": rule.get("name", scope), "scope": scope, "settings": settings})
    return {"name": name, "settings": rules}


def main():
    path, name = sys.argv[1], sys.argv[2]
    with open(path, encoding="utf-8") as file:
        theme = json.load(file)
    sys.stdout.buffer.write(plistlib.dumps(convert(theme, name)))


if __name__ == "__main__":
    main()
