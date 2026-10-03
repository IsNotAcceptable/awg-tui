{ lib, rustPlatform }:

let
  manifest = (lib.importTOML ../Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name;
  inherit (manifest) version;

  src = lib.cleanSourceWith {
    src = ../.;
    filter = path: type:
      let b = baseNameOf path; in
      !(b == "nix" || b == "flake.nix" || b == "flake.lock" || b == "target" || b == "result");
  };

  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    description = "Terminal UI for AmneziaWG (kernel module)";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "awg-tui";
  };
}
