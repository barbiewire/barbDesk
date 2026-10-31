use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

const DEFAULT_SOURCE: &str = "https://raw.githubusercontent.com/barbiewire/barbDesk/main";

#[derive(Deserialize)]
pub struct Entry {
    #[serde(rename = "osName")]
    pub os_name: String,
    pub path: String,
}

#[derive(Deserialize)]
pub struct Manifest {
    #[serde(rename = "osName")]
    pub os_name: String,
    pub sounds: Vec<Sound>,
}

#[derive(Deserialize, Clone)]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub file: String,
    pub sha256: String,
}

pub struct Repo {
    source: String,
}

impl Repo {
    pub fn new() -> Repo {
        let source = std::env::var("BARBDESK_SOURCE").unwrap_or_else(|_| DEFAULT_SOURCE.to_string());
        Repo { source: source.trim_end_matches(['/', '\\']).to_string() }
    }

    fn fetch(&self, rel: &str) -> Result<Vec<u8>, String> {
        if self.source.starts_with("http") {
            let url = format!("{}/{}", self.source, rel.split('/').map(encode).collect::<Vec<_>>().join("/"));
            let resp = ureq::get(&url).call().map_err(|e| format!("couldn't download {rel}: {e}"))?;
            let mut out = Vec::new();
            resp.into_reader()
                .take(50 * 1024 * 1024)
                .read_to_end(&mut out)
                .map_err(|e| format!("download of {rel} broke: {e}"))?;
            Ok(out)
        } else {
            std::fs::read(Path::new(&self.source).join(rel)).map_err(|e| format!("couldn't read {rel}: {e}"))
        }
    }

    pub fn manifest(&self, os_name: &str) -> Result<Option<(String, Manifest)>, String> {
        let index: Vec<Entry> =
            serde_json::from_slice(&self.fetch("projects/index.json")?).map_err(|e| format!("bad project index: {e}"))?;
        let Some(entry) = index.into_iter().find(|e| e.os_name == os_name) else { return Ok(None) };
        let manifest: Manifest = serde_json::from_slice(&self.fetch(&format!("{}/manifest.json", entry.path))?)
            .map_err(|e| format!("bad manifest for {os_name}: {e}"))?;
        Ok(Some((entry.path, manifest)))
    }

    pub fn download(&self, base: &str, sound: &Sound, into: &Path) -> Result<PathBuf, String> {
        let target = into.join(file_name(&sound.file));
        if let Ok(have) = std::fs::read(&target) {
            if hex(&Sha256::digest(&have)) == sound.sha256 {
                return Ok(target);
            }
        }
        let bytes = self.fetch(&format!("{base}/{}", sound.file))?;
        let got = hex(&Sha256::digest(&bytes));
        if got != sound.sha256 {
            return Err(format!("{} came down damaged (checksum mismatch)", sound.name));
        }
        std::fs::write(&target, &bytes).map_err(|e| format!("couldn't save {}: {e}", target.display()))?;
        Ok(target)
    }
}

fn file_name(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

fn encode(part: &str) -> String {
    part.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
