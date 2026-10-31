# barbDesk

I made this for people who use my open source projects. The sounds in them are uploaded to my group, and Roblox only plays audio in games the owner allows, so in your copy a lot of them are silent.

barbDesk puts them on your account instead. Open your place in Studio and run the exe. It reads the `osName` attribute on ReplicatedStorage to know which project you have (so don't delete that), downloads my original sound files into a `sfx` folder and uploads them. Then it swaps every old id in your Sounds, AudioPlayers and scripts for the new ones. For the Battlegrounds framework that's 25 sounds.

If you don't like what it did, press Ctrl+Z in Studio and it's all undone. It's Windows only for now.

## Before you run it

You need Studio open with your place, and Studio's MCP server turned on. That's in Assistant > Settings > MCP Servers > Enable Studio as MCP server. barbDesk talks to Studio through that, so there's no plugin to install.

You also need an Open Cloud API key, since that's what it uploads with.

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

Uploads count toward your monthly audio limit on Roblox. I hit mine while testing this lol. If that happens just wait for it to reset and run barbDesk again, it picks up where it stopped.

Upload as whoever owns the game, otherwise the sounds still won't play (your own account won't work if the game belongs to a group).

The sounds in here are only meant for the projects they came with, so please don't reupload them for other stuff.

## Projects it works with right now

- Battlegrounds Framework V2 - Stage one [BAREBONES]

## For me (adding a project)

Set `osName` on ReplicatedStorage in the project, put the sounds in `projects/<folder>/sfx/`, write a `manifest.json` (osName, then id / name / file / sha256 for each sound) and add it to `projects/index.json`. The exe reads the list from this repo every time, so there's no need for a new release.

## Building

```
cargo build --release
```

If you're testing manifest changes, set `BARBDESK_SOURCE` to your local copy of the repo and it'll read from there instead of GitHub.
