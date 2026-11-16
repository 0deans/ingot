import { isTauri } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"

export const subscribeWindowResized = async (onResize: () => void): Promise<() => void> => {
	if (!isTauri()) {
		return () => {}
	}
	return await getCurrentWindow().onResized(() => {
		onResize()
	})
}

export const subscribeDeepLinkUrls = async (
	onUrls: (urls: string[]) => void,
): Promise<() => void> => {
	if (!isTauri()) {
		return () => {}
	}
	const { onOpenUrl } = await import("@tauri-apps/plugin-deep-link")
	return await onOpenUrl((urls) => {
		onUrls(urls)
	})
}

export const subscribeTauriEvent = async <TPayload>(
	eventName: string,
	handler: (payload: TPayload) => void,
): Promise<() => void> => {
	if (!isTauri()) {
		return () => {}
	}
	return await listen<TPayload>(eventName, (event) => {
		handler(event.payload)
	})
}
