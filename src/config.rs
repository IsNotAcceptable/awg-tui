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
}
