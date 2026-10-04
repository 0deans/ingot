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
	assert.deepEqual(await rpc.settings.set_memory_settings(1024, 8192), memory)
	assert.deepEqual(calls, [
		{
			command: "TauRPC__settings.set_memory_settings",
			args: { min_ram_mb: 1024, max_ram_mb: 8192 },
		},
	])
	assert.throws(() => rpc.set_memory_settings, /not found/)
})

test("expected settings failures still reject instead of becoming successful values", async () => {
	mockIPC(() => Promise.reject("Invalid launcher behavior: typo"))
	const rpc = createTauRPCProxy()
	await assert.rejects(rpc.settings.set_launcher_behavior("typo"), (error) => {
		assert.equal(error, "Invalid launcher behavior: typo")
		return true
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

test("unmigrated root commands retain their invocation path", async () => {
	const calls = []
	mockIPC((command, args) => {
		calls.push({ command, args })
		return "hello"
	})
	assert.equal(await createTauRPCProxy().greet("world"), "hello")
	assert.deepEqual(calls, [{ command: "TauRPC__greet", args: { name: "world" } }])
})
