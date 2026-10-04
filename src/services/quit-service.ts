import { create } from "zustand"
import type { QuitRequest } from "@/bindings"
import { rpc } from "@/services/server-service"

/** What leaving Ingot is for: quitting, installing an update, or restarting after one */
export type QuitMode = "quit" | "update" | "restart"

export interface QuitPrompt {
	mode: QuitMode
	request: QuitRequest
	/** Runs once the servers are stopped (update and restart) */
	proceed?: () => Promise<void>
}

/** The prompt the quit dialog is showing, if any */
export const useQuitPrompt = create<QuitPrompt | null>(() => null)

export const quitService = {
	ask(prompt: QuitPrompt) {
		useQuitPrompt.setState(prompt, true)
	},

	dismiss() {
		useQuitPrompt.setState(null, true)
	},

	/**
	 * Runs `action` (which ends Ingot) right away when no server would be left behind,
	 * otherwise asks to stop the servers first. A running server's console lives in
	 * Ingot: without it the server keeps running out of reach.
	 */
	async guard(mode: Exclude<QuitMode, "quit">, action: () => Promise<void>): Promise<void> {
		let request: QuitRequest
		try {
			request = await rpc.app.get_quit_blockers()
		} catch {
			return action()
		}
		if (request.servers.length === 0) return action()
		quitService.ask({ mode, request, proceed: action })
	},
}
