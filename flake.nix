{
  description = "awg-tui — terminal UI for AmneziaWG (kernel module)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        awg-tui = pkgs.callPackage ./nix/package.nix { };
        default = awg-tui;
      });

      apps = forAllSystems (pkgs: rec {
        awg-tui = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.awg-tui}/bin/awg-tui";
        };
        default = awg-tui;
      });

      overlays.default = final: _prev: {
        awg-tui = final.callPackage ./nix/package.nix { };
      };

      nixosModules = {
        awg-tui = import ./nix/module.nix self;
        default = self.nixosModules.awg-tui;
      };

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc clippy rustfmt rust-analyzer amneziawg-tools ];
        };
      });
    };
}
