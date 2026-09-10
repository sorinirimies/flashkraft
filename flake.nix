{
  description = "FlashKraft — Lightning-fast OS image writer (GUI + TUI)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        commonMeta = with pkgs.lib; {
          homepage = "https://github.com/sorinirimies/flashkraft";
          license = licenses.mit;
          maintainers = [ ];
        };

        linuxBuildInputs = with pkgs; [
          vulkan-loader
          wayland
          wayland-protocols
          libxkbcommon
          fontconfig
          freetype
          libGL
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          gtk3
        ];

        linuxRuntimeLibs = with pkgs; [
          vulkan-loader
          wayland
          libxkbcommon
          libGL
        ];

        darwinBuildInputs = with pkgs.darwin.apple_sdk.frameworks; [
          AppKit
          CoreFoundation
          CoreGraphics
          Metal
          QuartzCore
        ];

        flashkraft-gui = pkgs.rustPlatform.buildRustPackage {
          pname = "flashkraft";
          version = self.shortRev or self.dirtyShortRev or "dev";

          src = pkgs.lib.cleanSource ./.;

          cargoLock.lockFile = ./Cargo.lock;

          # Tests require a real filesystem (file explorer tests fail in sandbox)
          doCheck = false;

          nativeBuildInputs = with pkgs;
            [
              pkg-config
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              wrapGAppsHook3
            ];

          buildInputs =
            pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux linuxBuildInputs
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin darwinBuildInputs;

          cargoBuildFlags = [
            "-p"
            "flashkraft"
          ];
          cargoTestFlags = [
            "-p"
            "flashkraft"
          ];

          postFixup = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            wrapProgram "$out/bin/flashkraft" \
              --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath linuxRuntimeLibs}"
          '';

          meta = commonMeta // {
            description = "FlashKraft — OS image writer desktop application (Iced GUI)";
            mainProgram = "flashkraft";
            platforms = with pkgs.lib.platforms; linux ++ darwin;
          };
        };

        flashkraft-tui = pkgs.rustPlatform.buildRustPackage {
          pname = "flashkraft-tui";
          version = self.shortRev or self.dirtyShortRev or "dev";

          src = pkgs.lib.cleanSource ./.;

          cargoLock.lockFile = ./Cargo.lock;

          doCheck = false;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs =
            pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin darwinBuildInputs;

          cargoBuildFlags = [
            "-p"
            "flashkraft-tui"
          ];
          cargoTestFlags = [
            "-p"
            "flashkraft-tui"
          ];

          meta = commonMeta // {
            description = "FlashKraft — OS image writer terminal application (Ratatui TUI)";
            mainProgram = "flashkraft-tui";
            platforms = with pkgs.lib.platforms; linux ++ darwin;
          };
        };
      in
      {
        packages = {
          default = flashkraft-gui;
          inherit flashkraft-gui flashkraft-tui;
        };

        apps = {
          # `nix run` executes the plain, non-setuid store binary. Thanks to the
          # Linux transparent-escalation fallback in flashkraft-core, it still
          # works for raw-device access: FlashKraft re-execs itself through
          # sudo/pkexec on demand (prompting once per run) and drops back to
          # your UID immediately after. For a permanent, no-prompt-per-run
          # install use the NixOS module below or `just install-nix`.
          default = flake-utils.lib.mkApp {
            drv = flashkraft-gui;
            name = "flashkraft";
          };
          flashkraft-tui = flake-utils.lib.mkApp {
            drv = flashkraft-tui;
            name = "flashkraft-tui";
          };
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ flashkraft-gui ];

          packages = with pkgs; [
            rust-analyzer
            rustfmt
            clippy
          ];
        };
      }
    )
    // {
      # ── NixOS module ───────────────────────────────────────────────────────
      # FlashKraft's flash pipeline requires the installed binary to carry the
      # setuid-root bit (see README.md "How flashing works"). Binaries built
      # by Nix live in the immutable, typically nosuid-mounted /nix/store, so
      # they can never be setuid themselves. NixOS solves this exact class of
      # problem with `security.wrappers`: a small setuid-root wrapper is
      # installed at /run/wrappers/bin (a regular, suid-capable filesystem)
      # that execs the real, unprivileged store binary.
      #
      # Usage, in a NixOS configuration:
      #
      #   {
      #     imports = [ flashkraft.nixosModules.default ];
      #     programs.flashkraft.enable = true;
      #   }
      #
      # Then launch the wrapper (first on PATH on NixOS), not `nix run`:
      #   flashkraft
      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.flashkraft;
          system = pkgs.stdenv.hostPlatform.system;
          flashkraft-gui = self.packages.${system}.flashkraft-gui;
        in
        {
          options.programs.flashkraft.enable =
            lib.mkEnableOption "FlashKraft OS image writer (setuid-root wrapped)";

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ flashkraft-gui ];

            security.wrappers.flashkraft = {
              owner = "root";
              group = "root";
              setuid = true;
              source = "${flashkraft-gui}/bin/flashkraft";
            };
          };
        };
    };
}
