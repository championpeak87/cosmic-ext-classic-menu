Name:           cosmic-ext-classic-menu-plus
Version:        0.1.3
Release:        1%{?dist}
Summary:        Classic Menu Plus Applet

License:        GPLv3
URL:            https://github.com/shagovAlexei/cosmic-ext-classic-menu-plus
Source0:        https://github.com/shagovAlexei/cosmic-ext-classic-menu-plus/archive/refs/tags/%{version}.tar.gz

%define debug_package %{nil}

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  rust-xkbcommon-devel
BuildRequires:  just
Requires:       cosmic-osd

%description
Classic Menu Plus is a Rust-based applet for COSMIC Desktop, providing an app menu launcher with apps divided into their respective categories.
It is a fork of cosmic-ext-classic-menu (https://github.com/championpeak87/cosmic-ext-classic-menu)
with favorites, custom categories, hibernate and appearance settings.

%prep
%autosetup

%build
just build-release --verbose

%install
just rootdir=%{buildroot} install

%files
%{_bindir}/%{name}-applet
%{_bindir}/%{name}-settings
%{_datadir}/applications/io.github.shagovAlexei.cosmic-ext-classic-menu-plus.desktop
%{_datadir}/metainfo/io.github.shagovAlexei.cosmic-ext-classic-menu-plus.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/io.github.shagovAlexei.cosmic-ext-classic-menu-plus.svg
%{_datadir}/cosmic/io.github.shagovAlexei.cosmic-ext-classic-menu-plus/applet-buttons/*

%changelog
* Wed Oct 14 2026 Shagov Alexei <shagov.alexei@gmail.com> 0.1.3-1
- Settings button in the menu header, the user name opens COSMIC user settings

* Wed Oct 14 2026 Shagov Alexei <shagov.alexei@gmail.com> 0.1.2-1
- About page in the settings with a "Support the project" button

* Wed Sep 30 2026 Shagov Alexei <shagov.alexei@gmail.com> 0.1.1-1
- New application icon

* Mon Sep 28 2026 Shagov Alexei <shagov.alexei@gmail.com> 0.1.0-1
- First release of the Classic Menu Plus fork
- Hibernate button with confirmation, configurable power buttons
- Favorites, custom categories, moving and hiding apps
- Popup size, icon size and compact list density
- Fixed context menu position when the list is scrolled

* Mon Jun 12 2026 Shagov Alexei <shagov.alexei@gmail.com> 0.0.14-1
- Patched issue with applet crashing
- Fixed scrolling of the app list when using arrow keys

* Mon Mar 09 2026 Shagov Alexei <shagov.alexei@gmail.com> 0.0.13-1
- Patched issue with application icon loading

* Mon Jan 01 2026 Shagov Alexei <shagov.alexei@gmail.com> 0.0.12-1
- Added context menu for application (KNOWN ISSUE: misalignment when menu is scrolled)
- Added support for navigating the menu with arrow keys (KNOWN ISSUE: popup initially out of focus when opened)

* Mon Dec 27 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.11-1
- Fixed issue with flatpak applications not showing up in the list
- Added support for NixOS

* Mon Nov 24 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.10-1
- Rename to Classic Menu
- Update manifest file

* Wed Nov 19 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.9-1
- Fix flatpak issues

* Sat Oct 25 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.8-1
- Resolve performance issues

* Fri Oct 17 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.7-1
- Rename applet to cosmic-ext-classic-menu

* Mon Sep 29 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.6-1
- Resolve performance issues

* Sat Sep 27 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.5-1
- Patch popup positioning
- Ability to set custom icon

* Tue Sep 24 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.4-1
- Add configuration options

* Fri Sep 11 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.3-1
- Fix launching applications in flatpak version of the applet

* Sat May 17 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.2-1
- Layout updates

* Mon May 12 2025 Shagov Alexei <shagov.alexei@gmail.com> 0.0.1-0.1.preview
- Initial test release