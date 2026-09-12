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
