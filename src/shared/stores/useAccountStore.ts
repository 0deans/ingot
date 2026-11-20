import { create } from "zustand"

export type AccountType = "Microsoft" | "Horizon Auth" | "Offline" | "Ely.by" | "Xbox Live"

export type AccountTypeLabelKey =
	| "settings.accounts.types.microsoft"
	| "settings.accounts.types.horizonAuth"
	| "settings.accounts.types.offline"
	| "settings.accounts.types.elyBy"
	| "settings.accounts.types.xboxLive"

export const ACCOUNT_TYPE_LABEL_KEYS: Record<AccountType, AccountTypeLabelKey> = {
	Microsoft: "settings.accounts.types.microsoft",
	"Horizon Auth": "settings.accounts.types.horizonAuth",
	Offline: "settings.accounts.types.offline",
	"Ely.by": "settings.accounts.types.elyBy",
	"Xbox Live": "settings.accounts.types.xboxLive",
}

export interface AccountProfile {
	id: string
	username: string
	type: AccountType
	badgeColor: string
	avatarBg: string
	skinModel: string
	uuid: string
}

const ACCOUNT_BADGE_COLORS: Record<AccountType, string> = {
	Microsoft:
		"border-emerald-600/25 bg-emerald-500/15 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/20 dark:text-emerald-300",
	"Horizon Auth":
		"border-rose-600/25 bg-rose-500/15 text-rose-700 dark:border-rose-500/30 dark:bg-rose-500/20 dark:text-rose-300",
	Offline:
		"border-amber-600/25 bg-amber-500/15 text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/20 dark:text-amber-300",
	"Ely.by":
		"border-sky-600/25 bg-sky-500/15 text-sky-700 dark:border-sky-500/30 dark:bg-sky-500/20 dark:text-sky-300",
	"Xbox Live":
		"border-cyan-600/25 bg-cyan-500/15 text-cyan-800 dark:border-cyan-500/30 dark:bg-cyan-500/20 dark:text-cyan-300",
}

const INITIAL_ACCOUNTS: AccountProfile[] = [
	{
		id: "acc-1",
		username: "Alex_Craft",
		type: "Microsoft",
		badgeColor: ACCOUNT_BADGE_COLORS.Microsoft,
		avatarBg: "from-emerald-400 to-teal-600",
		skinModel: "Slim (Alex)",
		uuid: "8667ba71-b85a-4004-af54-457a9734eed7",
	},
	{
		id: "acc-2",
		username: "Horizon_RustDev",
		type: "Horizon Auth",
		badgeColor: ACCOUNT_BADGE_COLORS["Horizon Auth"],
		avatarBg: "from-rose-400 to-orange-600",
		skinModel: "Slim (Alex)",
		uuid: "b912ca11-912a-4410-8b12-991827364511",
	},
	{
		id: "acc-3",
		username: "Notch_Legacy",
		type: "Offline",
		badgeColor: ACCOUNT_BADGE_COLORS.Offline,
		avatarBg: "from-amber-400 to-orange-600",
		skinModel: "Classic (Steve)",
		uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5",
	},
	{
		id: "acc-4",
		username: "Steve_ElyBy",
		type: "Ely.by",
		badgeColor: ACCOUNT_BADGE_COLORS["Ely.by"],
		avatarBg: "from-sky-400 to-indigo-600",
		skinModel: "Classic (Steve)",
		uuid: "f84c6a79-0a4e-45e0-879b-cd49ebd4c4e2",
	},
	{
		id: "acc-5",
		username: "Redstone_Builder",
		type: "Microsoft",
		badgeColor: ACCOUNT_BADGE_COLORS.Microsoft,
		avatarBg: "from-violet-400 to-purple-600",
		skinModel: "Slim (Alex)",
		uuid: "319c6a79-1a4e-45e0-879b-cd49ebd49912",
	},
	{
		id: "acc-6",
		username: "Bedrock_XBL_Pro",
		type: "Xbox Live",
		badgeColor: ACCOUNT_BADGE_COLORS["Xbox Live"],
		avatarBg: "from-cyan-400 to-blue-600",
		skinModel: "Bedrock Persona",
		uuid: "774c6a79-2b4e-45e0-879b-cd49ebd41188",
	},
	{
		id: "acc-7",
		username: "Speedrun_Bot_01",
		type: "Offline",
		badgeColor: ACCOUNT_BADGE_COLORS.Offline,
		avatarBg: "from-zinc-400 to-zinc-600",
		skinModel: "Classic (Steve)",
		uuid: "991c6a79-3c4e-45e0-879b-cd49ebd47721",
	},
	{
		id: "acc-8",
		username: "Kiev_SMP_Admin",
		type: "Microsoft",
		badgeColor: ACCOUNT_BADGE_COLORS.Microsoft,
		avatarBg: "from-yellow-400 to-blue-600",
		skinModel: "Classic (Steve)",
		uuid: "552c6a79-4d4e-45e0-879b-cd49ebd43309",
	},
	{
		id: "acc-9",
		username: "Shader_Tester",
		type: "Ely.by",
		badgeColor: ACCOUNT_BADGE_COLORS["Ely.by"],
		avatarBg: "from-fuchsia-400 to-pink-600",
		skinModel: "Slim (Alex)",
		uuid: "118c6a79-5e4e-45e0-879b-cd49ebd40094",
	},
]

export interface AccountState {
	accounts: AccountProfile[]
	activeAccountId: string
	setActiveAccountId: (id: string) => void
	addAccount: () => void
}

export const getAccountInitials = (username: string): string => {
	const clean = username.replace(/[^A-Za-z0-9]/g, "")
	return (clean.slice(0, 2) || "IN").toUpperCase()
}

export const findActiveAccount = (
	accounts: AccountProfile[],
	activeAccountId: string,
): AccountProfile => {
	const foundAccount = accounts.find((acc) => {
		return acc.id === activeAccountId
	})
	return foundAccount ?? accounts[0]
}

export const selectActiveAccount = (state: AccountState): AccountProfile => {
	return findActiveAccount(state.accounts, state.activeAccountId)
}

export const useAccountStore = create<AccountState>((set) => {
	return {
		accounts: INITIAL_ACCOUNTS,
		activeAccountId: "acc-1",
		setActiveAccountId: (id) => {
			set({ activeAccountId: id })
		},
		addAccount: () => {
			set((state) => {
				const id = `acc-${crypto.randomUUID()}`
				const suffix = Math.floor(100 + Math.random() * 899)
				const nextAccount: AccountProfile = {
					id,
					username: `Player_${suffix}`,
					type: "Microsoft",
					badgeColor: ACCOUNT_BADGE_COLORS.Microsoft,
					avatarBg: "from-teal-400 to-cyan-600",
					skinModel: "Slim (Alex)",
					uuid: crypto.randomUUID(),
				}
				return {
					accounts: [nextAccount, ...state.accounts],
					activeAccountId: id,
				}
			})
		},
	}
})
