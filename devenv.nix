{
  pkgs,
  nixpkgs,
  rust-overlay,
  lib,
  # config,
  # inputs,
  ...
}:
let
  overlays = [ (import rust-overlay) ];
  system = pkgs.stdenv.system;
  rustPkgs = import nixpkgs { inherit system overlays; };
  # visit rust-toolchain.toml to specify rust toolchain version and associated tools (clippy, etc)
  rust-toolchain = rustPkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
in
{
  # https://devenv.sh/basics/

  # may be useful - wrt bevy/devenv - https://github.com/cachix/devenv/issues/1681

  # https://devenv.sh/overlays/#common-use-cases
  overlays = [
    (final: prev: {
      # Add a package from local derivations, in this case, we're
      # typically providing a version of godot newer than stable
      godot-latest = final.callPackage ./nix/godot-bin.nix { };
    })
  ];

  # https://devenv.sh/packages/
  packages =
    with pkgs;
    [
      #
      # Packages supporting all platforms, typically cross-platform developer tools
      #

      # dev tools
      samply # profiler, ref https://github.com/mstange/samply
      sccache # cache rust build artifacts, ref https://github.com/mozilla/sccache
      just # simple command runner via justfile, ref https://github.com/casey/just
      rust-toolchain
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
      #
      # Linux specific packages
      #
      alsa-lib
      godot # tracks stable releases, provides `godot` binary
      godot-latest # tracks development releases, provides `godot-latest` binary
      # libdecor # <- For client-side decorations (look bad)
      libGL
      libxkbcommon
      pkg-config
      udev
      vulkan-headers
      vulkan-loader
      vulkan-tools
      vulkan-validation-layers
      wayland

      # execution of godot-exported binaries in a FHS-like environment
      # https://nix.dev/permalink/stub-ld
      steam-run
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin (
      with pkgs.darwin.apple_sdk;
      [
        # apple stuff
      ]
    );

  # speed up rust builds through caching
  env.RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";

  # https://devenv.sh/git-hooks/
  git-hooks.hooks = {
    # lint shell scripts
    shellcheck.enable = true;

    rustfmt.enable = true;

    # some hooks have more than one package, like clippy:
    clippy.enable = true;
    clippy.packageOverrides.cargo = pkgs.cargo;
    clippy.packageOverrides.clippy = pkgs.clippy;
    clippy.settings.allFeatures = true;
  };
}
