import type { TFunction } from "i18next"
import type { TranslationKey } from "@/i18n"

/**
 * Friendly descriptions of common server.properties keys. The texts live in the locale
 * files under properties.* (groups, keys.<key>.label/description/options/placeholder,
 * units); localizedPropertyGroups fills them in.
 */

type SchemaControl =
	| { type: "boolean" }
	| { type: "number"; min?: number; max?: number; step?: number; unit?: string }
	| { type: "select"; options: { value: string }[] }
	| { type: "text"; placeholder?: boolean }

interface SchemaDef {
	key: string
	control: SchemaControl
	/** Used when the key isn't in the file yet */
	defaultValue: string
}

interface SchemaGroup {
	id: string
	properties: SchemaDef[]
}

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
	defaultValue: string
}

export interface PropertyGroup {
	id: string
	title: string
	properties: PropertyDef[]
}

const bool = { type: "boolean" } as const

const PROPERTY_GROUPS: SchemaGroup[] = [
	{
		id: "gameplay",
		properties: [
			{
				key: "gamemode",
				defaultValue: "survival",
				control: {
					type: "select",
					options: [
						{ value: "survival" },
						{ value: "creative" },
						{ value: "adventure" },
						{ value: "spectator" },
					],
				},
			},
			{
				key: "difficulty",
				defaultValue: "easy",
				control: {
					type: "select",
					options: [
						{ value: "peaceful" },
						{ value: "easy" },
						{ value: "normal" },
						{ value: "hard" },
					],
				},
			},
			{
				key: "force-gamemode",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "hardcore",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "pvp",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "allow-flight",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "spawn-protection",
				defaultValue: "16",
				control: { type: "number", min: 0, max: 256, unit: "blocks" },
			},
		],
	},
	{
		id: "players",
		properties: [
			{
				key: "max-players",
				defaultValue: "20",
				control: { type: "number", min: 1, max: 1000 },
			},
			{
				key: "online-mode",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "player-idle-timeout",
				defaultValue: "0",
				control: { type: "number", min: 0, max: 1440, unit: "min" },
			},
			{
				key: "op-permission-level",
				defaultValue: "4",
				control: { type: "number", min: 1, max: 4 },
			},
			{
				key: "hide-online-players",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "enforce-secure-profile",
				defaultValue: "true",
				control: bool,
			},
		],
	},
	{
		id: "world",
		properties: [
			{
				key: "view-distance",
				defaultValue: "10",
				control: { type: "number", min: 2, max: 32, unit: "chunks" },
			},
			{
				key: "simulation-distance",
				defaultValue: "10",
				control: { type: "number", min: 2, max: 32, unit: "chunks" },
			},
			{
				key: "level-seed",
				defaultValue: "",
				control: { type: "text", placeholder: true },
			},
			{
				key: "level-type",
				defaultValue: "minecraft:normal",
				control: {
					type: "select",
					options: [
						{ value: "minecraft:normal" },
						{ value: "minecraft:flat" },
						{ value: "minecraft:large_biomes" },
						{ value: "minecraft:amplified" },
					],
				},
			},
			{
				key: "generate-structures",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "allow-nether",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "max-world-size",
				defaultValue: "29999984",
				control: { type: "number", min: 1, max: 29999984, unit: "blocks" },
			},
		],
	},
	{
		id: "network",
		properties: [
			{
				key: "server-port",
				defaultValue: "25565",
				control: { type: "number", min: 1024, max: 65535 },
			},
			{
				key: "enable-status",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "network-compression-threshold",
				defaultValue: "256",
				control: { type: "number", min: -1, max: 65536, unit: "bytes" },
			},
		],
	},
	{
		id: "advanced",
		properties: [
			{
				key: "enable-command-block",
				defaultValue: "false",
				control: bool,
			},
			{
				key: "sync-chunk-writes",
				defaultValue: "true",
				control: bool,
			},
			{
				key: "max-tick-time",
				defaultValue: "60000",
				control: { type: "number", min: -1, unit: "ms" },
			},
			{
				key: "entity-broadcast-range-percentage",
				defaultValue: "100",
				control: { type: "number", min: 10, max: 1000, unit: "%" },
			},
		],
	},
]

/** Keys edited elsewhere in the UI */
export const HANDLED_ELSEWHERE = new Set(["motd", "white-list", "enforce-whitelist", "server-ip"])

export const KNOWN_KEYS = new Set(PROPERTY_GROUPS.flatMap((g) => g.properties.map((p) => p.key)))

const key = (k: string) => k as TranslationKey
/** Locale keys can't contain ":" (namespace) or "." (nesting) */
const optionKey = (v: string) => v.replace(/[:.]/g, "_")
const unitKey = (u: string) => (u === "%" ? "percent" : u)

/** The schema with its texts in the user's language */
export function localizedPropertyGroups(t: TFunction): PropertyGroup[] {
	return PROPERTY_GROUPS.map((g) => ({
		id: g.id,
		title: t(key(`properties.groups.${g.id}`)),
		properties: g.properties.map((p): PropertyDef => {
			const base = `properties.keys.${p.key}`
			const c = p.control
			const control: PropertyControl =
				c.type === "select"
					? {
							...c,
							options: c.options.map((o) => ({
								value: o.value,
								label: t(key(`${base}.options.${optionKey(o.value)}`)),
							})),
						}
					: c.type === "text"
						? {
								type: "text",
								placeholder: c.placeholder ? t(key(`${base}.placeholder`)) : undefined,
							}
						: c.type === "number" && c.unit
							? { ...c, unit: t(key(`properties.units.${unitKey(c.unit)}`)) }
							: c
			return {
				key: p.key,
				defaultValue: p.defaultValue,
				label: t(key(`${base}.label`)),
				description: t(key(`${base}.description`)),
				control,
			}
		}),
	}))
}
