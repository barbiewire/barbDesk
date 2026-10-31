pub const SCAN: &str = r#"
local HS = game:GetService("HttpService")
local RS = game:GetService("ReplicatedStorage")

local Found = {}
local Seen = 0

local function Note(Id, Where)
	local Entry = Found[Id]
	if not Entry then
		Entry = {}
		Found[Id] = Entry
	end
	if #Entry < 4 then
		table.insert(Entry, Where)
	end
	Seen += 1
end

local function Grab(Text, Where)
	for Id in string.gmatch(Text, "rbxassetid://(%d+)") do
		Note(Id, Where)
	end
	for Id in string.gmatch(Text, "roblox%.com/asset/%?id=(%d+)") do
		Note(Id, Where)
	end
end

for _, Thing in game:GetDescendants() do
	pcall(function()
		if Thing:IsA("Sound") then
			Grab(Thing.SoundId, Thing:GetFullName())
		elseif Thing:IsA("AudioPlayer") then
			Grab(Thing.Asset, Thing:GetFullName())
		elseif Thing:IsA("LuaSourceContainer") then
			Grab(Thing.Source, Thing:GetFullName())
		end
	end)
end

return HS:JSONEncode({
	osName = RS:GetAttribute("osName"),
	placeId = game.PlaceId,
	creatorId = game.CreatorId,
	creatorType = game.CreatorType.Name,
	refs = Seen,
	ids = Found,
})
"#;

pub const SWAP: &str = r#"
local HS = game:GetService("HttpService")
local CHS = game:GetService("ChangeHistoryService")

local Map = HS:JSONDecode([==[__MAP__]==])
local Done = { sounds = 0, players = 0, scripts = 0, swaps = 0 }

local function Swap(Text)
	local Hits = 0
	Text = string.gsub(Text, "rbxassetid://(%d+)", function(Id)
		local New = Map[Id]
		if New then
			Hits += 1
			return "rbxassetid://" .. New
		end
		return nil
	end)
	Text = string.gsub(Text, "(roblox%.com/asset/%?id=)(%d+)", function(Pre, Id)
		local New = Map[Id]
		if New then
			Hits += 1
			return Pre .. New
		end
		return nil
	end)
	return Text, Hits
end

local Recording = CHS:TryBeginRecording("barbDesk audio swap")

for _, Thing in game:GetDescendants() do
	pcall(function()
		if Thing:IsA("Sound") then
			local New, Hits = Swap(Thing.SoundId)
			if Hits > 0 then
				Thing.SoundId = New
				Done.sounds += 1
				Done.swaps += Hits
			end
		elseif Thing:IsA("AudioPlayer") then
			local New, Hits = Swap(Thing.Asset)
			if Hits > 0 then
				Thing.Asset = New
				Done.players += 1
				Done.swaps += Hits
			end
		elseif Thing:IsA("LuaSourceContainer") then
			local New, Hits = Swap(Thing.Source)
			if Hits > 0 then
				Thing.Source = New
				Done.scripts += 1
				Done.swaps += Hits
			end
		end
	end)
end

if Recording then
	CHS:FinishRecording(Recording, Enum.FinishRecordingOperation.Commit)
end

return HS:JSONEncode(Done)
"#;
