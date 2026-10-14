# mdview

A simple Markdown viewer for the [COSMIC](https://github.com/pop-os/cosmic-epoch) desktop.
Colours, fonts and light/dark mode follow the system theme.

```sh
mdview path/to/file.md
```

The view reloads when the file changes on disk. Code blocks are highlighted with the
COSMIC syntax themes, and the settings let the document background be translucent.

| Keys | Action |
|---|---|
| Ctrl+O | Open a file |
| Ctrl+R | Reload |
| Ctrl+U | Toggle Markdown source |
| Ctrl+, | Settings |
| Ctrl+Q | Quit |

## Build and install

Toolchain versions are pinned in `mise.toml` (`mise install`).

```sh
just install          # installs into ~/.local
just prefix=/usr/local install
just check            # fmt, clippy, tests
```

See [PLAN.md](PLAN.md) for the design.
