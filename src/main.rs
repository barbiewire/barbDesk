mod cloud;
mod luau;
mod repo;
mod studio;

use cloud::Creator;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

struct Args {
    dry: bool,
    yes: bool,
    project: Option<PathBuf>,
}

fn args() -> Args {
    let mut out = Args { dry: false, yes: false, project: None };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--dry-run" => out.dry = true,
            "--yes" | "-y" => out.yes = true,
            "--project" => out.project = it.next().map(PathBuf::from),
            "--help" | "-h" => {
                println!("barbDesk [--dry-run] [--yes] [--project <folder>]");
                println!("  --dry-run   scan and download only, upload and change nothing");
                println!("  --yes       don't ask before uploading or swapping");
                println!("  --project   where the sfx folder goes (default: the current folder)");
                std::process::exit(0);
            }
            other => eprintln!("ignoring unknown option {other}"),
        }
    }
    out
}

fn ask(question: &str) -> String {
    print!("{question}");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    line.trim().to_string()
}

fn yes(question: &str, assume: bool) -> bool {
    if assume {
        return true;
    }
    let answer = ask(&format!("{question} [Y/n] ")).to_lowercase();
    answer.is_empty() || answer.starts_with('y')
}

fn safe(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    cleaned.trim().trim_end_matches('.').to_string()
}

fn pause() {
    if std::env::args().any(|a| a == "--yes" || a == "-y") {
        return;
    }
    let _ = ask("\nPress Enter to close.");
}

type Cache = BTreeMap<String, BTreeMap<String, String>>;

fn load_cache(path: &Path) -> Cache {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_cache(path: &Path, cache: &Cache) {
    if let Ok(text) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(path, text);
    }
}

fn pick_creator(scan: &Value, assume: bool) -> Result<Creator, String> {
    let owner = scan["creatorId"].as_u64().unwrap_or(0).to_string();
    let default = match scan["creatorType"].as_str() {
        Some("Group") => Creator::Group(owner),
        _ => Creator::User(owner),
    };
    println!("\nThe new audio has to be owned by whoever owns your experience, or it won't play.");
    println!("This place belongs to {}.", default.describe());
    if assume {
        return Ok(default);
    }
    loop {
        let answer = ask("Upload as that creator? Press Enter, or type \"user <id>\" / \"group <id>\": ");
        if answer.is_empty() {
            return Ok(default);
        }
        let mut parts = answer.split_whitespace();
        let kind = parts.next().unwrap_or("").to_lowercase();
        let id = parts.next().unwrap_or("");
        if !id.chars().all(|c| c.is_ascii_digit()) || id.is_empty() {
            println!("That needs a number, like: group 1234567");
            continue;
        }
        match kind.as_str() {
            "user" => return Ok(Creator::User(id.to_string())),
            "group" => return Ok(Creator::Group(id.to_string())),
            _ => println!("Start it with user or group."),
        }
    }
}

fn api_key() -> Result<String, String> {
    if let Ok(key) = std::env::var("BARBDESK_KEY") {
        if !key.trim().is_empty() {
            return Ok(key.trim().to_string());
        }
    }
    println!("\nPaste your Open Cloud API key (it needs asset:read and asset:write). It won't show while you type.");
    let key = rpassword::prompt_password("API key: ").map_err(|e| format!("couldn't read the key: {e}"))?;
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("no API key given".into());
    }
    Ok(key)
}

fn run(opts: &Args) -> Result<(), String> {
    println!("barbDesk {}", env!("CARGO_PKG_VERSION"));
    println!("Connecting to Roblox Studio...");
    let mut studio = studio::Studio::connect()?;
    let list = studio.instances()?;
    let chosen = match list.len() {
        0 => {
            return Err("no Studio found. Open your place in Studio and turn on the MCP server \
                        (Assistant > Settings > MCP Servers > Enable Studio as MCP server)."
                .into())
        }
        1 => &list[0],
        _ => {
            println!("More than one Studio is open:");
            for (i, s) in list.iter().enumerate() {
                println!("  {}. {}", i + 1, s.name);
            }
            loop {
                let n: usize = ask("Which one? ").parse().unwrap_or(0);
                if (1..=list.len()).contains(&n) {
                    break &list[n - 1];
                }
            }
        }
    };
    println!("Using {}", chosen.name);

    println!("Scanning the place for audio...");
    let scan: Value =
        serde_json::from_str(&studio.run(&chosen.id, luau::SCAN)?).map_err(|e| format!("couldn't read the scan: {e}"))?;
    let Some(os_name) = scan["osName"].as_str().filter(|s| !s.is_empty()).map(str::to_string) else {
        return Err("this place has no osName attribute on ReplicatedStorage, so it isn't one of barbiewire's \
                    open-source projects (or the attribute was removed)."
            .into());
    };
    println!("Project: {os_name}");
    let in_place: Map<String, Value> = scan["ids"].as_object().cloned().unwrap_or_default();

    let source = repo::Repo::new();
    let Some((base, manifest)) = source.manifest(&os_name)? else {
        return Err(format!("barbDesk doesn't have audio for \"{os_name}\" yet."));
    };
    let wanted: Vec<repo::Sound> = manifest.sounds.iter().filter(|s| in_place.contains_key(&s.id)).cloned().collect();
    println!(
        "{} of this project's {} sounds are still in the place.",
        wanted.len(),
        manifest.sounds.len()
    );
    if wanted.is_empty() {
        println!("Nothing to swap. They've already been replaced, or the place doesn't use them.");
        return Ok(());
    }

    let root = match &opts.project {
        Some(dir) => dir.clone(),
        None => std::env::current_dir().map_err(|e| e.to_string())?,
    };
    let folder = root.join("sfx").join(safe(&manifest.os_name));
    std::fs::create_dir_all(&folder).map_err(|e| format!("couldn't make {}: {e}", folder.display()))?;
    println!("Downloading into {}", folder.display());
    let mut files = Vec::new();
    for sound in &wanted {
        let path = source.download(&base, sound, &folder)?;
        println!("  got {}", sound.name);
        files.push((sound, path));
    }

    if opts.dry {
        println!("\nDry run, stopping here. These would be uploaded and swapped:");
        for (sound, _) in &files {
            let spots = in_place[&sound.id].as_array().map(|a| a.len()).unwrap_or(0);
            println!("  {} ({}) used in {}+ spot(s)", sound.name, sound.id, spots);
        }
        return Ok(());
    }

    let creator = pick_creator(&scan, opts.yes)?;
    let cache_path = folder.join("uploaded.json");
    let mut cache = load_cache(&cache_path);
    let mut map = BTreeMap::new();
    let todo: Vec<_> = files.iter().filter(|(s, _)| !cache.get(&creator.tag()).is_some_and(|m| m.contains_key(&s.id))).collect();
    if !todo.is_empty() {
        if !yes(&format!("\nUpload {} sound(s) as {}?", todo.len(), creator.describe()), opts.yes) {
            return Err("stopped before uploading".into());
        }
        let key = api_key()?;
        for (sound, path) in &todo {
            let bytes = std::fs::read(path).map_err(|e| format!("couldn't read {}: {e}", path.display()))?;
            print!("  uploading {}... ", sound.name);
            let _ = io::stdout().flush();
            match cloud::upload(&key, &creator, &sound.name, &bytes) {
                Ok(new) => {
                    println!("{new}");
                    cache.entry(creator.tag()).or_default().insert(sound.id.clone(), new);
                    save_cache(&cache_path, &cache);
                }
                Err(e) => {
                    println!("failed");
                    eprintln!("    {e}");
                }
            }
        }
    }
    if let Some(done) = cache.get(&creator.tag()) {
        for (sound, _) in &files {
            if let Some(new) = done.get(&sound.id) {
                map.insert(sound.id.clone(), new.clone());
            }
        }
    }
    if map.is_empty() {
        return Err("nothing was uploaded, so nothing was swapped".into());
    }

    if !yes(&format!("\nSwap {} audio id(s) in {}?", map.len(), chosen.name), opts.yes) {
        println!("Left the place alone. Run barbDesk again to swap; uploads are remembered in {}", cache_path.display());
        return Ok(());
    }
    let map_json = serde_json::to_string(&map).map_err(|e| e.to_string())?;
    let result = studio.run(&chosen.id, &luau::SWAP.replace("__MAP__", &map_json))?;
    let done: Value = serde_json::from_str(&result).unwrap_or(Value::Null);
    println!(
        "Swapped {} reference(s): {} Sound(s), {} AudioPlayer(s), {} script(s).",
        done["swaps"], done["sounds"], done["players"], done["scripts"]
    );
    let missed = files.len() - map.len();
    if missed > 0 {
        println!("{missed} sound(s) failed to upload and kept their old ids. Run barbDesk again to retry them.");
    }
    println!("Ctrl+Z in Studio undoes the swap. Save or publish the place to keep it.");
    Ok(())
}

fn main() {
    let opts = args();
    if let Err(e) = run(&opts) {
        eprintln!("\nbarbDesk stopped: {e}");
        pause();
        std::process::exit(1);
    }
    pause();
}
