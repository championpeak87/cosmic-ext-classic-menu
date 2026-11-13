# Builds the checked out sources, e.g. with
#   nix-build -E 'with import <nixpkgs> { }; callPackage ./package.nix { }'
{
  lib,
  stdenv,
  rustPlatform,
  just,
  libcosmicAppHook,
}:

let
  target = "target/${stdenv.hostPlatform.rust.cargoShortTarget}/release";
in
rustPlatform.buildRustPackage {
  pname = "cosmic-ext-classic-menu";
  version = "1.2.0";

  # Only what the build needs, leaving out e.g. build output.
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./justfile
      ./LICENSE
      ./applet
      ./settings
      ./res
    ];
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
    # Fetches the git dependencies, e.g. libcosmic, without listing their
    # hashes here.
    allowBuiltinFetchGit = true;
  };

  nativeBuildInputs = [
    just
    libcosmicAppHook
  ];

  # Cargo builds the binaries, just installs them with the other files.
  dontUseJustBuild = true;
  dontUseJustCheck = true;
  justFlags = [
    "--set"
    "prefix"
    (placeholder "out")
    "--set"
    "bin-src"
    "${target}/cosmic-ext-classic-menu-applet"
    "--set"
    "settings-bin-src"
    "${target}/cosmic-ext-classic-menu-settings"
  ];

  meta = {
    description = "Classic application menu applet for the COSMIC desktop";
    homepage = "https://github.com/championpeak87/cosmic-ext-classic-menu";
    license = lib.licenses.gpl3Only;
    mainProgram = "cosmic-ext-classic-menu-applet";
    platforms = lib.platforms.linux;
  };
}
