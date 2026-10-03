{ lib, rustPlatform, makeWrapper, curl, iputils }:

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

  nativeBuildInputs = [ makeWrapper ];

  # curl (exit-IP check) and ping (endpoint scan). Appended to PATH so that the
  # setcap `ping` from /run/wrappers/bin keeps priority on NixOS.
  postInstall = ''
    wrapProgram $out/bin/awg-tui --suffix PATH : ${lib.makeBinPath [ curl iputils ]}
  '';

  meta = {
    description = "Terminal UI for AmneziaWG (kernel module)";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "awg-tui";
  };
}
