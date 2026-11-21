import { useMemo } from "react"
import {
	findActiveAccount,
	mapBackendAccount,
	type UiAccountProfile,
} from "@/features/accounts/accountModel"
import { useAccounts } from "@/features/settings/api/accountsApi"

export interface UseActiveAccountResult {
	accounts: UiAccountProfile[]
	activeAccount: UiAccountProfile | null
	isLoading: boolean
	isError: boolean
	error: unknown
	refetch: () => Promise<unknown>
}

export const useActiveAccount = (): UseActiveAccountResult => {
	const query = useAccounts()

	const accounts = useMemo<UiAccountProfile[]>(() => {
		if (!query.data) {
			return []
		}
		return query.data.map((profile) => {
			return mapBackendAccount(profile)
		})
	}, [query.data])

	const activeAccount = useMemo<UiAccountProfile | null>(() => {
		return findActiveAccount(accounts)
	}, [accounts])

	return {
		accounts,
		activeAccount,
		isLoading: query.isPending,
		isError: query.isError,
		error: query.error,
		refetch: query.refetch,
	}
}
