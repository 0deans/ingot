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

const listeners = new Set<(prompt: QuitPrompt) => void>()

export const quitService = {
	subscribe(listener: (prompt: QuitPrompt) => void): () => void {
		listeners.add(listener)
		return () => {
			listeners.delete(listener)
		}
	},

	ask(prompt: QuitPrompt) {
		for (const listener of listeners) listener(prompt)
	},

	/**
	 * Runs `action` (which ends Ingot) right away when no server would be left behind,
	 * otherwise asks to stop the servers first. A running server's console lives in
	 * Ingot: without it the server keeps running out of reach.
	 */
	async guard(mode: Exclude<QuitMode, "quit">, action: () => Promise<void>): Promise<void> {
		let request: QuitRequest
		try {
			request = await rpc.get_quit_blockers()
		} catch {
			return action()
		}
		if (request.servers.length === 0) return action()
		quitService.ask({ mode, request, proceed: action })
	},
}
