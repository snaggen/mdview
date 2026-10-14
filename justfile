name := 'mdview'
appid := 'io.github.snaggen.MdView'

# Installs into ~/.local by default; use `just prefix=/usr/local install` for a system install.
prefix := env_var('HOME') / '.local'
bin-dst := prefix / 'bin' / name
desktop-dst := prefix / 'share' / 'applications' / appid + '.desktop'
icon-dst := prefix / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / appid + '.svg'

default: build-release

build-release *args:
    cargo build --release {{args}}

# Formatting, lints and tests, as required before committing.
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

run *args:
    cargo run -- {{args}}

install: build-release
    install -Dm0755 target/release/{{name}} {{bin-dst}}
    install -Dm0644 res/{{appid}}.desktop {{desktop-dst}}
    install -Dm0644 res/icons/hicolor/scalable/apps/{{appid}}.svg {{icon-dst}}

uninstall:
    rm -f {{bin-dst}} {{desktop-dst}} {{icon-dst}}
