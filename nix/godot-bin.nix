# Cross-platform Godot binary package
# Fetches official binaries from godotengine/godot-builds
{
  stdenv,
  lib,
  fetchurl,
  unzip,
  autoPatchelfHook,
  makeWrapper,
  # Linux dependencies
  alsa-lib ? null,
  dbus ? null,
  fontconfig ? null,
  libGL ? null,
  libpulseaudio ? null,
  libX11 ? null,
  libXcursor ? null,
  libXext ? null,
  libXi ? null,
  libXinerama ? null,
  libxkbcommon ? null,
  libXrandr ? null,
  libXrender ? null,
  speechd ? null,
  udev ? null,
  vulkan-loader ? null,
  wayland ? null,
}:
let
  version = "4.5.1";
  qualifier = "stable";

  sources = {
    # macOS universal binary (works on both x86_64 and aarch64)
    "x86_64-darwin" = {
      url = "https://github.com/godotengine/godot-builds/releases/download/${version}-${qualifier}/Godot_v${version}-${qualifier}_macos.universal.zip";
      hash = "sha256-ZcJ5WdAqqs/BMex+y5AXm6gEUgDLApgr8r6W0RcBC4o=";
      executable = "Godot.app/Contents/MacOS/Godot";
    };
    "aarch64-darwin" = {
      url = "https://github.com/godotengine/godot-builds/releases/download/${version}-${qualifier}/Godot_v${version}-${qualifier}_macos.universal.zip";
      hash = "sha256-ZcJ5WdAqqs/BMex+y5AXm6gEUgDLApgr8r6W0RcBC4o=";
      executable = "Godot.app/Contents/MacOS/Godot";
    };
    "x86_64-linux" = {
      url = "https://github.com/godotengine/godot-builds/releases/download/${version}-${qualifier}/Godot_v${version}-${qualifier}_linux.x86_64.zip";
      hash = "sha256-AuxT0czNu9nPvMxjU1Oys0NEQRJEoEKLPy+WoMeHiM0=";
      executable = "Godot_v${version}-${qualifier}_linux.x86_64";
    };
    "aarch64-linux" = {
      url = "https://github.com/godotengine/godot-builds/releases/download/${version}-${qualifier}/Godot_v${version}-${qualifier}_linux.arm64.zip";
      hash = "sha256-SkxtbQYGrMnQD3dVRkUp8SKy/LIhJEdU+JoNAyhwiA8=";
      executable = "Godot_v${version}-${qualifier}_linux.arm64";
    };
  };

  src = sources.${stdenv.system} or (throw "Unsupported system: ${stdenv.system}");

  # Linux runtime dependencies
  linuxLibs = [
    alsa-lib
    dbus
    dbus.lib
    fontconfig
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
in
stdenv.mkDerivation {
  pname = "godot-bin";
  inherit version;

  src = fetchurl {
    inherit (src) url hash;
  };

  nativeBuildInputs = [ unzip ]
    ++ lib.optionals stdenv.isLinux [ autoPatchelfHook makeWrapper ];

  buildInputs = lib.optionals stdenv.isLinux linuxLibs;

  unpackPhase = "unzip $src";

  installPhase = if stdenv.isDarwin then ''
    mkdir -p $out/Applications $out/bin
    cp -r Godot.app $out/Applications/
    ln -s "$out/Applications/Godot.app/Contents/MacOS/Godot" $out/bin/godot
  '' else ''
    mkdir -p $out/bin
    install -m 0755 "${src.executable}" $out/bin/godot
  '';

  postFixup = lib.optionalString stdenv.isLinux ''
    wrapProgram $out/bin/godot \
      --set LD_LIBRARY_PATH ${lib.makeLibraryPath linuxLibs}
  '';

  meta = with lib; {
    description = "Godot Engine - Multi-platform 2D and 3D game engine";
    homepage = "https://godotengine.org";
    license = licenses.mit;
    platforms = builtins.attrNames sources;
    mainProgram = "godot";
  };
}
