# NixOS module. Imported from flake.nix as `nixosModules.default`.
self:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.awg-tui;
  inherit (lib) mkEnableOption mkIf mkOption types;
in
{
  options.programs.awg-tui = {
    enable = mkEnableOption "awg-tui, a terminal UI for AmneziaWG";

    package = mkOption {
      type = types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "inputs.awg-tui.packages.\${system}.default";
      description = "The awg-tui package to install.";
    };

    toolsPackage = mkOption {
      type = types.package;
      default = pkgs.amneziawg-tools;
      defaultText = lib.literalExpression "pkgs.amneziawg-tools";
      description = "Package providing `awg` and `awg-quick`.";
    };

    configDir = mkOption {
      type = types.path;
      default = "/etc/amnezia/amneziawg";
      description = "Directory with AmneziaWG `*.conf` tunnels.";
    };

    kernelModule = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Build and load the `amneziawg` kernel module.";
      };

      package = mkOption {
        type = types.package;
        default = config.boot.kernelPackages.amneziawg;
        defaultText = lib.literalExpression "config.boot.kernelPackages.amneziawg";
        description = "Out-of-tree kernel module package.";
      };
    };
  };

  config = mkIf cfg.enable {
    boot.extraModulePackages = mkIf cfg.kernelModule.enable [ cfg.kernelModule.package ];
    boot.kernelModules = mkIf cfg.kernelModule.enable [ "amneziawg" ];

    environment.systemPackages = [ cfg.package cfg.toolsPackage ];

    # Picked up by awg-tui when run as root (`sudo -E awg-tui`) or by the user.
    environment.sessionVariables.AWG_TUI_CONFIG_DIR = toString cfg.configDir;
  };
}
