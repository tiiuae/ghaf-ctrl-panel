{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;

        # FIXME: disable source cleaning, at the moment it give more problems than benefits
        #src = craneLib.cleanCargoSource ./.;
        src = ./.;

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;
        
          nativeBuildInputs = [
             pkgs.pkg-config
             pkgs.glib
             pkgs.protobuf
             pkgs.wrapGAppsHook4
             pkgs.dbus
          ];
          buildInputs = [
            # Add additional build inputs here
             pkgs.glib
             pkgs.cairo
             pkgs.pango
             pkgs.gtk4
             pkgs.libadwaita
             pkgs.dbus
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        my-crate = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          postUnpack = ''
            find .
          '';
          postFixup = ''
            wrapProgram $out/bin/ctrl-panel \
              --prefix PATH : ${lib.makeBinPath [ pkgs.glibc ]} \
              --prefix PATH : ${lib.makeBinPath [ pkgs.dmidecode ]} \
              --prefix PATH : ${lib.makeBinPath [ pkgs.zenity ]}
          '';
        });
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit my-crate;
        };

        packages = {
          default = my-crate;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = my-crate;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
             pkgs.glib
             pkgs.gtk4
             pkgs.libadwaita
             pkgs.pkg-config
             pkgs.protobuf
             pkgs.dbus
          ];
        };
      });
}
