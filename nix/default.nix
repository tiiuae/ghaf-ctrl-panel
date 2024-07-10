# Copyright 2024 TII (SSRC) and the Ghaf contributors
# SPDX-License-Identifier: Apache-2.0
{
  cargo,
  glib,
  gtk4,
  lib,
  libadwaita,
  librsvg,
  pkg-config,
  protobuf,
  protoc-gen-tonic,
  rustPlatform,
  rustc,
  version ? "git",
}:
rustPlatform.buildRustPackage {
  pname = "ghaf-ctrl-panel";
  inherit version;
  meta = with lib; {
    description = "Ghaf Control Panel";
    license = licenses.asl20;
    mainProgram = "ctrl-panel";
  };

  buildInputs = [
    gtk4
    glib
    libadwaita
  ];

  nativeBuildInputs = [
    gtk4
    glib
    rustPlatform.cargoSetupHook
    rustc
    cargo
    pkg-config
    protoc-gen-tonic
    protobuf
  ];

  gappsWrapperArgs = ''
    --prefix XDG_DATA_DIRS : "${librsvg}/share"
  '';

  cargoLock.lockFile = ../Cargo.lock;

  src = ./..;
}
