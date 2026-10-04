import { useEffect } from "react"
import { create } from "zustand"
import type { AccountProfile, MicrosoftDeviceCode } from "@/bindings"
import { rpc } from "@/lib/rpc"

const useAccountStore = create<{ accounts: AccountProfile[]; fetched: boolean }>(() => ({
	accounts: [],
	fetched: false,
}))
const current = () => useAccountStore.getState().accounts
const skinCache = new Map<string, string>()

function notify(accounts: AccountProfile[]) {
	useAccountStore.setState({ accounts })
}

let refreshAccountsPromise: Promise<AccountProfile[]> | null = null

export const accountService = {
	getCachedAccounts(): AccountProfile[] {
		return current()
	},

	isFetched(): boolean {
		return useAccountStore.getState().fetched
	},

	async getAccounts(): Promise<AccountProfile[]> {
		return this.refreshAccounts()
	},

	async refreshAccounts(): Promise<AccountProfile[]> {
		if (refreshAccountsPromise) {
			return refreshAccountsPromise
		}

		refreshAccountsPromise = (async () => {
			try {
				const list = await rpc.get_accounts()
				useAccountStore.setState({ accounts: list, fetched: true })
				return list
			} catch (error) {
				console.error("Failed to load accounts:", error)
				return current()
			} finally {
				refreshAccountsPromise = null
			}
		})()

		return refreshAccountsPromise
	},

	async elyLogin(username: string, password: string): Promise<AccountProfile> {
		const account = await rpc.ely_login(username, password)
		const updated = [
			...current()
				.filter((a) => a.id !== account.id)
				.map((a) => ({ ...a, isActive: false })),
			account,
		]
		notify(updated)
		return account
	},

	/** Gets the code the user enters on microsoft.com/link */
	startMicrosoftLogin(): Promise<MicrosoftDeviceCode> {
		return rpc.microsoft_login_start()
	},

	/** Resolves once the user has entered the code and the account is saved */
	async finishMicrosoftLogin(code: MicrosoftDeviceCode): Promise<AccountProfile> {
		const account = await rpc.microsoft_login_finish(code)
		await this.refreshAccounts()
		return account
	},

	cancelMicrosoftLogin(): Promise<void> {
		return rpc.microsoft_login_cancel()
	},

	async addOfflineAccount(username: string): Promise<AccountProfile> {
		const account = await rpc.add_offline_account(username)
		const updated = [
			...current()
				.filter((a) => a.id !== account.id)
				.map((a) => ({ ...a, isActive: false })),
			account,
		]
		notify(updated)
		return account
	},

	async setActiveAccount(accountId: string): Promise<void> {
		await rpc.set_active_account(accountId)
		const updated = current().map((a) => ({
			...a,
			isActive: a.id === accountId,
		}))
		notify(updated)
	},

	async removeAccount(accountId: string): Promise<void> {
		await rpc.remove_account(accountId)
		const remaining = current().filter((a) => a.id !== accountId)
		if (remaining.length > 0 && !remaining.some((a) => a.isActive)) {
			remaining[0] = { ...remaining[0], isActive: true }
		}
		notify(remaining)
	},

	async getActiveToken(): Promise<string> {
		return await rpc.get_active_account_token()
	},

	getCachedSkinDataUrl(skinUrl?: string | null): string | null {
		if (!skinUrl) return null
		if (skinUrl.startsWith("data:")) return skinUrl
		return skinCache.get(skinUrl) ?? null
	},

	async getSkinDataUrl(skinUrl?: string | null): Promise<string | null> {
		if (!skinUrl) return null
		if (skinUrl.startsWith("data:")) return skinUrl
		if (skinCache.has(skinUrl)) {
			return skinCache.get(skinUrl) ?? null
		}
		try {
			const dataUrl = await rpc.get_skin_data_url(skinUrl)
			skinCache.set(skinUrl, dataUrl)
			return dataUrl
		} catch (error) {
			console.warn("Failed to fetch skin via IPC:", error)
			return null
		}
	},

	async saveSkinToDownloads(username: string, skinUrl: string): Promise<string> {
		return await rpc.save_skin_to_downloads(username, skinUrl)
	},

	async reorderAccounts(accountIds: string[]): Promise<void> {
		const reordered: AccountProfile[] = []
		for (const id of accountIds) {
			const found = current().find((a) => a.id === id)
			if (found) reordered.push(found)
		}
		for (const acc of current()) {
			if (!reordered.some((a) => a.id === acc.id)) {
				reordered.push(acc)
			}
		}
		notify(reordered)

		try {
			await rpc.reorder_accounts(accountIds)
		} catch (error) {
			console.error("Failed to persist account order:", error)
			await accountService.refreshAccounts()
		}
	},

	async getElySkins(
		page: number,
		query?: string,
		sort?: string,
		model?: string,
		uploader?: string,
	): Promise<import("@/types/skin").ElySkinsCatalogResponse> {
		return await rpc.get_ely_skins(
			page,
			query ?? null,
			sort ?? null,
			model ?? null,
			uploader ?? null,
		)
	},

	async getPlayerSkin(usernameOrUuid: string): Promise<import("@/types/skin").ElySkinItem | null> {
		const trimmed = usernameOrUuid.trim()
		if (!trimmed) return null
		try {
			const res = await fetch(
				`https://playerdb.co/api/player/minecraft/${encodeURIComponent(trimmed)}`,
			)
			if (!res.ok) return null
			const json = (await res.json()) as {
				success?: boolean
				data?: {
					player?: {
						username?: string
						id?: string
						raw_id?: string
						skin_texture?: string
						cape_texture?: string
						properties?: Array<{ name?: string; value?: string }>
					}
				}
			}
			const player = json.data?.player
			const skinTexture = player?.skin_texture
			if (!json.success || !player || !skinTexture) return null
			let isSlim = false
			let capeUrl = player.cape_texture || null
			try {
				const texturesProp = player.properties?.find((p) => p.name === "textures")
				if (texturesProp?.value) {
					const decoded = JSON.parse(atob(texturesProp.value))
					if (decoded?.textures?.SKIN?.metadata?.model === "slim") {
						isSlim = true
					}
					if (!capeUrl && decoded?.textures?.CAPE?.url) {
						capeUrl = decoded.textures.CAPE.url
					}
				}
			} catch {}

			let id = 1
			if (typeof player.raw_id === "string") {
				id =
					Math.abs(
						player.raw_id
							.slice(0, 8)
							.split("")
							.reduce((acc, char) => (acc * 31 + char.charCodeAt(0)) | 0, 0),
					) || 1
			}

			return {
				id,
				skinUrl: skinTexture,
				capeUrl,
				isSlim,
				countWearers: 1,
				countCubes: 0,
				countViews: 0,
				tags: [player.username || "Player", "Mojang", player.raw_id?.slice(0, 8) || ""].filter(
					Boolean,
				),
				name: player.username || "Player",
				provider: "player",
			}
		} catch (e) {
			console.warn("Failed to fetch player skin from PlayerDB:", e)
			return null
		}
	},

	async getMultiplePlayerSkins(usernames: string[]): Promise<import("@/types/skin").ElySkinItem[]> {
		const results = await Promise.allSettled(usernames.map((u) => this.getPlayerSkin(u)))
		return results
			.map((r) => {
				if (r.status === "fulfilled" && r.value) {
					return r.value
				}
				return null
			})
			.filter((s): s is import("@/types/skin").ElySkinItem => s !== null)
	},

	async getPlayerSkinsCatalog(
		query?: string,
		model?: string,
	): Promise<import("@/types/skin").ElySkinsCatalogResponse> {
		const clean = query?.trim()
		if (clean) {
			const single = await this.getPlayerSkin(clean)
			const items = single ? [single] : []
			return {
				items,
				currentPage: 1,
				lastPage: 1,
				totalItems: items.length,
			}
		}

		const items = await this.getMultiplePlayerSkins(FEATURED_PLAYERS)
		let filtered = items
		if (model === "slim") {
			filtered = filtered.filter((i) => i.isSlim)
		} else if (model === "steve") {
			filtered = filtered.filter((i) => !i.isSlim)
		}

		return {
			items: filtered,
			currentPage: 1,
			lastPage: 1,
			totalItems: filtered.length,
		}
	},

	async applyElySkin(accountId: string, skinId: number, password?: string): Promise<void> {
		await rpc.apply_ely_skin(accountId, skinId, password ?? null)
		await this.refreshAccounts()
	},

	async uploadElySkin(accountId: string, imageBase64: string, password?: string): Promise<void> {
		await rpc.upload_ely_skin(accountId, imageBase64, password ?? null)
		await this.refreshAccounts()
	},

	/** Sets a Microsoft account's skin to a public PNG, such as a catalog skin */
	async applyMicrosoftSkin(accountId: string, skinUrl: string, isSlim: boolean): Promise<void> {
		await rpc.apply_microsoft_skin(accountId, skinUrl, isSlim)
		await this.refreshAccounts()
	},

	/** Uploads a PNG data URL as a Microsoft account's skin */
	async uploadMicrosoftSkin(accountId: string, dataUrl: string, isSlim: boolean): Promise<void> {
		await rpc.upload_microsoft_skin(accountId, dataUrl, isSlim)
		await this.refreshAccounts()
	},

	async hasElyWebCredentials(accountId: string): Promise<boolean> {
		return await rpc.has_ely_web_credentials(accountId)
	},
}

export const FEATURED_PLAYERS = [
	"Steve",
	"Alex",
	"Technoblade",
	"Notch",
	"Jeb_",
	"MumboJumbo",
	"Grian",
	"DanTDM",
	"Dream",
	"GeorgeNotFound",
	"CaptainSparklez",
	"Stampylongnose",
	"TommyInnit",
	"Philza",
]

const UPLOADED_SKINS_STORAGE_KEY_PREFIX = "ingot_uploaded_skins_"

export const skinStorageService = {
	getUploadedSkins(accountId: string): import("@/types/skin").UserUploadedSkin[] {
		if (!accountId) return []
		try {
			const raw = localStorage.getItem(`${UPLOADED_SKINS_STORAGE_KEY_PREFIX}${accountId}`)
			if (!raw) return []
			return JSON.parse(raw) as import("@/types/skin").UserUploadedSkin[]
		} catch {
			return []
		}
	},

	saveUploadedSkin(accountId: string, skin: import("@/types/skin").UserUploadedSkin): void {
		if (!accountId) return
		try {
			const existing = this.getUploadedSkins(accountId)
			const filtered = existing.filter((s) => s.id !== skin.id && s.dataUrl !== skin.dataUrl)
			const updated = [skin, ...filtered]
			localStorage.setItem(
				`${UPLOADED_SKINS_STORAGE_KEY_PREFIX}${accountId}`,
				JSON.stringify(updated),
			)
		} catch (e) {
			console.error("Failed to save uploaded skin to local storage:", e)
		}
	},

	removeUploadedSkin(accountId: string, skinId: string): void {
		if (!accountId) return
		try {
			const existing = this.getUploadedSkins(accountId)
			const updated = existing.filter((s) => s.id !== skinId)
			localStorage.setItem(
				`${UPLOADED_SKINS_STORAGE_KEY_PREFIX}${accountId}`,
				JSON.stringify(updated),
			)
		} catch (e) {
			console.error("Failed to remove uploaded skin from local storage:", e)
		}
	},
}

export function useAccounts() {
	const accounts = useAccountStore((s) => s.accounts)

	useEffect(() => {
		if (!accountService.isFetched()) accountService.refreshAccounts()
	}, [])

	const activeAccount = accounts.find((a) => a.isActive) || accounts[0]

	return {
		accounts,
		activeAccount,
		refreshAccounts: accountService.refreshAccounts.bind(accountService),
		setActiveAccount: accountService.setActiveAccount.bind(accountService),
		removeAccount: accountService.removeAccount.bind(accountService),
		reorderAccounts: accountService.reorderAccounts.bind(accountService),
	}
}

export { rpc }
