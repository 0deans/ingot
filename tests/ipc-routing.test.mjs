import assert from "node:assert/strict"
import { afterEach, beforeEach, test } from "node:test"
import { emit } from "@tauri-apps/api/event"
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"
import { createTauRPCProxy } from "../src/bindings.ts"

// Exercise the generated proxy and installed TauRPC runtime without a browser.
beforeEach(() => {
	globalThis.window = globalThis
})
afterEach(() => {
	clearMocks()
	delete globalThis.window
})

test("settings namespace sends the existing named arguments and unwraps success", async () => {
	const calls = []
	const memory = { minRamMb: 1024, maxRamMb: 8192 }
	mockIPC((command, args) => {
		calls.push({ command, args })
		return memory
	})
	const rpc = createTauRPCProxy()
	assert.deepEqual(await rpc.settings.set_memory_settings(1024, 8192), {
		status: "ok",
		data: memory,
	})
	assert.deepEqual(calls, [
		{
			command: "TauRPC__settings.set_memory_settings",
			args: { min_ram_mb: 1024, max_ram_mb: 8192 },
		},
	])
	assert.throws(() => rpc.set_memory_settings, /not found/)
})

test("expected settings failures return typed error envelopes", async () => {
	const apiError = { kind: "invalidInput", message: "Invalid launcher behavior: typo" }
	mockIPC(() => Promise.reject(apiError))
	const rpc = createTauRPCProxy()
	const result = await rpc.settings.set_launcher_behavior("typo")
	assert.deepEqual(result, {
		status: "error",
		error: apiError,
	})
})

test("namespaced events deliver payloads, filter other routes, and unsubscribe", async () => {
	const listeners = new Set()
	const removals = []
	// The installed event mock uses `id` for unlisten; the real API sends
	// `eventId`. Model the actual event-plugin contract at the invoke boundary.
	mockIPC((command, args) => {
		if (command === "plugin:event|listen") {
			listeners.add(args.handler)
			return args.handler
		}
		if (command === "plugin:event|emit") {
			for (const id of listeners) {
				window.__TAURI_INTERNALS__.runCallback(id, args)
			}
			return
		}
		if (command === "plugin:event|unlisten") {
			removals.push(args)
			listeners.delete(args.eventId)
			return
		}
		throw new Error(`Unexpected invoke: ${command}`)
	})
	const rpc = createTauRPCProxy()
	const received = []
	const unlisten = await rpc.events.on_memory_changed.on((memory) => received.push(memory))
	const memory = { minRamMb: 2048, maxRamMb: 4096 }
	const payload = (eventName) => ({ event_name: eventName, event: { input_type: memory } })
	await emit("TauRpc_event", payload("on_memory_changed"))
	assert.deepEqual(received, [])
	await emit("TauRpc_event", payload("events.on_memory_changed"))
	assert.deepEqual(received, [memory])
	await unlisten()
	assert.equal(listeners.size, 0)
	assert.equal(removals.length, 1)
	assert.equal(removals[0].event, "TauRpc_event")
	await emit("TauRpc_event", payload("events.on_memory_changed"))
	assert.deepEqual(received, [memory])
	assert.throws(() => rpc.on_memory_changed, /not found/)
})

test("application commands use their new namespace", async () => {
	const calls = []
	mockIPC((command, args) => {
		calls.push({ command, args })
		return "hello"
	})
	assert.equal(await createTauRPCProxy().app.greet("world"), "hello")
	assert.deepEqual(calls, [{ command: "TauRPC__app.greet", args: { name: "world" } }])
})

test("account and skin namespaces preserve positional arguments and typed error envelopes", async () => {
	const calls = []
	const apiError = { kind: "notFound", message: "Account not found" }
	mockIPC((command, args) => {
		calls.push({ command, args })
		if (command === "TauRPC__accounts.set_active_account") return Promise.reject(apiError)
		return null
	})
	const rpc = createTauRPCProxy()
	assert.deepEqual(await rpc.accounts.reorder_accounts(["offline:b", "offline:a"]), {
		status: "ok",
		data: null,
	})
	assert.deepEqual(await rpc.accounts.microsoft_login_cancel(), { status: "ok", data: null })
	assert.deepEqual(await rpc.skins.apply_ely_skin("ely:123", 42, null), {
		status: "ok",
		data: null,
	})
	assert.deepEqual(await rpc.skins.upload_microsoft_skin("microsoft:123", "image", true), {
		status: "ok",
		data: null,
	})
	assert.deepEqual(await rpc.accounts.set_active_account("missing"), {
		status: "error",
		error: apiError,
	})
	assert.deepEqual(calls, [
		{
			command: "TauRPC__accounts.reorder_accounts",
			args: { account_ids: ["offline:b", "offline:a"] },
		},
		{ command: "TauRPC__accounts.microsoft_login_cancel", args: {} },
		{
			command: "TauRPC__skins.apply_ely_skin",
			args: { account_id: "ely:123", skin_id: 42, password: null },
		},
		{
			command: "TauRPC__skins.upload_microsoft_skin",
			args: { account_id: "microsoft:123", image_base64: "image", is_slim: true },
		},
		{ command: "TauRPC__accounts.set_active_account", args: { account_id: "missing" } },
	])
	assert.throws(() => rpc.get_accounts, /not found/)
	assert.throws(() => rpc.apply_ely_skin, /not found/)
})

test("instance and server commands route through their namespaces", async () => {
	const calls = []
	mockIPC((command, args) => {
		calls.push({ command, args })
		return null
	})
	const rpc = createTauRPCProxy()
	assert.deepEqual(await rpc.instances.kill_instance("instance-id"), { status: "ok", data: null })
	assert.deepEqual(calls, [
		{ command: "TauRPC__instances.kill_instance", args: { instance_id: "instance-id" } },
	])
	assert.throws(() => rpc.kill_instance, /not found/)
})

test("Microsoft cancellation cleanup failures return typed error envelopes", async () => {
	const apiError = { kind: "incompleteRecovery", message: "pending grant cleanup failed" }
	mockIPC(() => Promise.reject(apiError))
	const result = await createTauRPCProxy().accounts.microsoft_login_cancel()
	assert.deepEqual(result, { status: "error", error: apiError })
})

test("pending Microsoft sign-in metadata and explicit tickets use the accounts namespace", async () => {
	const calls = []
	const pending = { ticket: "opaque-ticket", createdAt: 1000, expiresAt: 87400, status: "Ready" }
	const apiError = { kind: "invalidInput", message: "sign-in expired" }
	mockIPC((command, args) => {
		calls.push({ command, args })
		if (command === "TauRPC__accounts.get_pending_microsoft_login") return pending
		if (command === "TauRPC__accounts.resume_microsoft_login") return Promise.reject(apiError)
		return null
	})
	const rpc = createTauRPCProxy()
	assert.deepEqual(await rpc.accounts.get_pending_microsoft_login(), {
		status: "ok",
		data: pending,
	})
	assert.deepEqual(await rpc.accounts.resume_microsoft_login(pending.ticket), {
		status: "error",
		error: apiError,
	})
	assert.deepEqual(await rpc.accounts.discard_pending_microsoft_login(pending.ticket), {
		status: "ok",
		data: null,
	})
	assert.deepEqual(calls, [
		{ command: "TauRPC__accounts.get_pending_microsoft_login", args: {} },
		{ command: "TauRPC__accounts.resume_microsoft_login", args: { ticket: pending.ticket } },
		{
			command: "TauRPC__accounts.discard_pending_microsoft_login",
			args: { ticket: pending.ticket },
		},
	])
})

test("version change recovery commands route to their instances and servers namespaces", async () => {
	const calls = []
	const dummyConfig = { id: "test-id", name: "Test", gameVersion: "1.20.1" }
	mockIPC((command, args) => {
		calls.push({ command, args })
		return dummyConfig
	})
	const rpc = createTauRPCProxy()
	const instRes = await rpc.instances.version_change.recover_instance_version_change("inst-1")
	assert.deepEqual(instRes, { status: "ok", data: dummyConfig })

	const srvRes = await rpc.servers.version_change.recover_server_version_change("srv-1")
	assert.deepEqual(srvRes, { status: "ok", data: dummyConfig })

	assert.deepEqual(calls, [
		{
			command: "TauRPC__instances.version_change.recover_instance_version_change",
			args: { instance_id: "inst-1" },
		},
		{
			command: "TauRPC__servers.version_change.recover_server_version_change",
			args: { server_id: "srv-1" },
		},
	])
})

test("version change transaction status commands route to instances and servers namespaces", async () => {
	const calls = []
	const dummyStatus = {
		phase: "rollbackFailed",
		isIncomplete: true,
		canRecover: true,
		backup: null,
		error: "Restoration interrupted",
	}
	mockIPC((command, args) => {
		calls.push({ command, args })
		return dummyStatus
	})
	const rpc = createTauRPCProxy()
	const instRes =
		await rpc.instances.version_change.get_instance_version_transaction_status("inst-1")
	assert.deepEqual(instRes, { status: "ok", data: dummyStatus })

	const srvRes = await rpc.servers.version_change.get_server_version_transaction_status("srv-1")
	assert.deepEqual(srvRes, { status: "ok", data: dummyStatus })

	assert.deepEqual(calls, [
		{
			command: "TauRPC__instances.version_change.get_instance_version_transaction_status",
			args: { instance_id: "inst-1" },
		},
		{
			command: "TauRPC__servers.version_change.get_server_version_transaction_status",
			args: { server_id: "srv-1" },
		},
	])
})
