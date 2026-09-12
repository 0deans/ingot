import { useEffect, useState } from "react"
import { type AccountProfile, createTauRPCProxy } from "@/bindings"

export const rpc = createTauRPCProxy()

let cachedAccounts: AccountProfile[] = []
let isInitialFetched = false
const listeners = new Set<(accounts: AccountProfile[]) => void>()

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
	}
}
