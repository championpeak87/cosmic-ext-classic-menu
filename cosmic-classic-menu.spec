Name:           cosmic-classic-menu
Version:        0.0.1
Release:        1%{?dist}
Summary:        COSMIC Classic Menu Application

License:        GPLv2
URL:            https://example.com/cosmic-classic-menu
Source0:        %{name}-%{version}.tar.gz

%define debug_package %{nil}

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  rust-xkbcommon-devel
Requires:       cosmic-osd

%description
COSMIC Classic Menu is a Rust-based applet for COSMIC Desktop, providing an app menu launcher with apps divided into their respective categories.

%prep
%autosetup

%build
cargo build --release
strip target/release/%{name}

%install
install -Dm755 target/release/%{name} %{buildroot}%{_bindir}/%{name}
install -Dm0644 data/com.championpeak87.cosmic-classic-menu.desktop %{buildroot}%{_datadir}/applications/com.championpeak87.cosmic-classic-menu.desktop
install -Dm0644 data/icons/com.championpeak87.cosmic-classic-menu.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/com.championpeak87.cosmic-classic-menu.svg

%files
%{_bindir}/%{name}
%{_datadir}/applications/com.championpeak87.cosmic-classic-menu.desktop
%{_datadir}/icons/hicolor/scalable/apps/com.championpeak87.cosmic-classic-menu.svg

%changelog
* Wed Feb 19 2025 Kamil Lihan <k.lihan@outlook.com> - 0.0.1
- Initial package