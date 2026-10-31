use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

pub struct Studio {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next: u64,
}

pub struct Instance {
    pub id: String,
    pub name: String,
}

fn find_exe() -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let versions = PathBuf::from(local).join("Roblox").join("Versions");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(dirs) = std::fs::read_dir(&versions) {
        for dir in dirs.flatten() {
            let exe = dir.path().join("StudioMCP.exe");
            if let Ok(meta) = std::fs::metadata(&exe) {
                let when = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                if best.as_ref().map_or(true, |(t, _)| when > *t) {
                    best = Some((when, exe));
                }
            }
        }
    }
    if let Some((_, exe)) = best {
        return Some(exe);
    }
    let out = Command::new("reg")
        .args(["query", r"HKCU\Software\Roblox\RobloxStudio", "/v", "ContentFolder"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains("ContentFolder"))?;
    let folder = line.split("REG_SZ").nth(1)?.trim();
    let exe = PathBuf::from(folder).parent()?.join("StudioMCP.exe");
    exe.exists().then_some(exe)
}

impl Studio {
    pub fn connect() -> Result<Studio, String> {
        let exe = find_exe().ok_or("couldn't find StudioMCP.exe, is Roblox Studio installed and up to date?")?;
        let mut child = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("couldn't start {}: {e}", exe.display()))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut studio = Studio { child, stdin, lines: rx, next: 1 };
        studio.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "barbDesk", "version": env!("CARGO_PKG_VERSION") }
            }),
            Duration::from_secs(20),
        )?;
        studio.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))?;
        Ok(studio)
    }

    fn send(&mut self, msg: Value) -> Result<(), String> {
        let mut line = msg.to_string();
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.flush())
            .map_err(|e| format!("lost the Studio connection: {e}"))
    }

    fn request(&mut self, method: &str, params: Value, wait: Duration) -> Result<Value, String> {
        let id = self.next;
        self.next += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        let deadline = Instant::now() + wait;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(format!("Studio didn't answer {method} in time"));
            }
            let line = match self.lines.recv_timeout(left) {
                Ok(line) => line,
                Err(_) => continue,
            };
            let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
            if msg.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(err) = msg.get("error") {
                return Err(format!("Studio error: {err}"));
            }
            return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    fn tool(&mut self, name: &str, args: Value, wait: Duration) -> Result<String, String> {
        let result = self.request("tools/call", json!({ "name": name, "arguments": args }), wait)?;
        let text = result["content"]
            .as_array()
            .map(|parts| parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();
        if result["isError"].as_bool() == Some(true) {
            return Err(text);
        }
        Ok(text)
    }

    pub fn instances(&mut self) -> Result<Vec<Instance>, String> {
        let text = self.tool("list_roblox_studios", json!({}), Duration::from_secs(20))?;
        let parsed: Value = serde_json::from_str(&text).map_err(|_| format!("odd studio list: {text}"))?;
        Ok(parsed["studios"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|s| Instance {
                        id: s["id"].as_str().unwrap_or_default().to_string(),
                        name: s["name"].as_str().unwrap_or("(unnamed)").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn run(&mut self, studio: &str, code: &str) -> Result<String, String> {
        self.tool(
            "execute_luau",
            json!({ "studio_id": studio, "datamodel_type": "Edit", "code": code }),
            Duration::from_secs(120),
        )
    }
}

impl Drop for Studio {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
