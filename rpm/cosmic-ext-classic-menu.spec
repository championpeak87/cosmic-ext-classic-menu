Name:           cosmic-ext-classic-menu
Version:        1.1.0
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
* Sat Oct 17 2026 Kamil Lihan <k.lihan@outlook.com> 1.1.0-1
- New Whistler layout: a start page with favorite and recently used applications, leading to all applications
- Quick launcher type: favorites can be shown in place of the recently used apps, as a category and below the app grid of the Modern layout
- Choose the category the menu opens on
- Column or grid view of applications in any layout, with a choice of size and of showing application descriptions, on hover in the grid
- Add applications to the favorites by dragging them onto the favorites
- Drag applications from the menu into the dock to pin them
- --toggle option to open or close the menu, e.g. from a keyboard shortcut
- Applet description shown in COSMIC Settings
- New settings translated into all languages
- New screenshots of all layouts
- Fixed icons given as a path to a PNG image, e.g. of Citrix applications, showing the default icon
- Fixed applet crashing with PNG based icon themes
- Fixed short application lists showing only their first four applications
- Fixed the highlight of applications in the column view not reaching the edges of the list

* Thu Oct 08 2026 Kamil Lihan <k.lihan@outlook.com> 1.0.0-1
- Five menu layouts to choose from: Classic, Retro, Compact, Sidebar and Modern
- Drag and drop layout editor in the settings: move the search field and power controls, swap and resize the panes
- Places panel as an alternative to categories, with customizable order and separators
- Favorite applications in the Sidebar layout, added from the application context menu
- Menu opens faster and uses less memory
- Settings changes are applied to the menu immediately
- Fixed context menu position when the application list is scrolled

* Mon Jun 12 2026 Kamil Lihan <k.lihan@outlook.com> 0.0.14-1
- Patched issue with applet crashing
- Fixed scrolling of the app list when using arrow keys

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