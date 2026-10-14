import { convertFileSrc } from "@tauri-apps/api/core"
import { useEffect } from "react"
import { create } from "zustand"
import type { ScreenshotInfo } from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"

const useScreenshotStore = create<{
	screenshots: ScreenshotInfo[]
	fetched: boolean
	loading: boolean
}>(() => {
	return { screenshots: [], fetched: false, loading: false }
})

const state = () => {
	return useScreenshotStore.getState()
}

let refreshScreenshotsPromise: Promise<ScreenshotInfo[]> | null = null

export const loadedScreenshotImageCache = new Set<string>()

const urlCache = new Map<string, string>()

export const screenshotService = {
	async getScreenshots(): Promise<ScreenshotInfo[]> {
		return this.refreshScreenshots()
	},

	async refreshScreenshots(): Promise<ScreenshotInfo[]> {
		if (refreshScreenshotsPromise) {
			return refreshScreenshotsPromise
		}

		refreshScreenshotsPromise = (async () => {
			try {
				useScreenshotStore.setState({ loading: true })
				const list = await unwrapRpc(rpc.instances.screenshots.get_all_screenshots())
				useScreenshotStore.setState({ screenshots: list, fetched: true })
				return list
			} catch (error) {
				console.error("Failed to load screenshots:", error)
				return state().screenshots
			} finally {
				useScreenshotStore.setState({ loading: false })
				refreshScreenshotsPromise = null
			}
		})()

		return refreshScreenshotsPromise
	},

	async deleteScreenshot(instanceId: string, fileName: string): Promise<void> {
		await unwrapRpc(rpc.instances.screenshots.delete_screenshot(instanceId, fileName))
		useScreenshotStore.setState(({ screenshots }) => {
			return {
				screenshots: screenshots.filter((s) => {
					return !(s.instanceId === instanceId && s.fileName === fileName)
				}),
			}
		})
	},

	async openScreenshotsFolder(instanceId?: string | null): Promise<void> {
		await unwrapRpc(rpc.instances.screenshots.open_screenshots_folder(instanceId ?? null))
	},

	async revealScreenshotFile(filePath: string): Promise<void> {
		await unwrapRpc(rpc.instances.screenshots.reveal_screenshot_file(filePath))
	},

	getImageUrl(filePath: string): string {
		let cached = urlCache.get(filePath)
		if (!cached) {
			try {
				cached = convertFileSrc(filePath)
			} catch (e) {
				console.error("Failed to convert file src:", e)
				cached = filePath
			}
			urlCache.set(filePath, cached)
		}
		return cached
	},

	getThumbnailUrl(screenshot: ScreenshotInfo): string {
		const targetPath = screenshot.thumbnailPath || screenshot.filePath
		return this.getImageUrl(targetPath)
	},

	async ensureThumbnail(screenshot: ScreenshotInfo): Promise<string> {
		if (screenshot.thumbnailPath) {
			return this.getImageUrl(screenshot.thumbnailPath)
		}
		try {
			const thumbPath = await unwrapRpc(
				rpc.instances.screenshots.get_screenshot_thumbnail(
					screenshot.instanceId,
					screenshot.fileName,
					screenshot.filePath,
					screenshot.modifiedAt,
				),
			)
			screenshot.thumbnailPath = thumbPath
			return this.getImageUrl(thumbPath)
		} catch {
			return this.getImageUrl(screenshot.filePath)
		}
	},

	preloadThumbnails(screenshots: ScreenshotInfo[], limit = 30): void {
		if (typeof window === "undefined") return
		const slice = screenshots.slice(0, limit)
		for (const s of slice) {
			const url = this.getThumbnailUrl(s)
			if (loadedScreenshotImageCache.has(url)) continue
			const img = new Image()
			img.onload = () => {
				loadedScreenshotImageCache.add(url)
			}
			img.src = url
		}
	},
}

export function useScreenshots() {
	const screenshots = useScreenshotStore((s) => {
		return s.screenshots
	})
	const loading = useScreenshotStore((s) => {
		return s.loading
	})

	useEffect(() => {
		if (!state().fetched) {
			screenshotService.refreshScreenshots()
		}
	}, [])

	return {
		screenshots,
		isLoading: loading,
		refresh: () => {
			return screenshotService.refreshScreenshots()
		},
		deleteScreenshot: (instanceId: string, fileName: string) => {
			return screenshotService.deleteScreenshot(instanceId, fileName)
		},
		openScreenshotsFolder: (instanceId?: string | null) => {
			return screenshotService.openScreenshotsFolder(instanceId)
		},
		revealScreenshotFile: (filePath: string) => {
			return screenshotService.revealScreenshotFile(filePath)
		},
		getImageUrl: screenshotService.getImageUrl,
	}
}

export type { ScreenshotInfo }
export { rpc }
