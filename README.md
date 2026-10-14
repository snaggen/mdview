# mdview

A simple Markdown viewer for the [COSMIC](https://github.com/pop-os/cosmic-epoch) desktop.
Colours, fonts and light/dark mode follow the system theme.

```sh
mdview path/to/file.md
```

## Build and install

Toolchain versions are pinned in `mise.toml` (`mise install`).

```sh
just install          # installs into ~/.local
just prefix=/usr/local install
just check            # fmt, clippy, tests
```

See [PLAN.md](PLAN.md) for the design.
