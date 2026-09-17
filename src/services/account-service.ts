import { useEffect, useState } from "react"
import { type AccountProfile, createTauRPCProxy } from "@/bindings"

export const rpc = createTauRPCProxy()

let cachedAccounts: AccountProfile[] = []
let isInitialFetched = false
const listeners = new Set<(accounts: AccountProfile[]) => void>()
const skinCache = new Map<string, string>()

function notify(accounts: AccountProfile[]) {
	cachedAccounts = accounts
	for (const listener of listeners) {
		listener(accounts)
	}
}

export const accountService = {
	getCachedAccounts(): AccountProfile[] {
		return cachedAccounts
	},

	isFetched(): boolean {
		return isInitialFetched
	},

	subscribe(listener: (accounts: AccountProfile[]) => void): () => void {
		listeners.add(listener)
		listener(cachedAccounts)
		return () => {
			listeners.delete(listener)
		}
	},

	async getAccounts(): Promise<AccountProfile[]> {
		return this.refreshAccounts()
	},

	async refreshAccounts(): Promise<AccountProfile[]> {
		try {
			const list = await rpc.get_accounts()
			isInitialFetched = true
			notify(list)
			return list
		} catch (error) {
			console.error("Failed to load accounts:", error)
			return cachedAccounts
		}
	},

	async elyLogin(username: string, password: string): Promise<AccountProfile> {
		const account = await rpc.ely_login(username, password)
		const updated = [
			...cachedAccounts.filter((a) => a.id !== account.id).map((a) => ({ ...a, isActive: false })),
			account,
		]
		notify(updated)
		return account
	},

	async addOfflineAccount(username: string): Promise<AccountProfile> {
		const account = await rpc.add_offline_account(username)
		const updated = [
			...cachedAccounts.filter((a) => a.id !== account.id).map((a) => ({ ...a, isActive: false })),
			account,
		]
		notify(updated)
		return account
	},

	async setActiveAccount(accountId: string): Promise<void> {
		await rpc.set_active_account(accountId)
		const updated = cachedAccounts.map((a) => ({
			...a,
			isActive: a.id === accountId,
		}))
		notify(updated)
	},

	async removeAccount(accountId: string): Promise<void> {
		await rpc.remove_account(accountId)
		const remaining = cachedAccounts.filter((a) => a.id !== accountId)
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
			const found = cachedAccounts.find((a) => a.id === id)
			if (found) reordered.push(found)
		}
		for (const acc of cachedAccounts) {
			if (!reordered.some((a) => a.id === acc.id)) {
				reordered.push(acc)
			}
		}
		cachedAccounts = reordered
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

	async applyElySkin(accountId: string, skinId: number, password?: string): Promise<void> {
		await rpc.apply_ely_skin(accountId, skinId, password ?? null)
		await this.refreshAccounts()
	},

	async uploadElySkin(accountId: string, imageBase64: string, password?: string): Promise<void> {
		await rpc.upload_ely_skin(accountId, imageBase64, password ?? null)
		await this.refreshAccounts()
	},

	async hasElyWebCredentials(accountId: string): Promise<boolean> {
		return await rpc.has_ely_web_credentials(accountId)
	},
}

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
	const [accounts, setAccounts] = useState<AccountProfile[]>(accountService.getCachedAccounts())

	useEffect(() => {
		const unsubscribe = accountService.subscribe((updated) => {
			setAccounts(updated)
		})

		if (!accountService.isFetched()) {
			accountService.refreshAccounts()
		}

		return unsubscribe
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
