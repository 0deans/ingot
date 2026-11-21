import { type AccountProfile as BackendAccountProfile } from "@/bindings"

export type AccountType = "Microsoft" | "Offline" | "Ely.by"

export type AccountTypeLabelKey =
	| "settings.accounts.types.microsoft"
	| "settings.accounts.types.offline"
	| "settings.accounts.types.elyBy"

export const ACCOUNT_TYPE_LABEL_KEYS: Record<AccountType, AccountTypeLabelKey> = {
	Microsoft: "settings.accounts.types.microsoft",
	Offline: "settings.accounts.types.offline",
	"Ely.by": "settings.accounts.types.elyBy",
}

export interface UiAccountProfile {
	id: string
	username: string
	type: AccountType
	badgeColor: string
	avatarBg: string
	uuid: string
	skinUrl: string | null
	isActive: boolean
	createdAt: number
}

const ACCOUNT_BADGE_COLORS: Record<AccountType, string> = {
	Microsoft:
		"border-emerald-600/25 bg-emerald-500/15 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/20 dark:text-emerald-300",
	Offline:
		"border-amber-600/25 bg-amber-500/15 text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/20 dark:text-amber-300",
	"Ely.by":
		"border-sky-600/25 bg-sky-500/15 text-sky-700 dark:border-sky-500/30 dark:bg-sky-500/20 dark:text-sky-300",
}

const AVATAR_GRADIENTS: readonly string[] = [
	"from-emerald-400 to-teal-600",
	"from-violet-400 to-purple-600",
	"from-sky-400 to-indigo-600",
	"from-amber-400 to-orange-600",
	"from-rose-400 to-pink-600",
	"from-cyan-400 to-blue-600",
	"from-fuchsia-400 to-purple-600",
]

export const normalizeAccountType = (rawType: string): AccountType => {
	const normalized = rawType.trim().toLowerCase()
	if (normalized === "microsoft" || normalized === "msa") {
		return "Microsoft"
	}
	if (normalized === "ely" || normalized === "ely.by" || normalized === "elyby") {
		return "Ely.by"
	}
	return "Offline"
}

const pickAvatarGradient = (seed: string): string => {
	let hash = 0
	for (let i = 0; i < seed.length; i += 1) {
		hash = (hash * 31 + seed.charCodeAt(i)) >>> 0
	}
	const index = hash % AVATAR_GRADIENTS.length
	return AVATAR_GRADIENTS[index] ?? AVATAR_GRADIENTS[0]
}

export const mapBackendAccount = (profile: BackendAccountProfile): UiAccountProfile => {
	const type = normalizeAccountType(profile.accountType)

	return {
		id: profile.id,
		username: profile.username,
		type,
		badgeColor: ACCOUNT_BADGE_COLORS[type],
		avatarBg: pickAvatarGradient(profile.id || profile.uuid || profile.username),
		uuid: profile.uuid,
		skinUrl: profile.skinUrl,
		isActive: profile.isActive,
		createdAt: profile.createdAt,
	}
}

export const findActiveAccount = (
	accounts: readonly UiAccountProfile[],
): UiAccountProfile | null => {
	const active = accounts.find((account) => {
		return account.isActive
	})
	if (active) {
		return active
	}
	return accounts[0] ?? null
}

export const getAccountInitials = (username: string): string => {
	const cleaned = username.trim()
	if (!cleaned) {
		return "MC"
	}

	const parts = cleaned.split(/[_\-\s]+/).filter(Boolean)
	if (parts.length >= 2) {
		const first = parts[0]?.[0] ?? ""
		const second = parts[1]?.[0] ?? ""
		const combined = `${first}${second}`.toUpperCase()
		if (combined.length > 0) {
			return combined
		}
	}

	return cleaned.slice(0, 2).toUpperCase()
}
