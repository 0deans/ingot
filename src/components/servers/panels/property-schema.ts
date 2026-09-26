/** Friendly descriptions of common server.properties keys */

export type PropertyControl =
	| { type: "boolean" }
	| { type: "number"; min?: number; max?: number; step?: number; unit?: string }
	| { type: "select"; options: { value: string; label: string }[] }
	| { type: "text"; placeholder?: string }

export interface PropertyDef {
	key: string
	label: string
	description: string
	control: PropertyControl
	/** Used when the key isn't in the file yet */
	defaultValue: string
}

export interface PropertyGroup {
	id: string
	title: string
	properties: PropertyDef[]
}

const bool = { type: "boolean" } as const

export const PROPERTY_GROUPS: PropertyGroup[] = [
	{
		id: "gameplay",
		title: "Gameplay",
		properties: [
			{
				key: "gamemode",
				label: "Game mode",
				description: "Mode for new players.",
				defaultValue: "survival",
				control: {
					type: "select",
					options: [
						{ value: "survival", label: "Survival" },
						{ value: "creative", label: "Creative" },
						{ value: "adventure", label: "Adventure" },
						{ value: "spectator", label: "Spectator" },
					],
				},
			},
			{
				key: "difficulty",
				label: "Difficulty",
				description: "How dangerous mobs are.",
				defaultValue: "easy",
				control: {
					type: "select",
					options: [
						{ value: "peaceful", label: "Peaceful" },
						{ value: "easy", label: "Easy" },
						{ value: "normal", label: "Normal" },
						{ value: "hard", label: "Hard" },
					],
				},
			},
			{
				key: "force-gamemode",
				label: "Force game mode",
				description: "Put players back in the default mode every time they join.",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "hardcore",
				label: "Hardcore",
				description: "Players are switched to spectator when they die.",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "pvp",
				label: "PvP",
				description: "Players can damage each other.",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "allow-flight",
				label: "Allow flight",
				description: "Don't kick players for flying (needed for some mods and plugins).",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "spawn-protection",
				label: "Spawn protection",
				description: "Radius around spawn that only operators can build in. 0 turns it off.",
				defaultValue: "16",
				control: { type: "number", min: 0, max: 256, unit: "blocks" },
			},
		],
	},
	{
		id: "players",
		title: "Players",
		properties: [
			{
				key: "max-players",
				label: "Max players",
				description: "How many players can be online at once.",
				defaultValue: "20",
				control: { type: "number", min: 1, max: 1000 },
			},
			{
				key: "online-mode",
				label: "Online mode",
				description:
					"Check accounts with Mojang. Turn off to allow non-premium or Ely.by accounts.",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "player-idle-timeout",
				label: "Idle kick",
				description: "Kick players after this many idle minutes. 0 never kicks.",
				defaultValue: "0",
				control: { type: "number", min: 0, max: 1440, unit: "min" },
			},
			{
				key: "op-permission-level",
				label: "Operator level",
				description: "What operators may do: 1 bypass spawn protection … 4 everything.",
				defaultValue: "4",
				control: { type: "number", min: 1, max: 4 },
			},
			{
				key: "hide-online-players",
				label: "Hide player list",
				description: "Don't show who is online in the multiplayer menu.",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "enforce-secure-profile",
				label: "Require signed chat",
				description: "Only allow players with Mojang-signed chat keys.",
				defaultValue: "true",
				control: bool,
			},
		],
	},
	{
		id: "world",
		title: "World",
		properties: [
			{
				key: "view-distance",
				label: "View distance",
				description: "How far players can see. Lower is lighter on the device.",
				defaultValue: "10",
				control: { type: "number", min: 2, max: 32, unit: "chunks" },
			},
			{
				key: "simulation-distance",
				label: "Simulation distance",
				description: "How far around players the world keeps ticking.",
				defaultValue: "10",
				control: { type: "number", min: 2, max: 32, unit: "chunks" },
			},
			{
				key: "level-seed",
				label: "Seed",
				description: "Seed for a new world. Only used when the world is first created.",
				defaultValue: "",
				control: { type: "text", placeholder: "Random" },
			},
			{
				key: "level-type",
				label: "World type",
				description: "Only used when the world is first created.",
				defaultValue: "minecraft:normal",
				control: {
					type: "select",
					options: [
						{ value: "minecraft:normal", label: "Default" },
						{ value: "minecraft:flat", label: "Superflat" },
						{ value: "minecraft:large_biomes", label: "Large biomes" },
						{ value: "minecraft:amplified", label: "Amplified" },
					],
				},
			},
			{
				key: "generate-structures",
				label: "Generate structures",
				description: "Villages, temples and other structures in new chunks.",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "allow-nether",
				label: "Allow Nether",
				description: "Let players travel to the Nether.",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "max-world-size",
				label: "World border radius",
				description: "Maximum distance from the center players can go.",
				defaultValue: "29999984",
				control: { type: "number", min: 1, max: 29999984, unit: "blocks" },
			},
		],
	},
	{
		id: "network",
		title: "Network",
		properties: [
			{
				key: "server-port",
				label: "Port",
				description: "Port the server listens on.",
				defaultValue: "25565",
				control: { type: "number", min: 1024, max: 65535 },
			},
			{
				key: "enable-status",
				label: "Show in server list",
				description: "Answer pings from the multiplayer menu (MOTD, player count).",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "network-compression-threshold",
				label: "Compression threshold",
				description: "Compress packets bigger than this. -1 disables compression.",
				defaultValue: "256",
				control: { type: "number", min: -1, max: 65536, unit: "bytes" },
			},
		],
	},
	{
		id: "advanced",
		title: "Advanced",
		properties: [
			{
				key: "enable-command-block",
				label: "Command blocks",
				description: "Allow command blocks to run.",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "sync-chunk-writes",
				label: "Safe chunk saving",
				description: "Write chunks synchronously. Safer, slightly slower.",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "max-tick-time",
				label: "Watchdog timeout",
				description: "Stop the server if one tick takes longer than this. -1 disables.",
				defaultValue: "60000",
				control: { type: "number", min: -1, unit: "ms" },
			},
			{
				key: "entity-broadcast-range-percentage",
				label: "Entity render distance",
				description: "How far entities are sent to players, in percent.",
				defaultValue: "100",
				control: { type: "number", min: 10, max: 1000, unit: "%" },
			},
		],
	},
]

/** Keys edited elsewhere in the UI */
export const HANDLED_ELSEWHERE = new Set(["motd", "white-list", "enforce-whitelist"])

export const KNOWN_KEYS = new Set(PROPERTY_GROUPS.flatMap((g) => g.properties.map((p) => p.key)))
