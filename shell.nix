{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.glib
    pkgs.gtk4
    pkgs.libadwaita
    pkgs.pkg-config
    pkgs.gcc
    pkgs.rustc
    pkgs.cargo
    pkgs.protoc-gen-tonic
  ];

  shellHook = ''
    export PKG_CONFIG_PATH="${pkgs.glib.dev}/lib/pkgconfig:${pkgs.gtk4.dev}/lib/pkgconfig"
  '';
}
