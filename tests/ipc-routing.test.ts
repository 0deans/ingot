import { emit } from "@tauri-apps/api/event"
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"
import { afterEach, expect, test } from "vitest"
import { createTauRPCProxy } from "@/bindings"

interface RecordedIpcCall {
	command: string
	args: unknown
}

interface TauriInternalsWithCallback {
	runCallback: (id: number, data: unknown) => void
}

const isRecord = (value: unknown): value is Record<string, unknown> => {
	return typeof value === "object" && value !== null
}

const hasRunCallback = (value: unknown): value is TauriInternalsWithCallback => {
	return isRecord(value) && typeof value.runCallback === "function"
}

const getStringProperty = (args: unknown, key: string): string | undefined => {
	if (!isRecord(args)) {
		return undefined
	}
	const value = args[key]
	return typeof value === "string" ? value : undefined
}

const getNumberProperty = (args: unknown, key: string): number | undefined => {
	if (!isRecord(args)) {
		return undefined
	}
	const value = args[key]
	return typeof value === "number" ? value : undefined
}

afterEach(() => {
	clearMocks()
})

test("settings namespace sends the existing named arguments and unwraps success", async () => {
	const calls: RecordedIpcCall[] = []
	const memory = { minRamMb: 1024, maxRamMb: 8192 }
	mockIPC((command, args) => {
		calls.push({ command, args })
		return memory
	})

	const rpc = createTauRPCProxy()
	await expect(rpc.settings.set_memory_settings(1024, 8192)).resolves.toEqual(memory)
	expect(calls).toEqual([
		{
			command: "TauRPC__settings.set_memory_settings",
			args: { min_ram_mb: 1024, max_ram_mb: 8192 },
		},
	])
	expect(() => {
		return Reflect.get(rpc, "set_memory_settings")
	}).toThrow(/not found/)
})

test("expected settings failures return typed error envelopes", async () => {
	const apiError = { kind: "invalidInput", message: "Invalid launcher behavior: typo" }
	mockIPC(() => {
		return Promise.reject(apiError)
	})

	const rpc = createTauRPCProxy()
	await expect(rpc.settings.set_launcher_behavior("typo")).rejects.toEqual(apiError)
})

test("namespaced events deliver payloads, filter other routes, and unsubscribe", async () => {
	const listeners = new Set<number>()
	const removals: RecordedIpcCall["args"][] = []

	// The installed event mock uses `id` for unlisten; the real API sends
	// `eventId`. Model the actual event-plugin contract at the invoke boundary.
	mockIPC((command, args) => {
		if (command === "plugin:event|listen") {
			const handler = getNumberProperty(args, "handler")
			if (handler !== undefined) {
				listeners.add(handler)
			}
			return handler
		}
		if (command === "plugin:event|emit") {
			const internals: unknown = Reflect.get(window, "__TAURI_INTERNALS__")
			if (!hasRunCallback(internals)) {
				throw new Error("Missing __TAURI_INTERNALS__.runCallback")
			}
			for (const id of listeners) {
				internals.runCallback(id, args)
			}
			return
		}
		if (command === "plugin:event|unlisten") {
			removals.push(args)
			const eventId = getNumberProperty(args, "eventId")
			if (eventId !== undefined) {
				listeners.delete(eventId)
			}
			return
		}
		throw new Error(`Unexpected invoke: ${command}`)
	})

	const rpc = createTauRPCProxy()
	const received: Array<{ minRamMb: number; maxRamMb: number }> = []
	const unlisten = await rpc.events.on_memory_changed.on((memory) => {
		received.push(memory)
	})
	const memory = { minRamMb: 2048, maxRamMb: 4096 }
	const payload = (eventName: string) => {
		return { event_name: eventName, event: { input_type: memory } }
	}

	await emit("TauRpc_event", payload("on_memory_changed"))
	expect(received).toEqual([])
	await emit("TauRpc_event", payload("events.on_memory_changed"))
	expect(received).toEqual([memory])

	await unlisten()
	expect(listeners.size).toBe(0)
	expect(removals).toHaveLength(1)
	expect(getStringProperty(removals[0], "event")).toBe("TauRpc_event")

	await emit("TauRpc_event", payload("events.on_memory_changed"))
	expect(received).toEqual([memory])
	expect(() => {
		return Reflect.get(rpc, "on_memory_changed")
	}).toThrow(/not found/)
})

test("application commands use their new namespace", async () => {
	const calls: RecordedIpcCall[] = []
	mockIPC((command, args) => {
		calls.push({ command, args })
		return "hello"
	})

	await expect(createTauRPCProxy().app.greet("world")).resolves.toBe("hello")
	expect(calls).toEqual([{ command: "TauRPC__app.greet", args: { name: "world" } }])
})

test("account and skin namespaces preserve positional arguments and typed error envelopes", async () => {
	const calls: RecordedIpcCall[] = []
	const apiError = { kind: "notFound", message: "Account not found" }
	mockIPC((command, args) => {
		calls.push({ command, args })
		if (command === "TauRPC__accounts.set_active_account") {
			return Promise.reject(apiError)
		}
		return null
	})

	const rpc = createTauRPCProxy()
	await expect(rpc.accounts.reorder_accounts(["offline:b", "offline:a"])).resolves.toBeNull()
	await expect(rpc.accounts.microsoft_login_cancel()).resolves.toBeNull()
	await expect(rpc.skins.apply_ely_skin("ely:123", 42, null)).resolves.toBeNull()
	await expect(rpc.skins.upload_microsoft_skin("microsoft:123", "image", true)).resolves.toBeNull()
	await expect(rpc.accounts.set_active_account("missing")).rejects.toEqual(apiError)

	expect(calls).toEqual([
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
	expect(() => {
		return Reflect.get(rpc, "get_accounts")
	}).toThrow(/not found/)
	expect(() => {
		return Reflect.get(rpc, "apply_ely_skin")
	}).toThrow(/not found/)
})

test("instance and server commands route through their namespaces", async () => {
	const calls: RecordedIpcCall[] = []
	mockIPC((command, args) => {
		calls.push({ command, args })
		return null
	})

	const rpc = createTauRPCProxy()
	await expect(rpc.instances.kill_instance("instance-id")).resolves.toBeNull()
	await expect(rpc.instances.create_instance_shortcut("instance-id", "desktop")).resolves.toBeNull()
	expect(calls).toEqual([
		{ command: "TauRPC__instances.kill_instance", args: { instance_id: "instance-id" } },
		{
			command: "TauRPC__instances.create_instance_shortcut",
			args: { instance_id: "instance-id", location: "desktop" },
		},
	])
	expect(() => {
		return Reflect.get(rpc, "kill_instance")
	}).toThrow(/not found/)
})

test("Microsoft cancellation cleanup failures return typed error envelopes", async () => {
	const apiError = { kind: "incompleteRecovery", message: "pending grant cleanup failed" }
	mockIPC(() => {
		return Promise.reject(apiError)
	})

	await expect(createTauRPCProxy().accounts.microsoft_login_cancel()).rejects.toEqual(apiError)
})

test("pending Microsoft sign-in metadata and explicit tickets use the accounts namespace", async () => {
	const calls: RecordedIpcCall[] = []
	const pending = { ticket: "opaque-ticket", createdAt: 1000, expiresAt: 87400, status: "Ready" }
	const apiError = { kind: "invalidInput", message: "sign-in expired" }
	mockIPC((command, args) => {
		calls.push({ command, args })
		if (command === "TauRPC__accounts.get_pending_microsoft_login") {
			return pending
		}
		if (command === "TauRPC__accounts.resume_microsoft_login") {
			return Promise.reject(apiError)
		}
		return null
	})

	const rpc = createTauRPCProxy()
	await expect(rpc.accounts.get_pending_microsoft_login()).resolves.toEqual(pending)
	await expect(rpc.accounts.resume_microsoft_login(pending.ticket)).rejects.toEqual(apiError)
	await expect(rpc.accounts.discard_pending_microsoft_login(pending.ticket)).resolves.toBeNull()
	expect(calls).toEqual([
		{ command: "TauRPC__accounts.get_pending_microsoft_login", args: {} },
		{ command: "TauRPC__accounts.resume_microsoft_login", args: { ticket: pending.ticket } },
		{
			command: "TauRPC__accounts.discard_pending_microsoft_login",
			args: { ticket: pending.ticket },
		},
	])
})

test("version change recovery commands route to their instances and servers namespaces", async () => {
	const calls: RecordedIpcCall[] = []
	const dummyConfig = { id: "test-id", name: "Test", gameVersion: "1.20.1" }
	mockIPC((command, args) => {
		calls.push({ command, args })
		return dummyConfig
	})

	const rpc = createTauRPCProxy()
	await expect(
		rpc.instances.version_change.recover_instance_version_change("inst-1"),
	).resolves.toEqual(dummyConfig)
	await expect(rpc.servers.version_change.recover_server_version_change("srv-1")).resolves.toEqual(
		dummyConfig,
	)

	expect(calls).toEqual([
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
	const calls: RecordedIpcCall[] = []
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
	await expect(
		rpc.instances.version_change.get_instance_version_transaction_status("inst-1"),
	).resolves.toEqual(dummyStatus)
	await expect(
		rpc.servers.version_change.get_server_version_transaction_status("srv-1"),
	).resolves.toEqual(dummyStatus)

	expect(calls).toEqual([
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

test("instances.logs commands route through their namespace and preserve arguments", async () => {
	const calls: RecordedIpcCall[] = []
	const pasteResult = {
		provider: "mclogs",
		id: "abc1234",
		url: "https://mclo.gs/abc1234",
		rawUrl: "https://api.mclo.gs/1/raw/abc1234",
	}
	mockIPC((command, args) => {
		calls.push({ command, args })
		if (command === "TauRPC__instances.logs.upload_log_paste") {
			return pasteResult
		}
		return []
	})

	const rpc = createTauRPCProxy()
	await expect(rpc.instances.logs.get_live_instance_logs("inst-1")).resolves.toEqual([])
	await expect(rpc.instances.logs.list_instance_log_files("inst-1")).resolves.toEqual([])
	await expect(rpc.instances.logs.upload_log_paste("log text", "mclogs")).resolves.toEqual(
		pasteResult,
	)

	expect(calls).toEqual([
		{
			command: "TauRPC__instances.logs.get_live_instance_logs",
			args: { instance_id: "inst-1" },
		},
		{
			command: "TauRPC__instances.logs.list_instance_log_files",
			args: { instance_id: "inst-1" },
		},
		{
			command: "TauRPC__instances.logs.upload_log_paste",
			args: { content: "log text", provider: "mclogs" },
		},
	])
})
