# awg-tui

Terminal UI for [AmneziaWG](https://github.com/amnezia-vpn/amneziawg-linux-kernel-module)
running as a **kernel module**. It talks to the module through `awg` / `awg-quick`.

## Keys

| Key | Action |
|---|---|
| `↑↓` / `j k` | select tunnel |
| `Enter` / `Space` | bring tunnel up / down (others are stopped unless `--multi`) |
| `s` | scan WARP endpoints on the running tunnel (`Enter` applies + saves, `x` stops) |
| `t` | exit check via Cloudflare trace (IP, country, `warp=on`) |
| `e` | edit config in `$EDITOR` (`sudoedit` when not root) |
| `i` / `d` | import a `.conf` / delete the selected one |
| `Tab` | Status → Config (keys masked) → Kernel log |
| `m` | `modprobe amneziawg` |
| `r` | refresh |
| `q` / `Esc` | quit |

Run it as `sudo awg-tui`, or as a user with passwordless `sudo -n` for the tools.

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

Try without installing: `sudo nix run github:isnotacceptable/awg-tui`.

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
src/jobs/          background jobs: endpoint scan, exit check
src/editor.rs      suspend TUI and run $EDITOR
src/ui/            ratatui widgets (header, tunnels, details, footer, theme)
nix/               package.nix, module.nix
```
