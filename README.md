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

To make mdview the default application for Markdown files: `just set-default`.

## Fedora RPM

The package builds offline from vendored crates, so it also works in mock and COPR.

```sh
just rpm      # binary RPM in target/rpm/
just srpm     # source RPM, e.g. for COPR
```

`dist` packs the committed `HEAD`, so commit before building. When dependencies change, update
the spec's License tag with `packaging/dependency-licenses.py --spec`.

## License

EUPL-1.2. See [PLAN.md](PLAN.md) for the design.
