import { convertFileSrc } from "@tauri-apps/api/core"
import { useEffect, useState } from "react"
import { createTauRPCProxy, type ScreenshotInfo } from "@/bindings"

export const rpc = createTauRPCProxy()

let cachedScreenshots: ScreenshotInfo[] = []
let isInitialFetched = false
let isLoading = false
const listeners = new Set<(screenshots: ScreenshotInfo[]) => void>()
const loadingListeners = new Set<(loading: boolean) => void>()

function notify(screenshots: ScreenshotInfo[]) {
	cachedScreenshots = screenshots
	for (const listener of listeners) {
		listener(screenshots)
	}
}

function notifyLoading(loading: boolean) {
	isLoading = loading
	for (const listener of loadingListeners) {
		listener(loading)
	}
}

export const screenshotService = {
	getCachedScreenshots(): ScreenshotInfo[] {
		return cachedScreenshots
	},

	isFetched(): boolean {
		return isInitialFetched
	},

	isLoading(): boolean {
		return isLoading
	},

	subscribe(
		listener: (screenshots: ScreenshotInfo[]) => void,
		loadingListener?: (loading: boolean) => void,
	): () => void {
		listeners.add(listener)
		listener(cachedScreenshots)

		if (loadingListener) {
			loadingListeners.add(loadingListener)
			loadingListener(isLoading)
		}

		return () => {
			listeners.delete(listener)
			if (loadingListener) {
				loadingListeners.delete(loadingListener)
			}
		}
	},

	async getScreenshots(): Promise<ScreenshotInfo[]> {
		return this.refreshScreenshots()
	},

	async refreshScreenshots(): Promise<ScreenshotInfo[]> {
		try {
			notifyLoading(true)
			const list = await rpc.get_all_screenshots()
			isInitialFetched = true
			notify(list)
			return list
		} catch (error) {
			console.error("Failed to load screenshots:", error)
			return cachedScreenshots
		} finally {
			notifyLoading(false)
		}
	},

	async deleteScreenshot(instanceId: string, fileName: string): Promise<void> {
		await rpc.delete_screenshot(instanceId, fileName)
		const updated = cachedScreenshots.filter(
			(s) => !(s.instanceId === instanceId && s.fileName === fileName),
		)
		notify(updated)
	},

	async openScreenshotsFolder(instanceId?: string | null): Promise<void> {
		await rpc.open_screenshots_folder(instanceId ?? null)
	},

	async revealScreenshotFile(filePath: string): Promise<void> {
		await rpc.reveal_screenshot_file(filePath)
	},

	getImageUrl(filePath: string): string {
		try {
			return convertFileSrc(filePath)
		} catch (e) {
			console.error("Failed to convert file src:", e)
			return filePath
		}
	},
}

export function useScreenshots() {
	const [screenshots, setScreenshots] = useState<ScreenshotInfo[]>(
		screenshotService.getCachedScreenshots(),
	)
	const [loading, setLoading] = useState<boolean>(screenshotService.isLoading())

	useEffect(() => {
		const unsubscribe = screenshotService.subscribe(
			(updated) => {
				setScreenshots(updated)
			},
			(isLoading) => {
				setLoading(isLoading)
			},
		)

		if (!screenshotService.isFetched()) {
			screenshotService.refreshScreenshots()
		}

		return unsubscribe
	}, [])

	return {
		screenshots,
		isLoading: loading,
		refresh: () => screenshotService.refreshScreenshots(),
		deleteScreenshot: (instanceId: string, fileName: string) =>
			screenshotService.deleteScreenshot(instanceId, fileName),
		openScreenshotsFolder: (instanceId?: string | null) =>
			screenshotService.openScreenshotsFolder(instanceId),
		revealScreenshotFile: (filePath: string) => screenshotService.revealScreenshotFile(filePath),
		getImageUrl: screenshotService.getImageUrl,
	}
}

export type { ScreenshotInfo }
