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
  # Pinned nixpkgs for emscripten 3.1.73 (godot-rust web export compatibility)
  emscriptenPkgs = import inputs.nixpkgs-emscripten { inherit system; };
  # visit rust-toolchain.toml to specify rust toolchain version and associated tools (clippy, etc)
  rust-toolchain = rustPkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  # nightly toolchain for web builds (-Zbuild-std requires nightly)
  rust-nightly = rustPkgs.rust-bin.nightly.latest.default.override {
    extensions = [ "rust-src" ];
  };
  godot-bin = pkgs.callPackage ./nix/godot-bin.nix { };
  # tracy profiler - version pinned in devenv.yaml
  # On macOS, zig build needs framework search paths from system SDK
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
      rust-nightly # for web builds (-Zbuild-std requires nightly)

      # web export support - pinned to emscripten 3.1.73 for godot-rust compatibility
      # The latest nixpkgs has emscripten 4.x which causes linker errors with godot-rust
      # We also need binaryen from the same pinned nixpkgs for version compatibility
      emscriptenPkgs.emscripten
      emscriptenPkgs.binaryen

      # godot editor - version defined in nix/godot-bin.nix
      godot-bin
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
      # tracy profiler - version pinned in devenv.yaml
      tracy

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


    ];

  # speed up rust builds through caching
  env.RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";

  # nightly toolchain paths for web builds
  env.CARGO_NIGHTLY = "${rust-nightly}/bin/cargo";
  env.RUSTC_NIGHTLY = "${rust-nightly}/bin/rustc";

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
