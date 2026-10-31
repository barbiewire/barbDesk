# barbDesk

So you grabbed one of my open source projects and none of the sounds play. That's because they're uploaded to my group, and Roblox won't let audio play in a game unless the owner allows it.

This fixes that. You run the exe with your place open in Studio and it:

- figures out which of my projects you have (it reads the `osName` attribute on ReplicatedStorage, so don't delete that)
- finds all my sounds that are still in your place
- downloads the original files into a `sfx` folder
- uploads them to your account or group
- swaps the old ids for your new ones everywhere (Sounds, AudioPlayers, scripts)

If you don't like the result, just Ctrl+Z in Studio. The whole swap is one undo.

## Before you run it

You need Studio open with your place, and Studio's MCP server turned on. That's in Assistant > Settings > MCP Servers > Enable Studio as MCP server. barbDesk talks to Studio through that, so there's no plugin to install.

You also need an Open Cloud API key, because it has to upload audio for you.

Making one:

1. Go to https://create.roblox.com/dashboard/credentials and hit Create API Key. If your game is owned by a group, switch to the group first (top left), otherwise the key can't upload for it.
2. Add the **Assets** API and give it Read and Write.
3. Add your IP to the allowed IPs (or 0.0.0.0/0 if you don't care).
4. Save it and copy the key.

barbDesk asks for the key when it needs it and doesn't save it anywhere. If you don't want to paste it every time, set it as `BARBDESK_KEY` in your environment variables.

## Running it

Grab `barbDesk.exe` from [Releases](../../releases), drop it in your project folder (the `sfx` folder ends up next to it) and open it.

It'll ask before it uploads anything and before it touches your place. It also remembers what it already uploaded (`sfx/<project>/uploaded.json`), so if something fails halfway you can just run it again.

Some options if you run it from a terminal:

- `--dry-run` scans and downloads but doesn't upload or change anything
- `--yes` skips the questions
- `--project <folder>` puts the sfx folder somewhere else

## Stuff to know

Uploads count toward your monthly audio limit on Roblox. If you hit it, wait for it to reset and run barbDesk again, it picks up where it stopped.

Upload as whoever owns the game. If you upload to your own account but the game belongs to a group, the sounds still won't play.

The sounds in here are only meant to be used with the projects they came with. Please don't reupload them for other stuff.

## Projects it works with right now

- Battlegrounds Framework V2 - Stage one [BAREBONES]

## For me (adding a project)

Set `osName` on ReplicatedStorage in the project, put the sounds in `projects/<folder>/sfx/`, write a `manifest.json` (osName, then id / name / file / sha256 for each sound) and add it to `projects/index.json`. The exe reads the list from this repo every time, so there's no need for a new release.

## Building

```
cargo build --release
```

If you're testing manifest changes, set `BARBDESK_SOURCE` to your local copy of the repo and it'll read from there instead of GitHub.
