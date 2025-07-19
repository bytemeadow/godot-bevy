# copied/modified from https://github.com/mlabs-haskell/godot-cardano/blob/af8642895adf1c08efd2374f09fd7c1282484b34/nix/godot-bin.nix
{ stdenv, lib, autoPatchelfHook, makeWrapper, fetchurl, unzip, alsa-lib, dbus
, fontconfig, udev, vulkan-loader, libpulseaudio, libGL, libXcursor, libXinerama
, libxkbcommon, libXrandr, libXrender, libX11, libXext, libXi, speechd, wayland
, libdecor }:
let
  godot-version = "4.5-dev5";
  qualifier = ""; # e.g. `-stable`
in stdenv.mkDerivation rec {
  pname = "godot-latest";
  version = "${godot-version}";
  src = fetchurl {
    # https://github.com/godotengine/godot-builds/releases/download/4.5-dev5/Godot_v4.5-dev5_linux.x86_64.zip
    url =
      "https://github.com/godotengine/godot-builds/releases/download/${version}${qualifier}/Godot_v${version}${qualifier}_linux.x86_64.zip";
    sha256 = "sha256-19eEOrta4z8KO+3Q1ZzwWPJnO6xVw5gTYGfymqxsxZ8=";
  };

  nativeBuildInputs = [ autoPatchelfHook makeWrapper unzip ];

  buildInputs = [
    alsa-lib
    dbus
    dbus.lib
    fontconfig
    # libdecor # <- For client-side decorations (look bad)
    libGL
    libX11
    libXcursor
    libXext
    libXi
    libXinerama
    libXrandr
    libXrender
    libpulseaudio
    libxkbcommon
    speechd
    udev
    vulkan-loader
    wayland
  ];

  libraries = lib.makeLibraryPath buildInputs;

  unpackCmd = "unzip $curSrc -d source";
  installPhase = ''
    mkdir -p $out/bin
    install -m 0755 Godot_v${version}${qualifier}_linux.x86_64 $out/bin/godot-latest
  '';

  postFixup = ''
    wrapProgram $out/bin/godot-latest \
      --set LD_LIBRARY_PATH ${libraries}
  '';
}
