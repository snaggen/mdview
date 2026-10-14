Name:           mdview
Version:        0.1.0
Release:        1%{?dist}
Summary:        Simple Markdown viewer for the COSMIC desktop

# mdview is EUPL-1.2. The binary statically links its Rust dependencies, whose
# licenses are listed per crate in LICENSE.dependencies; regenerate this tag with
# packaging/dependency-licenses.py --spec when dependencies change.
License:        EUPL-1.2 AND ((MIT OR Apache-2.0) AND Unicode-3.0) AND (0BSD OR Apache-2.0 OR MIT) AND Apache-2.0 AND (Apache-2.0 AND MIT) AND (Apache-2.0 OR BSD-2-Clause OR MIT) AND (Apache-2.0 OR BSD-3-Clause) AND (Apache-2.0 OR GPL-2.0-only) AND (Apache-2.0 OR MIT) AND (Apache-2.0 OR MIT OR Zlib) AND (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT) AND BSD-2-Clause AND BSD-3-Clause AND CC0-1.0 AND ISC AND (LGPL-3.0-or-later OR MIT) AND MIT AND (MIT OR Unlicense) AND MPL-2.0 AND Unicode-3.0 AND Unlicense AND Zlib

# Created by `just dist` and `just vendor`.
Source0:        %{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.xz

ExclusiveArch:  %{rust_arches}

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  libxkbcommon-devel
BuildRequires:  desktop-file-utils
BuildRequires:  appstream

Requires:       hicolor-icon-theme

%description
A simple Markdown viewer for the COSMIC desktop. Colours, fonts and light or
dark mode follow the system theme. It renders tables, task lists, images and
syntax-highlighted code, reloads when the file changes on disk, and has a
read-only source view.

%prep
%autosetup -a1
# Build offline from the vendored crates, including the git dependencies.
mkdir -p .cargo
cp vendor-config.toml .cargo/config.toml

%build
export RUSTFLAGS='%{build_rustflags}'
cargo build --release --frozen --offline

%install
install -Dm0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}
desktop-file-install --dir=%{buildroot}%{_datadir}/applications \
    res/io.github.snaggen.MdView.desktop
install -Dm0644 res/icons/hicolor/scalable/apps/io.github.snaggen.MdView.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/io.github.snaggen.MdView.svg
install -Dm0644 res/io.github.snaggen.MdView.metainfo.xml \
    %{buildroot}%{_metainfodir}/io.github.snaggen.MdView.metainfo.xml

%check
appstreamcli validate --no-net %{buildroot}%{_metainfodir}/io.github.snaggen.MdView.metainfo.xml
export RUSTFLAGS='%{build_rustflags}'
cargo test --release --frozen --offline

%files
%license LICENSE LICENSE.dependencies
%doc README.md
%{_bindir}/%{name}
%{_datadir}/applications/io.github.snaggen.MdView.desktop
%{_datadir}/icons/hicolor/scalable/apps/io.github.snaggen.MdView.svg
%{_metainfodir}/io.github.snaggen.MdView.metainfo.xml

%changelog
* Wed Oct 14 2026 Mattias Eriksson <snaggen@tuta.com> - 0.1.0-1
- Initial package
