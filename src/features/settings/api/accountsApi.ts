import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type MicrosoftDeviceCode } from "@/bindings"
import { rpc } from "@/shared/api/rpc"

export const accountKeys = {
	all: ["accounts"] as const,
	list: () => {
		return [...accountKeys.all, "list"] as const
	},
	pendingMicrosoftLogin: () => {
		return [...accountKeys.all, "pendingMicrosoftLogin"] as const
	},
}

export const accountsQueryOptions = () => {
	return queryOptions({
		queryKey: accountKeys.list(),
		queryFn: () => {
			return rpc.accounts.get_accounts()
		},
	})
}

export const pendingMicrosoftLoginQueryOptions = () => {
	return queryOptions({
		queryKey: accountKeys.pendingMicrosoftLogin(),
		queryFn: () => {
			return rpc.accounts.get_pending_microsoft_login()
		},
		staleTime: 0,
	})
}

export const useAccounts = () => {
	return useQuery(accountsQueryOptions())
}

export const usePendingMicrosoftLogin = () => {
	return useQuery(pendingMicrosoftLoginQueryOptions())
}

export const useAddOfflineAccount = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (username: string) => {
			return rpc.accounts.add_offline_account(username)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useRemoveAccount = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (accountId: string) => {
			return rpc.accounts.remove_account(accountId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useSetActiveAccount = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (accountId: string) => {
			return rpc.accounts.set_active_account(accountId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useReorderAccounts = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (accountIds: string[]) => {
			return rpc.accounts.reorder_accounts(accountIds)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useElyLogin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { username: string; password: string }) => {
			return rpc.accounts.ely_login(input.username, input.password)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useStartMicrosoftLogin = () => {
	return useMutation({
		mutationFn: () => {
			return rpc.accounts.microsoft_login_start()
		},
	})
}

export const useFinishMicrosoftLogin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (code: MicrosoftDeviceCode) => {
			return rpc.accounts.microsoft_login_finish(code)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.all })
		},
	})
}

export const useCancelMicrosoftLogin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: () => {
			return rpc.accounts.microsoft_login_cancel()
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.pendingMicrosoftLogin() })
		},
	})
}

export const useActiveAccountToken = () => {
	return useMutation({
		mutationFn: () => {
			return rpc.accounts.get_active_account_token()
		},
	})
}

export const useDiscardPendingMicrosoftLogin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (ticket: string) => {
			return rpc.accounts.discard_pending_microsoft_login(ticket)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.pendingMicrosoftLogin() })
		},
	})
}

export const useResumeMicrosoftLogin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (ticket: string) => {
			return rpc.accounts.resume_microsoft_login(ticket)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.all })
		},
	})
}
