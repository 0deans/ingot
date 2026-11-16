import { isTauri } from "@tauri-apps/api/core"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { createTauRPCProxy } from "@/bindings"

export const isTauriRuntime = (): boolean => {
	return isTauri()
}

export const shouldShowWindowOnStartup = async (): Promise<boolean> => {
	if (!isTauri()) {
		return true
	}
	try {
		return await createTauRPCProxy().app.should_show_window_on_startup()
	} catch (error: unknown) {
		console.error("[Ingot] Failed to check startup window visibility:", error)
		return true
	}
}

export const showAndFocusMainWindow = async (): Promise<void> => {
	if (!isTauri()) {
		return
	}
	const appWindow = getCurrentWindow()
	await appWindow.show()
	await appWindow.setFocus()
}

export const minimizeMainWindow = async (): Promise<void> => {
	if (!isTauri()) {
		return
	}
	await getCurrentWindow().minimize()
}

export const toggleMaximizeMainWindow = async (): Promise<void> => {
	if (!isTauri()) {
		return
	}
	await getCurrentWindow().toggleMaximize()
}

export const closeMainWindow = async (): Promise<void> => {
	if (!isTauri()) {
		return
	}
	await getCurrentWindow().close()
}

export const queryMainWindowMaximized = async (): Promise<boolean> => {
	if (!isTauri()) {
		return false
	}
	return await getCurrentWindow().isMaximized()
}
