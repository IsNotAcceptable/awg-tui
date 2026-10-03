use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub name: String,
    pub path: PathBuf,
}

/// Lists `*.conf` files in `dir`. A missing directory is not an error.
pub fn scan(dir: &Path) -> Result<Vec<ConfigFile>> {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", dir.display())),
    };

    let mut list: Vec<ConfigFile> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "conf"))
        .filter_map(|p| {
            let name = p.file_stem()?.to_string_lossy().into_owned();
            Some(ConfigFile { name, path: p })
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

/// Hides secrets before showing a config on screen.
pub fn mask_secrets(text: &str) -> String {
    text.lines()
        .map(|l| {
            let key = l.split('=').next().unwrap_or("").trim();
            let lower = key.to_ascii_lowercase();
            if lower == "privatekey" || lower == "presharedkey" {
                format!("{key} = ********")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Replaces the first `Endpoint = …` line. Returns `None` if there is none.
pub fn replace_endpoint(text: &str, new: &str) -> Option<String> {
    let mut done = false;
    let lines: Vec<String> = text
        .lines()
        .map(|l| {
            let is_ep = !done
                && l.split_once('=')
                    .is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case("endpoint"));
            if is_ep {
                done = true;
                format!("Endpoint = {new}")
            } else {
                l.to_string()
            }
        })
        .collect();
    done.then(|| lines.join("\n") + "\n")
}

/// awg-quick uses the file stem as interface name: <= 15 chars of [A-Za-z0-9_=+.-].
pub fn valid_iface_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 15
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_=+.-".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_keys() {
        let t = "[Interface]\nPrivateKey = abc\nJc = 5\n[Peer]\nPresharedKey=zzz\nPublicKey = pub";
        let m = mask_secrets(t);
        assert!(!m.contains("abc") && !m.contains("zzz"));
        assert!(m.contains("Jc = 5") && m.contains("PublicKey = pub"));
    }

    #[test]
    fn replaces_endpoint() {
        let t = "[Peer]\nPublicKey = p\nEndpoint = 1.1.1.1:1\nAllowedIPs = 0.0.0.0/0\n";
        let r = replace_endpoint(t, "2.2.2.2:2").unwrap();
        assert!(r.contains("Endpoint = 2.2.2.2:2") && !r.contains("1.1.1.1"));
        assert!(replace_endpoint("[Peer]\n", "x").is_none());
    }

    #[test]
    fn iface_names() {
        assert!(valid_iface_name("WARPv3_29"));
        assert!(!valid_iface_name("has space"));
        assert!(!valid_iface_name("way-too-long-interface"));
    }
}
