{ lib
, fetchFromGitHub
, rustPlatform
, pkg-config
, just
, stdenv
, glib
, gtk3
, libcosmicAppHook
}:

rustPlatform.buildRustPackage rec {
  pname = "cosmic-ext-classic-menu-plus";
  version = "0.1.3";

  src = fetchFromGitHub {
    owner = "shagovAlexei";
    repo = "cosmic-ext-classic-menu-plus";
    rev = version;
    # Replace with the hash Nix reports on the first build
    hash = lib.fakeHash;
  };

  # This is required for Rust projects that don't have a vendor folder
  # You can use 'lib.fakeHash' initially to get the correct hash from the error message
  cargoHash = lib.fakeHash;

  nativeBuildInputs = [
    pkg-config
    just
    libcosmicAppHook
  ];

  buildInputs = [
    glib
    gtk3
  ];

  # COSMIC applets usually use 'just' for specific install tasks
  # but buildRustPackage handles the cargo build automatically.
  # If the applet needs specific RON files moved to /share/cosmic, 
  # you might need a postInstall hook.  
  dontCargoBuild = true;
  buildPhase = ''
    runHook preBuild
    just build-release
    runHook postBuild
  '';

  # Override the default cargo build to use 'just'
  justFlags = [
    "--set"
    "prefix"
    (placeholder "out")
    "--set"
    "bin-src"
    "target/release/cosmic-ext-classic-menu-plus-applet"
    "--set"
    "settings-bin-src"
    "target/release/cosmic-ext-classic-menu-plus-settings"
  ];
  
  meta = with lib; {
    description = "A classic-style application menu for the COSMIC Desktop";
    homepage = "https://github.com/shagovAlexei/cosmic-ext-classic-menu-plus";
    license = licenses.gpl3Only;
    maintainers = [ ];
    platforms = platforms.linux;
  };
}
