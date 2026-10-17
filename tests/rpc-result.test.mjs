import assert from "node:assert/strict"
import { test } from "node:test"
import { isRpcErrorKind, RpcError, unwrapRpc } from "../src/lib/rpc-result.ts"
import { createAccountLoader } from "../src/services/account-loader.ts"

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
