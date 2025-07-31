{
  pkgs,
  lib,
  inputs,
  # config,
  ...
}:
let
  overlays = [ (import inputs.rust-overlay) ];
  system = pkgs.stdenv.system;
  rustPkgs = import inputs.nixpkgs { inherit system overlays; };
  # visit rust-toolchain.toml to specify rust toolchain version and associated tools (clippy, etc)
  rust-toolchain = rustPkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  tracy = inputs.tracy.packages.${pkgs.stdenv.system}.default;
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
      sccache # cache rust build artifacts, ref https://github.com/mozilla/sccache
      just # simple command runner via justfile, ref https://github.com/casey/just
      python3 # for godot type generation script
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

      # faster link times
      mold-wrapped

      # TODO this works fine on linux, and *should* work on mac (moved to the category above) but currently fails.
      tracy # profiler
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin (
      with pkgs.darwin.apple_sdk;
      [
        # apple stuff
      ]
    );

  # speed up rust builds through caching
  env.RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";

  files =
    if pkgs.stdenv.isLinux then
      # On linux, we get ~5x faster link times using mold
      # https://bevy.org/learn/quick-start/getting-started/setup/#enable-fast-compiles-optional
      {
        ".cargo/config.toml".text = ''
          [target.x86_64-unknown-linux-gnu]
          linker = "${pkgs.clang}/bin/clang"
          rustflags = ["-C", "link-arg=-fuse-ld=${pkgs.mold-wrapped}/bin/mold"]
        '';
      }
    else
      { };
}
