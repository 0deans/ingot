import { convertFileSrc } from "@tauri-apps/api/core"
import { useEffect } from "react"
import { create } from "zustand"
import { createTauRPCProxy, type ScreenshotInfo } from "@/bindings"

export const rpc = createTauRPCProxy()

const useScreenshotStore = create<{
	screenshots: ScreenshotInfo[]
	fetched: boolean
	loading: boolean
}>(() => ({ screenshots: [], fetched: false, loading: false }))

const state = () => useScreenshotStore.getState()

export const screenshotService = {
	async getScreenshots(): Promise<ScreenshotInfo[]> {
		return this.refreshScreenshots()
	},

	async refreshScreenshots(): Promise<ScreenshotInfo[]> {
		try {
			useScreenshotStore.setState({ loading: true })
			const list = await rpc.get_all_screenshots()
			useScreenshotStore.setState({ screenshots: list, fetched: true })
			return list
		} catch (error) {
			console.error("Failed to load screenshots:", error)
			return state().screenshots
		} finally {
			useScreenshotStore.setState({ loading: false })
		}
	},

	async deleteScreenshot(instanceId: string, fileName: string): Promise<void> {
		await rpc.delete_screenshot(instanceId, fileName)
		useScreenshotStore.setState(({ screenshots }) => ({
			screenshots: screenshots.filter(
				(s) => !(s.instanceId === instanceId && s.fileName === fileName),
			),
		}))
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
	const screenshots = useScreenshotStore((s) => s.screenshots)
	const loading = useScreenshotStore((s) => s.loading)

	useEffect(() => {
		if (!state().fetched) screenshotService.refreshScreenshots()
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
