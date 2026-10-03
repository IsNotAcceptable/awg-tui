# awg-tui

Terminal UI for [AmneziaWG](https://github.com/amnezia-vpn/amneziawg-linux-kernel-module)
running as a **kernel module**. It talks to the module through `awg` / `awg-quick`.

## Keys

| Key | Action |
|---|---|
| `↑↓` / `j k` | select tunnel |
| `Enter` / `Space` | bring tunnel up / down |
| `Tab` | switch Status / Config (keys are masked) |
| `m` | `modprobe amneziawg` |
| `r` | refresh |
| `q` / `Esc` | quit |

Run as root, or with passwordless `sudo -n` for `awg`, `awg-quick`, `modprobe`, `cat`.

## NixOS

```nix
# flake.nix
{
  inputs.awg-tui.url = "github:isnotacceptable/awg-tui";
  inputs.awg-tui.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { nixpkgs, awg-tui, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [
        awg-tui.nixosModules.default
        { programs.awg-tui.enable = true; }
      ];
    };
  };
}
```

Options: `programs.awg-tui.{enable,package,toolsPackage,configDir,kernelModule.enable,kernelModule.package}`.

Try without installing: `sudo nix run github:<you>/awg-tui`.

## Layout

```
src/main.rs        entrypoint, main loop
src/args.rs        CLI args
src/app.rs         state and actions
src/event.rs       keyboard input
src/kmod.rs        kernel module status / modprobe
src/config.rs      *.conf scanning, secret masking
src/fmt.rs         formatting helpers
src/awg/           awg / awg-quick wrapper, `dump` parser, models
src/ui/            ratatui widgets (header, tunnels, details, footer, theme)
nix/               package.nix, module.nix
```
