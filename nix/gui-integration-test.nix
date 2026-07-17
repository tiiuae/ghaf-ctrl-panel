{ pkgs, lib, crane, ctrlPanel, system, ghaf-givc }:
let
  ghafGivcSrc = ghaf-givc.outPath;

  givcAdminPkg = import (ghafGivcSrc + "/nixos/packages/givc-admin.nix") {
    inherit lib pkgs crane;
    protobuf = pkgs.protobuf;
    src = ghafGivcSrc;
  };

  givcAgentPkg = import (ghafGivcSrc + "/nixos/packages/givc-agent.nix") {
    inherit pkgs;
    src = ghafGivcSrc;
  };

  givcSelf = {
    packages."${system}" = {
      "givc-admin" = givcAdminPkg;
      "givc-agent" = givcAgentPkg;
      "ota-update" = givcAdminPkg.ota;
      "ota-update-server" = givcAdminPkg.update_server;
    };
  };

  adminModule = import (ghafGivcSrc + "/nixos/modules/admin.nix") { self = givcSelf; };
  hostModule = import (ghafGivcSrc + "/nixos/modules/host.nix") { self = givcSelf; };
  sysvmModule = import (ghafGivcSrc + "/nixos/modules/sysvm.nix") { self = givcSelf; };
  otaUpdateServerModule = import (ghafGivcSrc + "/nixos/modules/update-server.nix") { self = givcSelf; };

  adminAddr = {
    name = "admin-vm";
    addr = "192.168.101.10";
    port = "9001";
    protocol = "tcp";
  };

  hostAddr = {
    name = "ghaf-host";
    addr = "192.168.101.2";
    port = "9000";
    protocol = "tcp";
  };

  guiAddr = {
    name = "gui-vm";
    addr = "192.168.101.3";
    port = "9000";
    protocol = "tcp";
  };

  netAddr = {
    name = "net-vm";
    addr = "192.168.101.200";
    port = "9000";
    protocol = "tcp";
  };

  ctrlPanelArgs = [
    "--name"
    "admin-vm"
    "--addr"
    "192.168.101.10"
    "--port"
    "9001"
    "--cacert"
    "/etc/givc/ca-cert.pem"
    "--cert"
    "/etc/givc/cert.pem"
    "--key"
    "/etc/givc/key.pem"
    "--wireguardlist"
    "/etc/ctrl-panel/wireguard-gui-vms.txt"
    "--log-output"
    "stdout"
    "--log-level"
    "info"
  ];

  ctrlPanelCommand = "${ctrlPanel}/bin/ctrl-panel ${lib.escapeShellArgs ctrlPanelArgs}";

  launchCtrlPanel = pkgs.writeShellScriptBin "launch-ctrl-panel" ''
    #!${pkgs.runtimeShell}
    set -eu
    export XDG_RUNTIME_DIR=/run/user/1000
    export SWAYSOCK=/tmp/sway-ipc.sock
    export WAYLAND_DISPLAY=wayland-1
    export GDK_BACKEND=wayland
    export WLR_RENDERER=pixman

    ${ctrlPanelCommand} >/tmp/ctrl-panel.log 2>&1 &
    touch /tmp/ctrl-panel-started
  '';

  updateServerKey = "${ghafGivcSrc}/nixos/tests/snakeoil/nix-serve.key";
in
let
  runNixOSTest =
    if pkgs ? testers && pkgs.testers ? runNixOSTest then
      pkgs.testers.runNixOSTest
    else
      pkgs.nixosTest;
in
runNixOSTest {
  name = "ctrl-panel-gui-integration";

  nodes = {
    adminvm =
      { ... }:
      {
        imports = [
          adminModule
          (ghafGivcSrc + "/nixos/tests/snakeoil/gen-test-certs.nix")
        ];
        givc-tls-test = {
          name = adminAddr.name;
          addresses = adminAddr.addr;
        };
        networking.interfaces.eth1.ipv4.addresses = lib.mkOverride 0 [
          {
            address = adminAddr.addr;
            prefixLength = 24;
          }
        ];
        givc.admin = {
          enable = true;
          debug = true;
          name = adminAddr.name;
          addresses = [ adminAddr ];
          services = [
            "microvm@ghaf-host.service"
            "microvm@net-vm.service"
          ];
          tls.enable = true;
        };
        systemd.services.givc-admin.environment.GIVC_MONITORING = "false";
      };

    hostvm =
      { pkgs, config, ... }:
      let
        mockGhafVersion = pkgs.writeShellScriptBin "ghaf-version" ''
          #!${pkgs.runtimeShell}
          printf '%s\n' "$0 $*" >> /tmp/host-sysinfo-calls
          echo "mock-ghaf-version"
        '';

        mockBootctl = pkgs.writeShellScriptBin "bootctl" ''
          #!${pkgs.runtimeShell}
          printf '%s\n' "$0 $*" >> /tmp/host-sysinfo-calls
          cat <<'EOF'
          System:
               Secure Boot: enabled
          EOF
        '';

        mockLsblk = pkgs.writeShellScriptBin "lsblk" ''
          #!${pkgs.runtimeShell}
          printf '%s\n' "$0 $*" >> /tmp/host-sysinfo-calls
          echo "crypt"
        '';

        mockOtaUpdate = pkgs.writeShellScriptBin "ota-update" ''
          #!${pkgs.runtimeShell}
          set -eu

          printf '%s\n' "$*" >> /tmp/host-ota-update-calls

          case "$1" in
            get)
              cat <<'EOF'
          [
            {
              "generation": 1,
              "nixosVersion": "mock-nixos",
              "kernelVersion": "mock-kernel",
              "configurationRevision": "mock-revision",
              "storePath": "/nix/store/mock-generation",
              "current": true
            }
          ]
          EOF
              ;;
            *)
              echo "hostvm ota-update only supports get: $*" >&2
              exit 1
              ;;
          esac
        '';
      in
      {
        imports = [
          hostModule
          (ghafGivcSrc + "/nixos/tests/snakeoil/gen-test-certs.nix")
        ];
        givc-tls-test = {
          name = hostAddr.name;
          addresses = hostAddr.addr;
        };
        nix.enable = true;
        virtualisation.writableStore = true;
        virtualisation.writableStoreUseTmpfs = true;
        boot.loader.systemd-boot.enable = true;
        users.mutableUsers = false;
        networking.interfaces.eth1.ipv4.addresses = lib.mkOverride 0 [
          {
            address = hostAddr.addr;
            prefixLength = 24;
          }
        ];
        networking.extraHosts = ''
          192.168.101.200 test-updates.example.com
        '';
        nixpkgs.overlays = lib.mkForce [
          (_final: _prev: {
            inherit system;
          })
          (_final: _prev: {
            ota-update = mockOtaUpdate;
          })
        ];
        systemd.services."givc-${hostAddr.name}".path = lib.mkForce [
          mockGhafVersion
          mockBootctl
          mockLsblk
          config.system.path
          mockOtaUpdate
          pkgs.openssh
        ];
        environment.systemPackages = [ pkgs.curl ];
        givc.host = {
          enable = true;
          debug = true;
          network = {
            agent.transport = hostAddr;
            admin.transport = adminAddr;
            tls.enable = true;
          };
          accessControl.enable = false;
          capabilities.update.enable = true;
        };
      };

    netvm =
      { pkgs, config, ... }:
      let
        curl = "${pkgs.curl}/bin/curl";

        mockOtaUpdate = pkgs.writeShellScriptBin "ota-update" ''
          #!${pkgs.runtimeShell}
          set -eu

          printf '%s\n' "$*" >> /tmp/netvm-ota-update-calls

          case "$1" in
            registry)
              shift
              while [ $# -gt 0 ]; do
                case "$1" in
                  --output|--username|--password|--token)
                    shift 2
                    ;;
                  --insecure)
                    shift
                    ;;
                  *)
                    break
                    ;;
                esac
              done

              ${curl} -fsS http://test-updates.example.com/update/ghaf-dev >/dev/null

              case "$1" in
                discover)
                  newest=$(cat /tmp/netvm-ota-newest-version 2>/dev/null || echo mock-ghaf-version)
                  cat <<EOF
          {"event":"done"}
          [
            {
              "repository": "mock-repository",
              "tag": "ghaf-updates",
              "version": "0.9.0",
              "hash": "sha256:mock-older"
            },
            {
              "repository": "mock-repository",
              "tag": "ghaf-updates",
              "version": "$newest",
              "hash": "sha256:mock"
            }
          ]
          EOF
                  ;;
                changelog)
                  cat <<'EOF'
          {"event":"done"}
          Mock changelog
          EOF
                  ;;
                pull)
                  cat <<'EOF'
          {"event":"pull_started","reference":"mock-repository:ghaf-updates","destination":"/persist/sysupdate"}
          {"event":"blob_downloading","digest":"sha256:mock","downloaded":12,"total":34}
          {"event":"blob_verified","digest":"sha256:mock"}
          {"event":"manifest_written","path":"/persist/sysupdate/manifest.json"}
          {"event":"done"}
          pulled to: /persist/sysupdate
          manifest: /persist/sysupdate/manifest.json
          EOF
                  ;;
                *)
                  echo "unexpected registry subcommand: $*" >&2
                  exit 1
                  ;;
              esac
              ;;
            image)
              shift
              case "$1" in
                install)
                  exit 0
                  ;;
                *)
                  echo "unexpected image subcommand: $*" >&2
                  exit 1
                  ;;
              esac
              ;;
            *)
              echo "netvm ota-update only supports registry: $*" >&2
              exit 1
              ;;
          esac
        '';

        softwareUpdateSwitch = pkgs.writeShellScriptBin "switch-to-configuration" ''
          #!${pkgs.runtimeShell}
          case "$1" in
            boot)
              touch /tmp/switch-to-configuration-boot
            ;;
            *)
              echo "fail!"
              exit 1
            ;;
          esac
        '';

        nixosVersion = pkgs.writeShellScriptBin "nixos-version" ''
          echo "Fake version"
          cat <<EOF
          {"nixosVersion": "UPDATE"}
          EOF
        '';

        softwareUpdate = pkgs.symlinkJoin {
          name = "nixos-system-ghaf-host";
          paths = [ softwareUpdateSwitch ];
          postBuild = ''
            ln -s "${config.system.build.kernel}/${config.system.boot.loader.kernelFile}" $out/kernel
            ln -s ${config.system.modulesTree} $out/kernel-modules

            ${config.boot.bootspec.writer}

            ln -s ${nixosVersion} $out/sw
            mkdir -p $out/specialisation

            echo -n "${config.system.nixos.label}" >$out/nixos-label
            echo -n "${config.boot.kernelPackages.stdenv.hostPlatform.system}" > $out/system
          '';
        };

        findSoftwareUpdate = pkgs.writeShellScriptBin "find-software-update" ''
          echo ${softwareUpdate}
        '';
      in
      {
        imports = [
          sysvmModule
          otaUpdateServerModule
          (ghafGivcSrc + "/nixos/tests/snakeoil/gen-test-certs.nix")
        ];
        givc-tls-test = {
          name = netAddr.name;
          addresses = netAddr.addr;
        };
        nix.enable = true;
        virtualisation.writableStore = true;
        virtualisation.writableStoreUseTmpfs = true;
        boot.loader.systemd-boot.enable = true;
        users.mutableUsers = false;
        networking.interfaces.eth1.ipv4.addresses = lib.mkOverride 0 [
          {
            address = netAddr.addr;
            prefixLength = 24;
          }
        ];
        networking.extraHosts = ''
          192.168.101.200 test-updates.example.com
        '';
        services.nix-serve = {
          enable = true;
          secretKeyFile = updateServerKey;
        };
        services.ota-update-server = {
          enable = true;
          allowedProfiles = [ "ghaf-dev" ];
          publicKey = "test-updates.example.com:/muLakHVUJWxVRPIacpLJatGimj6S3OocBkwOan1VVc=%";
          cachix = "http://test-updates.example.com";
        };
        services.nginx = {
          enable = true;
          virtualHosts."test-updates.example.com" = {
            listen = [
              {
                addr = netAddr.addr;
                port = 80;
              }
            ];
            forceSSL = false;
            default = true;
            locations = {
              "/update" = {
                proxyPass = "http://127.0.0.1:${toString config.services.ota-update-server.port}";
              };
              "/api" = {
                proxyPass = "http://127.0.0.1:${toString config.services.ota-update-server.port}";
              };
              "/" = {
                proxyPass = "http://${config.services.nix-serve.bindAddress}:${toString config.services.nix-serve.port}";
              };
            };
          };
        };
        networking.firewall.allowedTCPPorts = [ 80 ];
        systemd.services.givc-admin.environment.GIVC_MONITORING = "false";
        environment.systemPackages = [ findSoftwareUpdate ];
        nixpkgs.overlays = lib.mkForce [
          (_final: _prev: {
            inherit system;
          })
          (_final: _prev: {
            ota-update = mockOtaUpdate;
          })
        ];
        systemd.services."givc-${netAddr.name}".path = lib.mkForce [
          config.system.path
          pkgs.dbus
          mockOtaUpdate
        ];
        givc.sysvm = {
          enable = true;
          network = {
            agent.transport = netAddr;
            admin.transport = adminAddr;
            tls.enable = true;
          };
          accessControl.enable = false;
          capabilities.update.enable = true;
        };
      };

    guivm =
      { pkgs, config, ... }:
      let
        mockLocalectl = pkgs.writeShellScriptBin "localectl" ''
          #!${pkgs.runtimeShell}
          printf 'localectl %s\n' "$*" >> /tmp/guivm-locale-calls
        '';

        mockTimedatectl = pkgs.writeShellScriptBin "timedatectl" ''
          #!${pkgs.runtimeShell}
          printf 'timedatectl %s\n' "$*" >> /tmp/guivm-locale-calls
        '';

        mockSystemctl = pkgs.writeShellScriptBin "systemctl" ''
          #!${pkgs.runtimeShell}
          printf 'systemctl %s\n' "$*" >> /tmp/guivm-locale-calls
        '';
      in
      {
        imports = [
          sysvmModule
          (ghafGivcSrc + "/nixos/tests/snakeoil/gen-test-certs.nix")
        ];
        virtualisation.memorySize = 1024;
        givc-tls-test = {
          name = guiAddr.name;
          addresses = guiAddr.addr;
        };
        nix.enable = true;
        networking.interfaces.eth1.ipv4.addresses = lib.mkOverride 0 [
          {
            address = guiAddr.addr;
            prefixLength = 24;
          }
        ];
        users.groups.ghaf = { };
        users.users.ghaf = {
          isNormalUser = true;
          group = "ghaf";
          extraGroups = [ "users" ];
          linger = true;
        };
        services.getty.autologinUser = "ghaf";
        programs.sway.enable = true;
        givc.sysvm.enableUserTlsAccess = true;
        environment = {
          systemPackages = [
            ctrlPanel
            pkgs.procps
            pkgs.glib
            pkgs.sway
            pkgs.netcat
            launchCtrlPanel
          ];
          variables = {
            CTRL_PANEL_CONFIG = "/etc/ctrl-panel/config.toml";
            CTRL_PANEL_AUTOMATION_SOCKET = "/tmp/ctrl-panel-automation.sock";
            GDK_BACKEND = "wayland";
            SWAYSOCK = "/tmp/sway-ipc.sock";
            WLR_RENDERER = "pixman";
          };
        };
        environment.etc."ctrl-panel/wireguard-gui-vms.txt".text = "";
        environment.etc."ctrl-panel/config.toml".text = ''
          [update]
          auth_mode = "anonymous"
          reference = "ghaf-updates"
          insecure = false
          username = ""
          password = ""
          oauth_token = ""
        '';
        programs.bash.loginShellInit = ''
          if [ "$(tty)" = "/dev/tty1" ]; then
            set -e
            mkdir -p ~/.config/sway
            cat > ~/.config/sway/config <<'EOF'
set $mod Mod1
default_border none
seat * xcursor_theme default 24
bindsym $mod+Shift+e exit
EOF
            sway --validate
            sway
          fi
        '';
        systemd.services."givc-${guiAddr.name}".path = lib.mkForce [
          mockLocalectl
          mockTimedatectl
          mockSystemctl
          config.system.path
        ];
        givc.sysvm = {
          enable = true;
          network = {
            agent.transport = guiAddr;
            admin.transport = adminAddr;
            tls.enable = true;
          };
          accessControl.enable = false;
        };
      };
  };

  testScript =
    { nodes, ... }:
    ''
      adminvm.wait_for_unit("multi-user.target")
      hostvm.wait_for_unit("multi-user.target")
      netvm.wait_for_unit("multi-user.target")
      guivm.wait_for_unit("multi-user.target")

      hostvm.succeed("nix-env -p /nix/var/nix/profiles/system --set ${nodes.hostvm.system.build.toplevel}")

      netvm.wait_for_unit("ota-update-server.service")
      update = netvm.succeed("find-software-update").strip()
      netvm.succeed("mkdir -p /nix/var/nix/profiles/per-user/updates")
      netvm.succeed(f"ota-update-server register /nix/var/nix/profiles/per-user/updates ghaf-dev {update}")

      hostvm.wait_for_unit("givc-ghaf-host.service")
      adminvm.wait_for_unit("givc-admin.service")
      guivm.wait_for_unit("givc-gui-vm.service")
      netvm.wait_for_unit("givc-net-vm.service")


      guivm.wait_for_file("/run/user/1000/wayland-1")
      guivm.wait_for_file("/tmp/sway-ipc.sock")
      guivm.succeed("env XDG_RUNTIME_DIR=/run/user/1000 SWAYSOCK=/tmp/sway-ipc.sock swaymsg exec ${launchCtrlPanel}/bin/launch-ctrl-panel")

      guivm.wait_until_succeeds("test -f /tmp/ctrl-panel-started")
      guivm.wait_for_file("/tmp/ctrl-panel-automation.sock")

      def expect_ui(command, expected):
          guivm.wait_until_succeeds(
              f"echo '{command}' | nc -NU /tmp/ctrl-panel-automation.sock | grep -qx '{expected}'"
          )

      guivm.succeed("echo 'click settings_view_button' | nc -NU /tmp/ctrl-panel-automation.sock")
      guivm.succeed("echo 'select list_box 1' | nc -NU /tmp/ctrl-panel-automation.sock")
      hostvm.wait_until_succeeds("grep -q 'ghaf-version' /tmp/host-sysinfo-calls")
      hostvm.wait_until_succeeds("grep -q 'bootctl status' /tmp/host-sysinfo-calls")
      hostvm.wait_until_succeeds("grep -q 'lsblk -rno TYPE' /tmp/host-sysinfo-calls")

      netvm.succeed("echo -n mock-ghaf-version > /tmp/netvm-ota-newest-version")
      guivm.succeed("echo 'click check_button' | nc -NU /tmp/ctrl-panel-automation.sock")
      netvm.wait_until_succeeds("test -f /tmp/netvm-ota-update-calls")
      netvm.wait_until_succeeds("grep -q '^registry --output jsonl discover ghaf-updates$' /tmp/netvm-ota-update-calls")
      expect_ui("get current_version label", "OK mock-ghaf-version")
      expect_ui("get available_version label", "OK Not checked")
      expect_ui("get no_updates_box visible", "OK TRUE")
      expect_ui("get update_details_box visible", "OK FALSE")
      expect_ui("get download_button visible", "OK FALSE")
      expect_ui("get check_button visible", "OK TRUE")
      expect_ui("get error_label visible", "OK FALSE")

      netvm.succeed("echo -n 2.0.0 > /tmp/netvm-ota-newest-version")
      guivm.succeed("echo 'click check_button' | nc -NU /tmp/ctrl-panel-automation.sock")
      netvm.wait_until_succeeds("[ $(grep -c '^registry --output jsonl discover ghaf-updates$' /tmp/netvm-ota-update-calls) -ge 2 ]")
      expect_ui("get current_version label", "OK mock-ghaf-version")
      expect_ui("get available_version label", "OK 2.0.0")
      expect_ui("get no_updates_box visible", "OK FALSE")
      expect_ui("get update_details_box visible", "OK TRUE")
      expect_ui("get download_button visible", "OK TRUE")
      expect_ui("get check_button visible", "OK FALSE")
      expect_ui("get error_label visible", "OK FALSE")

      guivm.succeed("echo 'click download_button' | nc -NU /tmp/ctrl-panel-automation.sock")
      netvm.wait_until_succeeds("grep -q '^registry --output jsonl pull mock-repository:ghaf-updates --destination /persist/sysupdate --validate$' /tmp/netvm-ota-update-calls")
      # Keep install automation available, but do not run it in the default
      # integration test for now: the install-finished dialog currently pushes
      # the GUI VM over its memory budget.
      # guivm.succeed("echo 'click update_button' | nc -NU /tmp/ctrl-panel-automation.sock")
      # netvm.wait_until_succeeds("grep -q '^image install --manifest /persist/sysupdate/manifest.json$' /tmp/netvm-ota-update-calls")
      guivm.succeed("echo 'set-locale-timezone en_US.utf8 Europe/Helsinki' | nc -NU /tmp/ctrl-panel-automation.sock")
      guivm.wait_until_succeeds("grep -q '^localectl set-locale LANG=en_US.utf8$' /tmp/guivm-locale-calls")
      guivm.wait_until_succeeds("grep -q '^timedatectl set-timezone Europe/Helsinki$' /tmp/guivm-locale-calls")
    '';
}
