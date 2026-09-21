import { relaunch } from "@tauri-apps/plugin-process"
import { check, type Update } from "@tauri-apps/plugin-updater"
import { useEffect, useState } from "react"

export type UpdateStatus =
	| "idle"
	| "checking"
	| "available"
	| "downloading"
	| "ready"
	| "up-to-date"
	| "error"

export interface UpdateInfo {
	version: string
	currentVersion: string
	date?: string
	body?: string
}

export interface UpdateState {
	status: UpdateStatus
	updateInfo: UpdateInfo | null
	downloadProgress: number
	downloadedBytes: number
	totalBytes: number
	lastCheckedAt: number | null
	errorMessage: string | null
	isBannerDismissed: boolean
}

const isTauri =
	typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)

let cachedState: UpdateState = {
	status: "idle",
	updateInfo: null,
	downloadProgress: 0,
	downloadedBytes: 0,
	totalBytes: 0,
	lastCheckedAt: null,
	errorMessage: null,
	isBannerDismissed: false,
}

let activeUpdateHandle: Update | null = null
const stateListeners = new Set<(state: UpdateState) => void>()

function notifyListeners() {
	for (const listener of stateListeners) {
		listener({ ...cachedState })
	}
}

function updateState(partial: Partial<UpdateState>) {
	cachedState = { ...cachedState, ...partial }
	notifyListeners()
}

export const updateService = {
	getState(): UpdateState {
		return { ...cachedState }
	},

	subscribe(listener: (state: UpdateState) => void): () => void {
		stateListeners.add(listener)
		listener({ ...cachedState })
		return () => {
			stateListeners.delete(listener)
		}
	},

	async checkForUpdates(silent = false): Promise<UpdateInfo | null> {
		if (!isTauri) {
			console.log("[Updater] Not running in Tauri environment.")
			if (!silent) {
				updateState({
					status: "up-to-date",
					lastCheckedAt: Date.now(),
					errorMessage: null,
				})
			}
			return null
		}

		try {
			updateState({
				status: "checking",
				errorMessage: null,
			})

			const update = await check()
			const now = Date.now()

			if (update?.available) {
				activeUpdateHandle = update
				const info: UpdateInfo = {
					version: update.version,
					currentVersion: update.currentVersion,
					date: update.date,
					body: update.body,
				}
				updateState({
					status: "available",
					updateInfo: info,
					lastCheckedAt: now,
					isBannerDismissed: false,
					errorMessage: null,
				})
				return info
			}

			activeUpdateHandle = null
			updateState({
				status: silent ? "idle" : "up-to-date",
				updateInfo: null,
				lastCheckedAt: now,
				errorMessage: null,
			})
			return null
		} catch (err: unknown) {
			console.error("[Updater] Check for updates failed:", err)
			const message = err instanceof Error ? err.message : String(err)
			updateState({
				status: silent ? "idle" : "error",
				errorMessage: message,
				lastCheckedAt: Date.now(),
			})
			return null
		}
	},

	async downloadAndInstall(): Promise<void> {
		if (!activeUpdateHandle) {
			throw new Error("No update available to download.")
		}

		try {
			updateState({
				status: "downloading",
				downloadProgress: 0,
				downloadedBytes: 0,
				totalBytes: 0,
				errorMessage: null,
			})

			let totalLength = 0
			let downloaded = 0

			await activeUpdateHandle.downloadAndInstall((event) => {
				switch (event.event) {
					case "Started":
						totalLength = event.data.contentLength ?? 0
						updateState({
							totalBytes: totalLength,
							downloadedBytes: 0,
							downloadProgress: 0,
						})
						break
					case "Progress":
						downloaded += event.data.chunkLength
						{
							const progress = totalLength > 0 ? Math.round((downloaded / totalLength) * 100) : 0
							updateState({
								downloadedBytes: downloaded,
								downloadProgress: Math.min(progress, 100),
							})
						}
						break
					case "Finished":
						updateState({
							downloadProgress: 100,
						})
						break
				}
			})

			updateState({
				status: "ready",
				downloadProgress: 100,
			})
		} catch (err: unknown) {
			console.error("[Updater] Download and install failed:", err)
			const message = err instanceof Error ? err.message : String(err)
			updateState({
				status: "error",
				errorMessage: message,
			})
			throw err
		}
	},

	async relaunchApp(): Promise<void> {
		try {
			await relaunch()
		} catch (err) {
			console.error("[Updater] Relaunch failed:", err)
			throw err
		}
	},

	dismissBanner() {
		updateState({ isBannerDismissed: true })
	},

	resetStatus() {
		updateState({
			status: "idle",
			errorMessage: null,
		})
	},
}

// Auto-check silently 3.5s after app starts in background
if (typeof window !== "undefined" && isTauri) {
	setTimeout(() => {
		updateService.checkForUpdates(true).catch(() => {})
	}, 3500)
}

export function useUpdateService() {
	const [state, setState] = useState<UpdateState>(updateService.getState())

	useEffect(() => {
		return updateService.subscribe((updated) => {
			setState(updated)
		})
	}, [])

	return {
		...state,
		checkForUpdates: updateService.checkForUpdates.bind(updateService),
		downloadAndInstall: updateService.downloadAndInstall.bind(updateService),
		relaunchApp: updateService.relaunchApp.bind(updateService),
		dismissBanner: updateService.dismissBanner.bind(updateService),
		resetStatus: updateService.resetStatus.bind(updateService),
	}
}
