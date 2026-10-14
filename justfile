name := 'mdview'
appid := 'io.github.snaggen.MdView'
version := `sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1`
rpmdir := justfile_directory() / 'target' / 'rpm'

# Installs into ~/.local by default; use `just prefix=/usr/local install` for a system install.
prefix := env_var('HOME') / '.local'
bin-dst := prefix / 'bin' / name
desktop-dst := prefix / 'share' / 'applications' / appid + '.desktop'
icon-dst := prefix / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / appid + '.svg'
metainfo-dst := prefix / 'share' / 'metainfo' / appid + '.metainfo.xml'

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
    install -Dm0644 res/{{appid}}.metainfo.xml {{metainfo-dst}}

uninstall:
    rm -f {{bin-dst}} {{desktop-dst}} {{icon-dst}} {{metainfo-dst}}

# Makes mdview the default application for Markdown files for the current user.
set-default:
    xdg-mime default {{appid}}.desktop text/markdown text/x-markdown
    xdg-mime query default text/markdown

# Source tarball of the committed HEAD (uncommitted changes are not included).
dist:
    mkdir -p {{rpmdir}}
    git archive --format=tar.gz --prefix={{name}}-{{version}}/ -o {{rpmdir}}/{{name}}-{{version}}.tar.gz HEAD

# Vendored crates (including git dependencies) and their licenses, for offline builds.
vendor:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p {{rpmdir}}
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
    cargo vendor --locked "$work/vendor" > "$work/vendor-config.toml"
    # cargo prints an absolute path to the vendor directory; the spec unpacks it
    # next to the sources.
    sed -i "s|$work/vendor|vendor|" "$work/vendor-config.toml"
    python3 -I packaging/dependency-licenses.py > "$work/LICENSE.dependencies"
    tar -C "$work" -cJf {{rpmdir}}/{{name}}-{{version}}-vendor.tar.xz vendor vendor-config.toml LICENSE.dependencies

# Source RPM, e.g. for COPR or mock.
srpm: dist vendor
    rpmbuild -bs --define "_sourcedir {{rpmdir}}" --define "_srcrpmdir {{rpmdir}}" packaging/{{name}}.spec

# Binary RPM, built locally with rpmbuild.
rpm: dist vendor
    rpmbuild -bb --define "_topdir {{rpmdir}}/build" --define "_sourcedir {{rpmdir}}" --define "_rpmdir {{rpmdir}}" packaging/{{name}}.spec

# Binary RPM built with the Rust toolchain on PATH (rustup/mise), skipping the build dependency check.
rpm-local: dist vendor
    rpmbuild -bb --nodeps --define "_topdir {{rpmdir}}/build" --define "_sourcedir {{rpmdir}}" --define "_rpmdir {{rpmdir}}" packaging/{{name}}.spec
