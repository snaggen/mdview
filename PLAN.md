# mdview — plan

A simple Markdown viewer for the COSMIC desktop. Opens a `.md` file and renders it
natively, following the system theme. No editing, no tabs, no plugins.

## Scope

In:
- Render a Markdown file (CommonMark + GFM: tables, task lists, strikethrough).
- Colours, fonts, corner radii and light/dark follow the COSMIC theme.
- Syntax-highlighted code blocks with defaults for both light and dark.
- Toggle between rendered view and Markdown source.
- Settings panel: content area translucency (BlurStrength, default VeryLow, or opaque).
- Open file from CLI argument, file dialog, or `.desktop` MIME association.
- Clickable links: `http(s)` opens in the browser, relative `.md` links open in the app.
- Live reload: the view updates automatically when the file changes on disk.

Out: editing, tabs, export, print, TOC sidebar, search, math, mermaid.

Deferred: text selection in the rendered view. The markdown widget renders text with `rich_text`,
which has no selection support in libcosmic, and we are not building our own renderer. Text can be
selected and copied in the source view. Revisit if libcosmic gains selectable `rich_text`.

## Library research

| Option | Verdict |
|---|---|
| **libcosmic `markdown` + `highlighter` features** (iced's `markdown` widget, pulldown-cmark) | **Chosen.** Native widgets, styled by the COSMIC theme automatically. In current libcosmic (checked rev `5a8bd94`, 2026-10-02) the widget supports headings, paragraphs, code blocks, lists, task lists, quotes, rules, tables and images, and has a `Viewer` trait to override how each item is rendered. |
| comrak / markdown-rs → HTML in a webview | Full GFM fidelity, but a browser engine in a "simple" viewer, and theming has to be re-implemented in CSS from COSMIC tokens. Rejected. |
| Own pulldown-cmark → widget tree | What older libcosmic forced (its markdown widget lacked tables/quotes; `galaxy-md` does this). No longer needed. |

Prior art: `galaxy-md` (a COSMIC markdown viewer) uses an old libcosmic, a hardcoded Tokyo Night
theme and its own parsing. Useful as a reference for file watching and link handling, not as a base.

### Syntax highlighting

The markdown widget's built-in highlighter hard-codes `Base16Ocean` (a dark theme), which is
unreadable on a light desktop. Instead:

- Override `Viewer::code_block` (it receives `language` and the raw `code`) and highlight with
  **syntect** ourselves.
- Default themes: **`COSMIC Dark` / `COSMIC Light`** from `pop-os/cosmic-syntax-theme`, the same
  defaults COSMIC Text Editor uses. Chosen by `theme.cosmic().is_dark`, so they switch with the desktop.
- Clear the syntect theme background (as cosmic-edit does) so the code block uses the COSMIC
  container colour.
- `two-face` for the extended syntax set (TOML, Dockerfile, etc.), as cosmic-edit does.
- Cache highlighted blocks per (block index, dark/light); re-highlight only on theme or file change.

### Window translucency (BlurStrength)

How COSMIC does it (verified in source):
- `cosmic_theme::Theme` has `frosted: BlurStrength` (14 levels, `ExtremelyLow` … `ExtremelyHigh2`)
  and `frosted_windows: bool`. `ThemeBuilder::build()` turns the strength into the alpha of the
  translucent containers via `alpha_map.blurred_alpha(frosted)`.
- The blur itself is requested per window over `ext-background-effect-v1`, which only carries a
  region. The blur **radius** is global: cosmic-comp reads it from the system theme.

So a per-app BlurStrength controls how translucent the window is (how much blurred desktop shows
through), not the blur radius. This is the same thing the COSMIC setting changes for apps.

The setting applies to the **content area** (the document surface). Header bar and context drawer
keep following the system theme.

Implementation:
- `core.set_auto_blur(Auto::Window | Auto::Popup)` in `init`, so the compositor blurs behind the window.
- Setting `content_translucency`: `Strength(BlurStrength)` | `Opaque`. Default
  **`Strength(VeryLow)`** (alpha ≈ 0.85 with the default alpha map).
- The content container gets a custom style: the theme's background colour with alpha
  `theme.cosmic().alpha_map.blurred_alpha(strength)` (the user's own alpha map, not hard-coded
  values), or alpha 1.0 for `Opaque`. Only a style closure — no theme rebuild, so accent and
  light/dark changes keep working untouched.
- To verify in the prototype: whether the window is blurred at all when the system has
  `frosted_windows` off (libcosmic gates blur on it). If not, either call `window::enable_blur`
  ourselves when a strength is chosen, or document that system frosted windows must be on and
  show it as a hint in the settings panel.

### Source view

Read-only `text_editor` showing the raw file, highlighted with the same syntect setup
(`markdown` syntax) through a small `iced::advanced::text::Highlighter` implementation. Text is
selectable and copyable. Edit actions are dropped in `update`. Monospace font from the theme.

### Live reload

- `notify-debouncer-full` (inotify, from the notify project) watching the file's **parent directory**, filtered on the file name. Editors
  often save by writing a temp file and renaming it over the original, which a watch on the file
  itself loses after the first save.
- Exposed as an iced `Subscription` keyed on the path, so opening another file swaps the watcher.
- Debounce (~150 ms) to coalesce write bursts, then re-read off the UI thread
  (`Task::perform`); applies to both the rendered and the source view. Reads carry a request id
  so a late result never overwrites a newer one.
- The scroll position is kept across reloads: the scrollable's widget state survives because the
  widget tree keeps its shape, so editing in another window does not jump back to the top.
- File removed: keep showing the last content with a notice in the header; resume if the file
  reappears (rename-save). Read errors mid-save (truncated or partial file) are retried on the
  next event instead of replacing the view with an error.
- Local images are re-read on every reload of the document.

## UI (COSMIC patterns)

- Single primary view, no nav bar. Header bar: file name as title; end: toggle
  Rendered/Source (icon button, `text-x-generic-symbolic` / `view-reveal-symbolic`), open file,
  settings.
- Content: scrollable, centered column with a max readable width (theme spacing tokens, no raw
  numbers). The rendered view uses `markdown::Settings` derived from the theme's font sizes.
- Settings: context drawer with one settings section, "Content translucency", as a dropdown:
  ExtremelyLow … ExtremelyHigh2 (labelled readably, default VeryLow) · Opaque.
- Empty state (no file): centered icon + "Open a Markdown file" + suggested button.
- Error state (unreadable file): message in the content area, not a panic.
- Keys: `Ctrl+O` open, `Ctrl+U` toggle source, `Ctrl+,` settings, `Ctrl+R` reload, `Ctrl+Q` quit.

## Architecture

```
src/
  main.rs        startup, CLI arg, cosmic::app::run
  app.rs         Application impl: Message, update, view, header, context drawer
  document.rs    load file, parse into markdown::Content, resolve relative paths
  viewer.rs      markdown::Viewer impl: code blocks, images (local files), page-wide tables
  link.rs        resolve links and image references relative to the document
  highlight.rs   syntect setup, COSMIC themes, code-block and source highlighting
  config.rs      cosmic-config settings (content translucency), versioned
  style.rs       content container style from translucency + theme alpha map
  watch.rs       notify subscription for live reload
i18n/            fluent strings (en, sv)
res/             .desktop, metainfo, icon, MIME association
```

Dependencies: `libcosmic` (git, pinned rev; features `markdown`, `highlighter`, `wayland`,
`tokio`, `about`), `syntect`, `two-face`, `cosmic-syntax-theme` (git), `notify-debouncer-full`, `open`,
`i18n-embed`/`rust-embed` (libcosmic template standard).

Pure logic (path resolution, link classification, translucency → alpha, highlighting into
spans) lives in functions without UI types so it can be unit tested.

## Phases

1. **Skeleton** — from the `cosmic-app-template` layout: app shell, header bar, open file from
   CLI/dialog, render with the default `markdown` view. Empty and error states. `justfile`,
   `mise.toml` (Rust), `.desktop` + MIME.
2. **Live reload** — watcher subscription, debounce, scroll position kept, removed/rename handling.
3. **Viewer** — `Viewer` impl: links (browser / open relative `.md`), local images relative to the
   file, readable max width.
4. **Highlighting** — syntect + COSMIC Dark/Light in code blocks, switch on theme change.
5. **Source toggle** — read-only highlighted `text_editor`, `Ctrl+U`.
6. **Settings + translucency** — context drawer, cosmic-config, content container style; verify
   the open points above on a real COSMIC session.
7. **Polish** — keyboard shortcuts, about page, Swedish translation, `cosmic-ui` review pass.

Every phase ends with `cargo fmt`, `cargo clippy` clean and tests passing.

## Open questions

- Remote images (`https://` in `![]()`): skip (show alt text) to keep it offline and simple? (Plan
  assumes skip.)
- Packaging: just `just install`, or also Flatpak/RPM?
