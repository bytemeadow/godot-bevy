{
  pkgs,
  lib,
  inputs,
  ...
}:
let
  overlays = [ (import inputs.rust-overlay) ];
  system = pkgs.stdenv.system;
  rustPkgs = import inputs.nixpkgs { inherit system overlays; };
  # visit rust-toolchain.toml to specify rust toolchain version and associated tools (clippy, etc)
  rust-toolchain = rustPkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  godot-bin = pkgs.callPackage ./nix/godot-bin.nix { };
  tracy = inputs.tracy.packages.${system}.default;
in
{
  # https://devenv.sh/basics/
  # https://devenv.sh/packages/
  packages =
    with pkgs;
    [
      #
      # Packages supporting all platforms
      #

      # dev tools
      sccache # cache rust build artifacts, ref https://github.com/mozilla/sccache
      python3 # for godot type generation script
      rust-toolchain

      # godot editor - version defined in nix/godot-bin.nix
      godot-bin
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
      #
      # Linux specific packages
      #
      alsa-lib
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

      # profiler - TODO: should work on mac but currently fails
      tracy
    ];

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
