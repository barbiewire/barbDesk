# barbDesk

If you got one of my open-source Roblox projects, its sounds probably won't play for you: they're uploaded to my account, and Roblox only lets audio play in experiences its owner allows.

barbDesk fixes that in one go. It:

1. finds your open Roblox Studio,
2. works out which of my projects the place is (from the `osName` attribute on `ReplicatedStorage`),
3. finds every one of my sounds the place still uses,
4. downloads the original audio files into `sfx/<project name>/` next to where you ran it,
5. uploads them to **your** account or group with your Open Cloud API key,
6. swaps the old IDs for your new ones in Sounds, AudioPlayers and scripts.

The swap is one undo step, so Ctrl+Z in Studio reverts it.

## What you need

- Windows and an up-to-date Roblox Studio with your place open.
- Studio's MCP server turned on: **Assistant → Settings → MCP Servers → Enable Studio as MCP server**. barbDesk uses Studio's own `StudioMCP.exe` to read and edit the place. It doesn't need a plugin.
- An **Open Cloud API key** that can upload audio for the account or group that owns your experience.

### Making the API key

1. Go to [create.roblox.com/dashboard/credentials](https://create.roblox.com/dashboard/credentials) and click **Create API Key**.
   - If your experience belongs to a group, switch to that group first (top left), so the key can upload for the group.
2. Under **Access Permissions**, add the **Assets** API with **Read** and **Write**.
3. Under **Security**, add your IP address (or `0.0.0.0/0`) to the allowed IPs.
4. Save, then copy the key.

barbDesk asks for the key when it needs it and never saves it. To skip the prompt, put it in the `BARBDESK_KEY` environment variable instead.

## Using it

1. Download `barbDesk.exe` from [Releases](../../releases).
2. Put it in the folder you keep your project files in (the `sfx` folder goes there).
3. Open your place in Studio, then run `barbDesk.exe`.

It asks before uploading and before changing the place. Sounds it already uploaded are remembered in `sfx/<project>/uploaded.json`, so running it again won't upload them twice.

Options:

| Option | What it does |
|---|---|
| `--dry-run` | Scan and download only. Nothing is uploaded or changed. |
| `--yes` | Don't ask, just do it (uploads as the experience's owner). |
| `--project <folder>` | Put the `sfx` folder somewhere else. |

## Notes

- Uploaded audio counts toward your account's monthly audio upload limit.
- Upload as the creator that **owns the experience**. Audio owned by someone else won't play in it unless you share it.
- The sounds in this repo are only for use with the projects they ship with.

## Supported projects

| Project (`osName`) | Sounds |
|---|---|
| Battlegrounds Framework V2 - Stage one [BAREBONES] | 25 |

## Adding a project (maintainer)

1. Set `ReplicatedStorage` attribute `osName` in the project's place.
2. Add `projects/<folder>/sfx/*.ogg` and `projects/<folder>/manifest.json` (`osName`, and per sound: `id`, `name`, `file`, `sha256`).
3. Add an entry to `projects/index.json`.

No new release is needed: the exe reads the index from this repo every run.

## Building

```
cargo build --release
```

The exe lands in `target/release/barbDesk.exe`. Set `BARBDESK_SOURCE` to a local copy of this repo to test manifest changes before pushing.
