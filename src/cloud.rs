use serde_json::{json, Value};
use std::time::Duration;

const ASSETS: &str = "https://apis.roblox.com/assets/v1";

#[derive(Clone, Debug, PartialEq)]
pub enum Creator {
    User(String),
    Group(String),
}

impl Creator {
    pub fn describe(&self) -> String {
        match self {
            Creator::User(id) => format!("user {id}"),
            Creator::Group(id) => format!("group {id}"),
        }
    }

    pub fn tag(&self) -> String {
        match self {
            Creator::User(id) => format!("user:{id}"),
            Creator::Group(id) => format!("group:{id}"),
        }
    }

    fn json(&self) -> Value {
        match self {
            Creator::User(id) => json!({ "userId": id }),
            Creator::Group(id) => json!({ "groupId": id }),
        }
    }
}

fn read_error(err: ureq::Error) -> (u16, String) {
    match err {
        ureq::Error::Status(code, resp) => (code, resp.into_string().unwrap_or_default()),
        other => (0, other.to_string()),
    }
}

fn json_of(resp: ureq::Response) -> Result<Value, String> {
    let text = resp.into_string().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn explain(code: u16, body: &str) -> String {
    let hint = match code {
        401 => " (the API key is wrong or expired)",
        403 => " (the key is missing the asset:read / asset:write scopes, or isn't allowed for this creator)",
        429 => " (Roblox rate limit, try again in a minute)",
        _ => "",
    };
    format!("HTTP {code}{hint}: {}", body.chars().take(300).collect::<String>())
}

pub fn upload(key: &str, creator: &Creator, name: &str, file: &[u8]) -> Result<String, String> {
    let request = json!({
        "assetType": "Audio",
        "displayName": name.chars().take(50).collect::<String>(),
        "description": "Uploaded by barbDesk",
        "creationContext": { "creator": creator.json() },
    });
    let boundary = format!(
        "barbDesk{:x}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
    );
    let mut body = Vec::with_capacity(file.len() + 1024);
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"request\"\r\nContent-Type: application/json\r\n\r\n{request}\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"fileContent\"; filename=\"{name}.ogg\"\r\nContent-Type: audio/ogg\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let mut tries = 0;
    let op: Value = loop {
        tries += 1;
        let sent = ureq::post(&format!("{ASSETS}/assets"))
            .set("x-api-key", key)
            .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
            .send_bytes(&body);
        match sent {
            Ok(resp) => break json_of(resp).map_err(|e| format!("odd upload reply: {e}"))?,
            Err(err) => {
                let (code, text) = read_error(err);
                if (code == 429 || code >= 500) && tries < 5 {
                    std::thread::sleep(Duration::from_secs(5 * tries));
                    continue;
                }
                return Err(explain(code, &text));
            }
        }
    };
    if let Some(id) = op["response"]["assetId"].as_str() {
        return Ok(id.to_string());
    }
    let op_id = op["operationId"]
        .as_str()
        .map(str::to_string)
        .or_else(|| op["path"].as_str().and_then(|p| p.rsplit('/').next()).map(str::to_string))
        .ok_or_else(|| format!("upload reply had no operation: {op}"))?;
    for _ in 0..90 {
        std::thread::sleep(Duration::from_secs(1));
        let polled = ureq::get(&format!("{ASSETS}/operations/{op_id}")).set("x-api-key", key).call();
        let state: Value = match polled {
            Ok(resp) => json_of(resp).map_err(|e| format!("odd operation reply: {e}"))?,
            Err(err) => {
                let (code, text) = read_error(err);
                if code == 429 || code >= 500 {
                    continue;
                }
                return Err(explain(code, &text));
            }
        };
        if state["done"].as_bool() == Some(true) {
            if let Some(err) = state.get("error") {
                return Err(format!("Roblox rejected it: {err}"));
            }
            if let Some(id) = state["response"]["assetId"].as_str() {
                return Ok(id.to_string());
            }
            return Err(format!("upload finished without an asset id: {state}"));
        }
    }
    Err("Roblox took too long to process the upload".into())
}
