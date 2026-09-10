{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    ghaf-givc = {
      url = "git+https://github.com/slakkala/ghaf-givc?ref=update-gui";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      treefmt-nix,
      ghaf-givc,
      ...
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        treefmtEval = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";

          programs.nixfmt.enable = true;
          programs.rustfmt.enable = true;
          programs.taplo.enable = true;
        };

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
          ]
          ++ lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
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
        my-crate = craneLib.buildPackage (
          commonArgs
          // {
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
          }
        );

        ctrlPanelTestAutomation = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--features test-automation";
            postFixup = ''
              wrapProgram $out/bin/ctrl-panel \
                --prefix PATH : ${lib.makeBinPath [ pkgs.glibc ]} \
                --prefix PATH : ${lib.makeBinPath [ pkgs.dmidecode ]} \
                --prefix PATH : ${lib.makeBinPath [ pkgs.zenity ]}
            '';
          }
        );

        guiIntegrationTest = import ./nix/gui-integration-test.nix {
          inherit
            pkgs
            lib
            crane
            system
            ghaf-givc
            ;
          ctrlPanel = ctrlPanelTestAutomation;
        };
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit my-crate;
          treefmt = treefmtEval.config.build.check self;
          gui-integration = guiIntegrationTest;
        };

        formatter = treefmtEval.config.build.wrapper;

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
            pkgs.cargo-edit
            treefmtEval.config.build.wrapper
            (pkgs.writeShellScriptBin "update-deps" (builtins.readFile ./scripts/update-deps.sh))
          ];
        };
      }
    )
    // {
      overlays.default = final: prev: {
        ctrl-panel = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };
    };
}
