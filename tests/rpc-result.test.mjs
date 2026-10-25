import assert from "node:assert/strict"
import { test } from "node:test"
import { isRpcErrorKind, RpcError, unwrapRpc } from "../src/lib/rpc-result.ts"
import { createAccountLoader } from "../src/services/account-loader.ts"
import { dismissCurrentPrompt, stopServersAndExit } from "../src/services/quit-flow.ts"

test("RPC results retain error categories and transport failures", async () => {
	assert.equal(unwrapRpc({ status: "ok", data: 42 }), 42)
	const error = { kind: "incompleteRecovery", message: "Recovery required" }
	await assert.rejects(unwrapRpc(Promise.resolve({ status: "error", error })), (failure) => {
		assert.ok(failure instanceof RpcError)
		assert.ok(isRpcErrorKind(failure, "incompleteRecovery"))
		assert.deepEqual(failure.apiError, error)
		return true
	})
	const transport = new Error("Transport disconnected")
	await assert.rejects(unwrapRpc(Promise.reject(transport)), (failure) => {
		assert.equal(failure, transport)
		assert.equal(isRpcErrorKind(failure, "internal"), false)
		return true
	})
})

test("account refresh publishes only successful data and can retry after failure", async () => {
	let attempts = 0
	const published = []
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
	assert.equal(loader.load(), first)
	await assert.rejects(first, (error) => {
		return error === failure
	})
	assert.deepEqual(published, [])
	assert.deepEqual(await loader.load(), [])
	assert.equal(attempts, 2)
	assert.deepEqual(published, [[]])
})

test("account mutations invalidate older snapshots without replacing newer data or failures", async () => {
	const oldRequest = Promise.withResolvers()
	const newRequest = Promise.withResolvers()
	const requests = [oldRequest.promise, newRequest.promise]
	const published = []
	const failures = []
	const loader = createAccountLoader(
		() => {
			const request = requests.shift()
			assert.ok(request)
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
	oldRequest.resolve([{ id: "removed account" }])
	newRequest.resolve([])
	assert.deepEqual(await currentLoad, [])
	assert.deepEqual(await oldLoad, [])
	assert.deepEqual(published, [[]])
	assert.deepEqual(failures, [])
})

test("failed server shutdown prevents quitting, continuation and dismissal, then allows retry", async () => {
	for (const mode of ["quit", "update", "restart"]) {
		const events = []
		let attempts = 0
		const intent = {
			mode,
			proceed: async () => {
				events.push("proceed")
			},
		}
		const effects = {
			stopServers: async () => {
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
			quit: async () => {
				events.push("quit")
				return { status: "ok", data: null }
			},
			dismiss: () => {
				events.push("dismiss")
			},
		}
		await assert.rejects(stopServersAndExit(intent, effects), (failure) => {
			return isRpcErrorKind(failure, "incompleteRecovery")
		})
		assert.deepEqual(events, ["stop"], mode)
		await stopServersAndExit(intent, effects)
		if (mode === "quit") {
			assert.deepEqual(events, ["stop", "stop", "quit"])
		} else {
			assert.deepEqual(events, ["stop", "stop", "proceed", "dismiss"])
		}
	}
})

test("quit action waits for shutdown acknowledgement and retains quit RPC failures", async () => {
	const acknowledgement = Promise.withResolvers()
	const events = []
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
	assert.deepEqual(events, ["stop"])
	acknowledgement.resolve({ status: "ok", data: null })
	await assert.rejects(operation, (failure) => {
		return isRpcErrorKind(failure, "internal")
	})
	assert.deepEqual(events, ["stop", "quit"])
})

test("failed update continuation keeps its prompt available for retry", async () => {
	const events = []
	let attempts = 0
	const failure = new Error("Update unavailable")
	const intent = {
		mode: "update",
		proceed: async () => {
			events.push("proceed")
			attempts += 1
			if (attempts === 1) throw failure
		},
	}
	const effects = {
		stopServers: async () => {
			events.push("stop")
			return { status: "ok", data: null }
		},
		quit: async () => {
			events.push("quit")
			return { status: "ok", data: null }
		},
		dismiss: () => {
			events.push("dismiss")
		},
	}
	await assert.rejects(stopServersAndExit(intent, effects), (error) => {
		return error === failure
	})
	assert.deepEqual(events, ["stop", "proceed"])
	await stopServersAndExit(intent, effects)
	assert.deepEqual(events, ["stop", "proceed", "stop", "proceed", "dismiss"])
})

test("an older continuation cannot dismiss a newly installed quit prompt", async () => {
	const proceeding = Promise.withResolvers()
	const entered = Promise.withResolvers()
	const original = {
		mode: "update",
		proceed: async () => {
			entered.resolve()
			await proceeding.promise
		},
	}
	const replacement = { mode: "quit" }
	let currentPrompt = original
	let dismissals = 0
	const operation = stopServersAndExit(original, {
		stopServers: async () => {
			return { status: "ok", data: null }
		},
		quit: async () => {
			assert.fail("An update must not invoke quit")
		},
		dismiss: () => {
			dismissCurrentPrompt(original, currentPrompt, () => {
				dismissals += 1
				currentPrompt = null
			})
		},
	})
	await entered.promise
	assert.equal(currentPrompt, original)
	currentPrompt = replacement
	proceeding.resolve()
	await operation
	assert.equal(currentPrompt, replacement)
	assert.equal(dismissals, 0)
})
