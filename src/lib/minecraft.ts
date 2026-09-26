/** Helpers for showing Minecraft data (items, players, dimensions) in the UI */

const ICON_VERSION = "1.21.8"

/** "minecraft:diamond_sword" -> "diamond_sword" */
export function stripNamespace(id: string): string {
	const idx = id.indexOf(":")
	return idx >= 0 ? id.slice(idx + 1) : id
}

/** "minecraft:diamond_sword" -> "Diamond Sword" */
export function prettyId(id: string): string {
	return stripNamespace(id)
		.split("_")
		.filter(Boolean)
		.map((w) => w[0].toUpperCase() + w.slice(1))
		.join(" ")
}

/**
 * Icon URLs to try in order: rendered inventory icon (items and isometric blocks),
 * then the flat item texture, then the block texture.
 */
export function itemIconUrls(id: string): string[] {
	const name = stripNamespace(id)
	return [
		`https://mc.nerothe.com/img/${ICON_VERSION}/minecraft_${name}.png`,
		`https://assets.mcasset.cloud/${ICON_VERSION}/assets/minecraft/textures/item/${name}.png`,
		`https://assets.mcasset.cloud/${ICON_VERSION}/assets/minecraft/textures/block/${name}.png`,
	]
}

export function avatarUrl(name: string, size = 64): string {
	return `https://mc-heads.net/avatar/${encodeURIComponent(name)}/${size}`
}

export function bodyUrl(name: string, size = 180): string {
	return `https://mc-heads.net/body/${encodeURIComponent(name)}/${size}`
}

export interface DimensionStyle {
	label: string
	/** Tailwind classes for a small badge */
	badge: string
	/** Map background behind missing chunks */
	void: string
}

export function dimensionStyle(id: string): DimensionStyle {
	switch (id) {
		case "minecraft:overworld":
			return {
				label: "Overworld",
				badge: "border-emerald-500/30 bg-emerald-500/10 text-emerald-300",
				void: "#0b0f0c",
			}
		case "minecraft:the_nether":
			return {
				label: "Nether",
				badge: "border-rose-500/30 bg-rose-500/10 text-rose-300",
				void: "#140807",
			}
		case "minecraft:the_end":
			return {
				label: "The End",
				badge: "border-violet-500/30 bg-violet-500/10 text-violet-300",
				void: "#0c0a14",
			}
		default:
			return {
				label: prettyId(id),
				badge: "border-sky-500/30 bg-sky-500/10 text-sky-300",
				void: "#0a0c10",
			}
	}
}

export const GAMEMODES = ["survival", "creative", "adventure", "spectator"] as const
export type Gamemode = (typeof GAMEMODES)[number]

export function romanNumeral(n: number): string {
	return ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"][n - 1] ?? String(n)
}

/** Effect duration in ticks -> "1:30" / "∞" */
export function formatTicks(ticks: number): string {
	if (ticks < 0) return "∞"
	const total = Math.floor(ticks / 20)
	return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`
}

export function formatRelativeTime(ms: number | null | undefined): string {
	if (!ms) return "Unknown"
	const diff = Date.now() - ms
	const minutes = Math.floor(diff / 60_000)
	if (minutes < 1) return "Just now"
	if (minutes < 60) return `${minutes}m ago`
	const hours = Math.floor(minutes / 60)
	if (hours < 24) return `${hours}h ago`
	const days = Math.floor(hours / 24)
	if (days < 30) return `${days}d ago`
	return new Date(ms).toLocaleDateString()
}

export function formatUptime(seconds: number): string {
	const h = Math.floor(seconds / 3600)
	const m = Math.floor((seconds % 3600) / 60)
	if (h > 0) return `${h}h ${m}m`
	if (m > 0) return `${m}m`
	return `${Math.max(0, Math.floor(seconds))}s`
}

/** Minecraft usernames (offline servers allow up to 32 chars) */
export function isValidPlayerName(name: string): boolean {
	return /^[A-Za-z0-9_]{1,32}$/.test(name)
}
