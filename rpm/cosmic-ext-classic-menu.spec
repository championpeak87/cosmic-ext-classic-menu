Name:           cosmic-ext-classic-menu
Version:        0.0.9
Release:        1%{?dist}
Summary:        COSMIC Classic Menu Application

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
COSMIC Classic Menu is a Rust-based applet for COSMIC Desktop, providing an app menu launcher with apps divided into their respective categories.

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
* St Nov 19 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.9-1
- Fix flatpak issues

* So Okt 25 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.8-1
- Resolve performance issues

* Pi Okt 17 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.7-1
- Rename applet to cosmic-ext-classic-menu

* Po Sep 29 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.6-1
- Resolve performance issues

* So Sep 27 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.5-1
- Patch popup positioning
- Ability to set custom icon

* Ut Sep 24 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.4-1
- Add configuration options

* Pi Sep 11 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.3-1
- Fix launching applications in flatpak version of the applet

* So Máj 17 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.2-1
- Layout updates

* Po Máj 12 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.1-0.1.preview
- Initial test release