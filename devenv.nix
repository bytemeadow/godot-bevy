{ pkgs, lib, config, inputs, ... }:

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
  packages = with pkgs;
    [
      alsa-lib
      godot # tracks stable releases, provides `godot` binary
      godot-latest # tracks development releases, provides `godot-latest` binary
      pkg-config
      udev
      vulkan-headers
      vulkan-loader
      vulkan-tools
      vulkan-validation-layers

      # To use the wayland feature
      wayland
      libxkbcommon

      libGL
      # libdecor # <- For client-side decorations (look bad)

      # execution of godot-exported binaries in a FHS-like environment
      # https://nix.dev/permalink/stub-ld
      steam-run
    ] ++ lib.optionals pkgs.stdenv.isDarwin
    (with pkgs.darwin.apple_sdk; [ frameworks.Security ]);

  # https://devenv.sh/languages/
  languages.rust.enable = true;

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
