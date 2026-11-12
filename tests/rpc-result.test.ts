import { expect, test } from "vitest"
import type { AccountProfile, ApiError, TauRpcResult } from "@/bindings"
import { isRpcErrorKind, RpcError, unwrapRpc } from "@/lib/rpc-result"
import { createAccountLoader } from "@/services/account-loader"
import { dismissCurrentPrompt, stopServersAndExit } from "@/services/quit-flow"
import type { QuitExitIntent } from "@/services/quit-service"

const makeAccount = (id: string): AccountProfile => {
	return {
		id,
		accountType: "offline",
		username: id,
		uuid: id,
		skinUrl: null,
		isActive: false,
		createdAt: 0,
	}
}

test("RPC results retain error categories and transport failures", async () => {
	expect(unwrapRpc({ status: "ok", data: 42 })).toBe(42)

	const error: ApiError = { kind: "incompleteRecovery", message: "Recovery required" }
	try {
		await unwrapRpc(Promise.resolve({ status: "error", error }))
		expect.unreachable()
	} catch (failure: unknown) {
		expect(failure).toBeInstanceOf(RpcError)
		expect(isRpcErrorKind(failure, "incompleteRecovery")).toBe(true)
		if (failure instanceof RpcError) {
			expect(failure.apiError).toEqual(error)
		}
	}

	const transport = new Error("Transport disconnected")
	try {
		await unwrapRpc(Promise.reject(transport))
		expect.unreachable()
	} catch (failure: unknown) {
		expect(failure).toBe(transport)
		expect(isRpcErrorKind(failure, "internal")).toBe(false)
	}
})

test("account refresh publishes only successful data and can retry after failure", async () => {
	let attempts = 0
	const published: AccountProfile[][] = []
	const failure = new RpcError({ kind: "storageFailure", message: "Store unavailable" })
	const loader = createAccountLoader(
		() => {
			attempts += 1
			if (attempts === 1) {
				return Promise.reject(failure)
			}
			return Promise.resolve([])
		},
		(accounts) => {
			published.push(accounts)
		},
	)

	const first = loader.load()
	expect(loader.load()).toBe(first)
	await expect(first).rejects.toBe(failure)
	expect(published).toEqual([])
	await expect(loader.load()).resolves.toEqual([])
	expect(attempts).toBe(2)
	expect(published).toEqual([[]])
})

test("account mutations invalidate older snapshots without replacing newer data or failures", async () => {
	const oldRequest = Promise.withResolvers<AccountProfile[]>()
	const newRequest = Promise.withResolvers<AccountProfile[]>()
	const requests = [oldRequest.promise, newRequest.promise]
	const published: AccountProfile[][] = []
	const failures: unknown[] = []
	const loader = createAccountLoader(
		() => {
			const request = requests.shift()
			if (!request) {
				throw new Error("Expected pending request")
			}
			return request
		},
		(accounts) => {
			published.push(accounts)
		},
		(failure) => {
			failures.push(failure)
		},
	)

	const oldLoad = loader.load()
	await Promise.resolve()
	loader.invalidate()
	const currentLoad = loader.load()
	await Promise.resolve()
	oldRequest.resolve([makeAccount("removed account")])
	newRequest.resolve([])

	await expect(currentLoad).resolves.toEqual([])
	await expect(oldLoad).resolves.toEqual([])
	expect(published).toEqual([[]])
	expect(failures).toEqual([])
})

test("failed server shutdown prevents quitting, continuation and dismissal, then allows retry", async () => {
	const modes: QuitExitIntent["mode"][] = ["quit", "update", "restart"]

	for (const mode of modes) {
		const events: string[] = []
		let attempts = 0
		const intent: QuitExitIntent =
			mode === "quit"
				? { mode: "quit" }
				: {
						mode,
						proceed: async () => {
							events.push("proceed")
						},
					}
		const effects = {
			stopServers: async (): Promise<TauRpcResult<null, ApiError>> => {
				events.push("stop")
				attempts += 1
				if (attempts === 1) {
					return {
						status: "error",
						error: { kind: "incompleteRecovery", message: "Exit unconfirmed" },
					}
				}
				return { status: "ok", data: null }
			},
			quit: async (): Promise<TauRpcResult<null, ApiError>> => {
				events.push("quit")
				return { status: "ok", data: null }
			},
			dismiss: () => {
				events.push("dismiss")
			},
		}

		try {
			await stopServersAndExit(intent, effects)
			expect.unreachable()
		} catch (failure: unknown) {
			expect(isRpcErrorKind(failure, "incompleteRecovery")).toBe(true)
		}
		expect(events).toEqual(["stop"])

		await stopServersAndExit(intent, effects)
		if (mode === "quit") {
			expect(events).toEqual(["stop", "stop", "quit"])
		} else {
			expect(events).toEqual(["stop", "stop", "proceed", "dismiss"])
		}
	}
})

test("quit action waits for shutdown acknowledgement and retains quit RPC failures", async () => {
	const acknowledgement = Promise.withResolvers<TauRpcResult<null, ApiError>>()
	const events: string[] = []
	const operation = stopServersAndExit(
		{ mode: "quit" },
		{
			stopServers: () => {
				events.push("stop")
				return acknowledgement.promise
			},
			quit: async () => {
				events.push("quit")
				return { status: "error", error: { kind: "internal", message: "Quit failed" } }
			},
			dismiss: () => {
				events.push("dismiss")
			},
		},
	)

	expect(events).toEqual(["stop"])
	acknowledgement.resolve({ status: "ok", data: null })
	try {
		await operation
		expect.unreachable()
	} catch (failure: unknown) {
		expect(isRpcErrorKind(failure, "internal")).toBe(true)
	}
	expect(events).toEqual(["stop", "quit"])
})

test("failed update continuation keeps its prompt available for retry", async () => {
	const events: string[] = []
	let attempts = 0
	const failure = new Error("Update unavailable")
	const intent: QuitExitIntent = {
		mode: "update",
		proceed: async () => {
			events.push("proceed")
			attempts += 1
			if (attempts === 1) {
				throw failure
			}
		},
	}
	const effects = {
		stopServers: async (): Promise<TauRpcResult<null, ApiError>> => {
			events.push("stop")
			return { status: "ok", data: null }
		},
		quit: async (): Promise<TauRpcResult<null, ApiError>> => {
			events.push("quit")
			return { status: "ok", data: null }
		},
		dismiss: () => {
			events.push("dismiss")
		},
	}

	await expect(stopServersAndExit(intent, effects)).rejects.toBe(failure)
	expect(events).toEqual(["stop", "proceed"])
	await stopServersAndExit(intent, effects)
	expect(events).toEqual(["stop", "proceed", "stop", "proceed", "dismiss"])
})

test("an older continuation cannot dismiss a newly installed quit prompt", async () => {
	const proceeding = Promise.withResolvers<void>()
	const entered = Promise.withResolvers<void>()
	const original: QuitExitIntent = {
		mode: "update",
		proceed: async () => {
			entered.resolve()
			await proceeding.promise
		},
	}
	const replacement: QuitExitIntent = { mode: "quit" }
	let currentPrompt: QuitExitIntent | null = original
	let dismissals = 0

	const operation = stopServersAndExit(original, {
		stopServers: async () => {
			return { status: "ok", data: null }
		},
		quit: async () => {
			expect.unreachable("An update must not invoke quit")
		},
		dismiss: () => {
			dismissCurrentPrompt(original, currentPrompt, () => {
				dismissals += 1
				currentPrompt = null
			})
		},
	})

	await entered.promise
	expect(currentPrompt).toBe(original)
	currentPrompt = replacement
	proceeding.resolve()
	await operation
	expect(currentPrompt).toBe(replacement)
	expect(dismissals).toBe(0)
})
