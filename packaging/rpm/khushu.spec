Name:           khushu
Version:        1.0.0
Release:        1%{?dist}
Summary:        An all-in-one Muslim app for Linux

License:        GPLv3+
URL:            https://github.com/sniper1720/khushu
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz

%define debug_package %{nil}

BuildRequires:  meson
BuildRequires:  ninja-build
BuildRequires:  gcc
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gtk4-devel
BuildRequires:  libadwaita-devel
BuildRequires:  pkgconf-pkg-config
BuildRequires:  geoclue2-devel
BuildRequires:  alsa-lib-devel
BuildRequires:  openssl-devel
BuildRequires:  gettext

Requires:       hicolor-icon-theme

%description
Khushu is a modern GNOME application that provides prayer times,
Qibla direction, Adkar, and mosque search. It features a clean,
adaptive UI built with GTK4 and Libadwaita.

%prep
%autosetup

%build
%meson
%meson_build

%install
%meson_install

%check
%meson_test

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}
%{_datadir}/applications/io.github.sniper1720.khushu.desktop
%{_datadir}/dbus-1/services/io.github.sniper1720.khushu.service
%{_datadir}/icons/hicolor/scalable/apps/io.github.sniper1720.khushu.svg
%{_datadir}/icons/hicolor/symbolic/apps/io.github.sniper1720.khushu-symbolic.svg
%{_datadir}/metainfo/io.github.sniper1720.khushu.metainfo.xml
%{_datadir}/%{name}/
%{_datadir}/locale/*/LC_MESSAGES/khushu.mo
%{_datadir}/fonts/truetype/%{name}/

%changelog
* Fri Mar 06 2026 Djalel Oukid <sniper1720@linuxtechmore.com> - 1.0.0-1
- Initial release.
