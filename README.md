<p align="center">
  <img src="res/icons/hicolor/scalable/apps/io.github.snaggen.MdView.svg" width="128" alt="mdview icon">
</p>

<h1 align="center">mdview</h1>

<p align="center">A simple, native Markdown viewer for the <a href="https://github.com/pop-os/cosmic-epoch">COSMIC</a> desktop.</p>

mdview opens a Markdown file and renders it, nothing more. It is built with
[libcosmic](https://github.com/pop-os/libcosmic), so colours, fonts, accent and light/dark mode
come from your desktop theme.

```sh
mdview README.md
```

## Features

- **Native rendering** of CommonMark and GitHub extensions: tables, task lists, strikethrough,
  quotes, images and syntax-highlighted code.
- **Follows the system theme**, including switching between light and dark while open.
- **Live reload**: the view updates when the file is saved, keeping your scroll position. Works
  with editors that save by renaming a temporary file.
- **Source view** of the raw Markdown, with selection, copy and middle-click paste (PRIMARY).
- **Code colour themes**: COSMIC (default), GitHub, Gruvbox Material and Everforest, each in a
  light and a dark variant that follows the desktop.
- **Adjustable background opacity**, using COSMIC's blur levels so the blurred desktop can show
  through behind the document.
- **Links**: Markdown files open in mdview, everything else in your default application.
- Tables fit the page width and wrap their cells instead of scrolling sideways.

Text selection in the rendered view is not supported yet (libcosmic's rich text widget has none);
use the source view to copy text.

## Usage

```sh
mdview path/to/file.md
```

Without a file, mdview shows an **Open** button. Settings (opacity and code colours) open from the
gear button and are remembered between runs.

| Keys    | Action                          |
|---------|---------------------------------|
| Ctrl+O  | Open a file                     |
| Ctrl+R  | Reload                          |
| Ctrl+U  | Toggle rendered / source view   |
| Ctrl+,  | Settings                        |
| Esc     | Close settings                  |
| Ctrl+Q  | Quit                            |

To make mdview the default application for Markdown files, choose it in COSMIC Settings →
Default applications, or run `just set-default`.

## Installation

### Fedora RPM

```sh
just rpm-local                                     # build with your rustup/mise toolchain
sudo dnf install target/rpm/x86_64/mdview-*.rpm
```

`just rpm` builds with Fedora's own `cargo` and `rust` packages instead, and `just srpm` makes a
source RPM for COPR or mock. The package builds offline from vendored crates.

### From source

Requirements: Rust (pinned in `mise.toml`, install with `mise install`), `just`, a C compiler and
the `libxkbcommon` development files (`sudo dnf install gcc libxkbcommon-devel` on Fedora).

```sh
just install                    # into ~/.local
just prefix=/usr/local install  # system-wide
just uninstall
```

## Development

```sh
just run examples/sample.md   # run a debug build on a document using every element
just check                    # cargo fmt, clippy -D warnings and tests
```

[PLAN.md](PLAN.md) describes the design and the decisions behind it.

### Packaging notes

- `just dist` packs the committed `HEAD`, so commit before building packages.
- When dependencies change, update the License tag in `packaging/mdview.spec` with
  `packaging/dependency-licenses.py --spec`.
- The extra code colour themes are converted from their authors' VS Code themes with
  `tools/vscode-to-tmtheme.py`; see [res/syntax-themes](res/syntax-themes/README.md).

## Credits

- [libcosmic](https://github.com/pop-os/libcosmic) and [iced](https://github.com/iced-rs/iced) for
  the toolkit and markdown widget.
- [syntect](https://github.com/trishume/syntect) and [two-face](https://github.com/CosmicHorrorDev/two-face)
  for syntax highlighting, and [cosmic-syntax-theme](https://github.com/pop-os/cosmic-syntax-theme)
  for the COSMIC colour themes.
- Code colour themes: [GitHub](https://github.com/primer/github-vscode-theme),
  [Gruvbox Material](https://github.com/sainnhe/gruvbox-material) and
  [Everforest](https://github.com/sainnhe/everforest) (MIT).

## Use of AI

mdview is developed with the help of AI: much of the code and documentation was written with
[Claude Code](https://claude.com/claude-code), under human direction and review. Commits
with AI-written content carry a `Co-Authored-By: Claude` trailer.

## License

mdview is licensed under the [European Union Public Licence 1.2](LICENSE).
