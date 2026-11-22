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
| `b` | roll back the selected config to its `.conf.bak` (restarts the tunnel if up) |
| `g` | kill switch: drop all traffic except the selected tunnel and its endpoint (toggle, need nftables) |

Run it as `sudo awg-tui`, or as a user with passwordless `sudo -n` for the tools.

## Installation
awg-tui needs three things on the system: the AmneziaWG kernel module, the awg and awg-quick tools, and a few helper programs. The program checks for them at startup and can offer to install the module, but automatic installation is only planned for Debian. Everything else is manual.

## Manual installation 
awg-tui calls these through `PATH`:
| Program | Package | Used for |
| ------- | ------- | -------- |
| modprobe | kmod | loading kernel module |
| nft | nftables | kill switch |
| curl | curl | public ip check |
| ping | iputils | latency |
| journalctl | systemd | kernel log |

Tools for awg:
```bash
git clone https://github.com/amnezia-vpn/amneziawg-tools
cd amneziawg-tools/src
make
sudo make install
```
Dependencies: a C compiler and libc. Check with which awg awg-quick.

kernel module from [amneziawg-linux-kernel-module](https://github.com/amnezia-vpn/amneziawg-linux-kernel-module), or from your distribution's packages (see the sections below). Install kernel headers for the running kernel first, since the module is built against them. Check with:
```bash
modinfo amneziawg
sudo modprobe amneziawg && lsmod | grep amnezia
```

Build awg-tui as a normal user, not with sudo:
```bash
cargo build --release
```
Run:
```bash
sudo ./target/release/awg-tui
```

Tunnel configs live in /etc/amnezia/amneziawg/.

## Debian-based

```bash
# Install packages
sudo apt update
sudo apt install -y curl iputils-ping nftables kmod sudo git make gcc linux-headers-$(uname -r)
# The AmneziaWG PPA is built for Ubuntu focal, so on Debian it may not provide the package.
apt-cache policy amneziawg
# If Candidate is (none), build the module from source/
sudo apt install -y gnupg software-properties-common
sudo mkdir -p /etc/apt/keyrings
gpg --no-default-keyring --keyring /tmp/amnezia.kbx --keyserver keyserver.ubuntu.com --recv-keys 57290828
gpg --no-default-keyring --keyring /tmp/amnezia.kbx --export | sudo tee /etc/apt/keyrings/amnezia.gpg >/dev/null
rm -f /tmp/amnezia.kbx /tmp/amnezia.kbx~
echo "deb [signed-by=/etc/apt/keyrings/amnezia.gpg] https://ppa.launchpadcontent.net/amnezia/ppa/ubuntu focal main" | sudo tee /etc/apt/sources.list.d/amneziawg.list
sudo apt update
sudo apt install -y amneziawg
```


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
