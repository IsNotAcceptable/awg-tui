# awg-tui

Terminal UI for [AmneziaWG](https://github.com/amnezia-vpn/amneziawg-linux-kernel-module)
running as a **kernel module**. It talks to the module through `awg` / `awg-quick`.

## Usage 
### Keys

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
| `q` / `Esc` | quit |
| `b` | roll back the selected config to its `.conf.bak` (restarts the tunnel if up) |
| `g` | kill switch: drop all traffic except the selected tunnel and its endpoint (toggle, need nftables) |

Run it as `sudo awg-tui`, or as a user with passwordless `sudo -n` for the tools.

### Command-line options

- `-m`, `--multi`: do not bring other tunnels down when starting one
- `-a`, `--auto-rescan`: rescan endpoints automatically when a handshake goes stale

## Installation
awg-tui needs three things on the system: the AmneziaWG kernel module, the awg and awg-quick tools, and a few helper programs. The program checks for them at startup and can offer to install the module, but automatic installation is only planned for Debian. Everything else is manual.

## Requirements

- Linux with root access (via `sudo`)
- AmneziaWG kernel module (`amneziawg`)
- AmneziaWG tools `awg` and `awg-quick`
- Helper programs: `kmod`, `nftables`, `curl`, `iputils`
- Rust toolchain to build

## Manual installation 
Tools for awg:
```bash
git clone https://github.com/amnezia-vpn/amneziawg-tools
cd amneziawg-tools/src
make
sudo make install
```
Dependencies: a C compiler and libc. Check with `which awg awg-quick`.

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

If the module is missing at startup, awg-tui offers to install it. See the Debian section for what that step does.

### Debian-based

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

After follow manual with installation tools

The module must match the running kernel. Build it with DKMS from the upstream [amneziawg-linux-kernel-module](https://github.com/amnezia-vpn/amneziawg-linux-kernel-module) repository, then verify with `sudo modinfo amneziawg`. Automatic module build is not implemented yet, so the install step in awg-tui only installs the dependencies above.

If Secure Boot is on, the DKMS module must be signed with a key the system trusts, or `modprobe` will refuse to load it.

If your tunnel config has a `DNS` line, `awg-quick` runs `resolvconf` to apply it. On Debian 13 it is not installed by default, and without it `awg-quick up` rolls the interface back. Either install a resolver integration:

```bash
sudo apt install -y resolvconf    # or openresolv
```

or remove the `DNS` line from the config.

### Fedora / RHEL

Not yet verified. Build the module from source with `kernel-devel` for the running kernel.

### Arch Linux

Not yet verified. The AUR has `amneziawg-dkms` and `amneziawg-tools`; install them with an AUR helper.

### NixOS

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

## Troubleshooting

- **`failed to execute awg`** - the tools are not installed or not in `PATH`. See manual with tools.
- **`modprobe: Module amneziawg not found`** - the module is not built for the running kernel. See manual with mod kernel.
- **`cannot start editor`** - `$EDITOR` is unset or points to a program that `sudo` cannot find. Run with `sudo -E` or pass the full path, for example `sudo EDITOR="$(which nvim)" ./target/release/awg-tui`.
- **`awg-quick up` rolls back after `resolvconf: command not found`** - see the DNS note in the Debian section.
- **Kill switch cut off your SSH session** - turn it off from a local console, or run it only when traffic goes through the tunnel. If awg-tui exits while the kill switch is on, remove the rule manually with `sudo nft delete table inet awg_tui_ks`.
