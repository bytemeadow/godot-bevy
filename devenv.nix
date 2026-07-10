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
  emscriptenPkgs = import inputs.nixpkgs-emscripten { inherit system; };
  rust-toolchain = rustPkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  # -Zbuild-std (web builds) requires nightly
  rust-nightly = rustPkgs.rust-bin.nightly.latest.default.override {
    extensions = [ "rust-src" ];
  };
  godot-bin = pkgs.callPackage ./nix/godot-bin.nix { };
  tracy = inputs.tracy.packages.${system}.default;
in
{
  packages =
    with pkgs;
    [
      sccache # rust build artifact cache
      python3 # godot type generation script
      mdbook # builds book/
      rust-toolchain
      rust-nightly # web builds
      act # run GitHub Actions locally

      # emscripten 4.x breaks the godot-rust linker, so pin 3.1.73 (+ matching binaryen)
      emscriptenPkgs.emscripten
      emscriptenPkgs.binaryen

      godot-bin
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
      tracy

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

      # FHS-like env for running godot-exported binaries, https://nix.dev/permalink/stub-ld
      steam-run

      mold-wrapped # faster link times
    ];

  env.RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";

  # nightly for web builds
  env.CARGO_NIGHTLY = "${rust-nightly}/bin/cargo";
  env.RUSTC_NIGHTLY = "${rust-nightly}/bin/rustc";

  # rustfmt runs on commit; clippy stays in CI
  git-hooks.hooks.rustfmt.enable = true;

  # enable claude code
  claude.code.enable = true;

  scripts = {
    ci-test.exec = ''
      echo "Running CI locally with act..."
      echo "Note: Docker must be running"
      act workflow_dispatch -W .github/workflows/ci.yml --container-architecture linux/amd64 "$@"
    '';

    ci-itest.exec = ''
      echo "Running integration tests locally with act..."
      echo "Note: Docker must be running"
      act workflow_dispatch -W .github/workflows/ci.yml -j integration-tests --container-architecture linux/amd64 "$@"
    '';

    ci-benches.exec = ''
      echo "Running benchmarks locally with act..."
      echo "Note: Docker must be running"
      act workflow_dispatch -W .github/workflows/benchmarks.yml --container-architecture linux/amd64 "$@"
    '';

    ci-lint.exec = ''
      echo "Running lint checks..."
      cargo fmt --all -- --check
      cargo clippy --all-targets -- -D warnings
    '';

    # native, needs local godot
    itest.exec = ''
      echo "Running integration tests..."
      cd itest && ./run-tests.sh "$@"
    '';

    # native, needs local godot
    bench.exec = ''
      echo "Running benchmarks..."
      cd itest && ./run-benches.sh "$@"
    '';

    book.exec = ''
      cd book && mdbook build "$@"
    '';

    book-serve.exec = ''
      cd book && mdbook serve "$@"
    '';
  };

  files =
    if pkgs.stdenv.isLinux then
      # mold gives ~5x faster link times on linux
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
