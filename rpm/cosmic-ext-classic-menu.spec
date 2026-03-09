Name:           cosmic-ext-classic-menu
Version:        0.0.13
Release:        1%{?dist}
Summary:        Classic Menu Applet

License:        GPLv3
URL:            https://github.com/championpeak87/cosmic-ext-classic-menu
Source0:        https://github.com/championpeak87/cosmic-ext-classic-menu/archive/refs/tags/%{version}.tar.gz

%define debug_package %{nil}

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  rust-xkbcommon-devel
BuildRequires:  just
Requires:       cosmic-osd

%description
Classic Menu is a Rust-based applet for COSMIC Desktop, providing an app menu launcher with apps divided into their respective categories.

%prep
%autosetup

%build
just build-release --verbose

%install
just rootdir=%{buildroot} install

%files
%{_bindir}/%{name}-applet
%{_bindir}/%{name}-settings
%{_datadir}/applications/com.championpeak87.cosmic-ext-classic-menu.desktop
%{_datadir}/metainfo/com.championpeak87.cosmic-ext-classic-menu.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/com.championpeak87.cosmic-ext-classic-menu.svg
%{_datadir}/cosmic/com.championpeak87.cosmic-ext-classic-menu/applet-buttons/*

%changelog
* Mon Mar 09 2026 Kamil Lihan <k.lihan@outlook.com> 0.0.13-1
- Patched issue with application icon loading

* Mon Jan 01 2026 Kamil Lihan <k.lihan@outlook.com> 0.0.12-1
- Added context menu for application (KNOWN ISSUE: misalignment when menu is scrolled)
- Added support for navigating the menu with arrow keys (KNOWN ISSUE: popup initially out of focus when opened)

* Mon Dec 27 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.11-1
- Fixed issue with flatpak applications not showing up in the list
- Added support for NixOS

* Mon Nov 24 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.10-1
- Rename to Classic Menu
- Update manifest file

* Wed Nov 19 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.9-1
- Fix flatpak issues

* Sat Oct 25 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.8-1
- Resolve performance issues

* Fri Oct 17 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.7-1
- Rename applet to cosmic-ext-classic-menu

* Mon Sep 29 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.6-1
- Resolve performance issues

* Sat Sep 27 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.5-1
- Patch popup positioning
- Ability to set custom icon

* Tue Sep 24 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.4-1
- Add configuration options

* Fri Sep 11 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.3-1
- Fix launching applications in flatpak version of the applet

* Sat May 17 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.2-1
- Layout updates

* Mon May 12 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.1-0.1.preview
- Initial test release