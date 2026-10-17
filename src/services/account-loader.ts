import type { AccountProfile } from "@/bindings"

interface AccountLoader {
	load: () => Promise<AccountProfile[]>
	invalidate: () => void
}

export const createAccountLoader = (
	fetchAccounts: () => Promise<AccountProfile[]>,
	publishAccounts: (accounts: AccountProfile[]) => void,
	reportFailure: (failure: unknown) => void = () => {},
): AccountLoader => {
	let pending: Promise<AccountProfile[]> | null = null
	let generation = 0

	const load = (): Promise<AccountProfile[]> => {
		if (pending) {
			return pending
		}

		const requestGeneration = generation
		pending = Promise.resolve()
			.then(fetchAccounts)
			.then((accounts) => {
				if (requestGeneration !== generation) {
					return load()
				}
				publishAccounts(accounts)
				return accounts
			})
			.catch((failure: unknown) => {
				if (requestGeneration === generation) {
					reportFailure(failure)
				}
				throw failure
			})
			.finally(() => {
				if (requestGeneration === generation) {
					pending = null
				}
			})
		return pending
	}
	const invalidate = () => {
		generation += 1
		pending = null
	}
	return { load, invalidate }
}
