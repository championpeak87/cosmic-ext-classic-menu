Name:           cosmic-classic-menu
Version:        0.0.5
Release:        1%{?dist}
Summary:        COSMIC Classic Menu Application

License:        GPLv3
URL:            https://github.com/championpeak87/cosmic-classic-menu
Source0:        https://github.com/championpeak87/cosmic-classic-menu/archive/refs/tags/%{version}.tar.gz

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
%{_bindir}/%{name}
%{_bindir}/%{name}-settings
%{_datadir}/applications/com.championpeak87.cosmic-classic-menu.desktop
%{_datadir}/metainfo/com.championpeak87.cosmic-classic-menu.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/com.championpeak87.cosmic-classic-menu.svg
%{_datadir}/cosmic/com.championpeak87.cosmic-classic-menu/applet-buttons/*

%changelog
* Wed Feb 19 2025 Kamil Lihan <k.lihan@outlook.com> - 0.0.1
- Initial package