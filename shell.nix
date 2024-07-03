# Copyright 2024 TII (SSRC) and the Ghaf contributors
# SPDX-License-Identifier: Apache-2.0
{pkgs ? import <nixpkgs> {}}:
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
    pkgs.protobuf
  ];

  shellHook = ''
    export PKG_CONFIG_PATH="${pkgs.glib.dev}/lib/pkgconfig:${pkgs.gtk4.dev}/lib/pkgconfig"
  '';
}
