import { getCurrentWindow } from "@tauri-apps/api/window"
import { useSyncExternalStore } from "react"

export type ThemeMode = "auto" | "dark" | "light"
export type ResolvedTheme = "dark" | "light"

export const THEME_STORAGE_KEY = "ingot:theme"

const listeners = new Set<() => void>()

function notifyListeners() {
	for (const listener of listeners) {
		listener()
	}
}

function getSystemTheme(): ResolvedTheme {
	if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
		return "dark"
	}
	return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark"
}

function readStoredTheme(): ThemeMode {
	try {
		const raw = localStorage.getItem(THEME_STORAGE_KEY)
		if (raw === "auto" || raw === "dark" || raw === "light") {
			return raw
		}
	} catch {
		// Ignore storage errors
	}
	return "auto"
}

let currentThemeMode: ThemeMode = readStoredTheme()
let currentResolvedTheme: ResolvedTheme =
	currentThemeMode === "auto" ? getSystemTheme() : currentThemeMode

function applyThemeToDom(mode: ThemeMode, resolved: ResolvedTheme) {
	if (typeof document === "undefined") return

	const root = document.documentElement
	root.classList.toggle("dark", resolved === "dark")
	root.classList.toggle("light", resolved === "light")
	root.dataset.theme = resolved
	root.dataset.themeMode = mode

	const bgColor = resolved === "dark" ? "#09090b" : "#fafafa"
	const fgColor = resolved === "dark" ? "#ffffff" : "#09090b"
	root.style.backgroundColor = bgColor
	root.style.colorScheme = resolved

	if (document.body) {
		document.body.style.backgroundColor = bgColor
		document.body.style.color = fgColor
	}

	const isTauri =
		typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
	if (isTauri) {
		try {
			getCurrentWindow()
				.setTheme(mode === "auto" ? null : mode)
				.catch(() => {})
		} catch {
			// Ignore window theme errors on unsupported platforms
		}
	}
}

export function setThemeMode(mode: ThemeMode) {
	currentThemeMode = mode
	currentResolvedTheme = mode === "auto" ? getSystemTheme() : mode
	try {
		localStorage.setItem(THEME_STORAGE_KEY, mode)
	} catch {
		// Ignore storage errors
	}
	applyThemeToDom(currentThemeMode, currentResolvedTheme)
	notifyListeners()
}

export function getThemeMode(): ThemeMode {
	return currentThemeMode
}

export function getResolvedTheme(): ResolvedTheme {
	return currentResolvedTheme
}

// Initialize DOM and media query listener immediately
if (typeof window !== "undefined") {
	applyThemeToDom(currentThemeMode, currentResolvedTheme)

	if (typeof window.matchMedia === "function") {
		const mediaQuery = window.matchMedia("(prefers-color-scheme: light)")
		const handleChange = () => {
			if (currentThemeMode === "auto") {
				const nextResolved = getSystemTheme()
				if (nextResolved !== currentResolvedTheme) {
					currentResolvedTheme = nextResolved
					applyThemeToDom(currentThemeMode, currentResolvedTheme)
					notifyListeners()
				}
			}
		}
		mediaQuery.addEventListener("change", handleChange)
	}
}

function subscribeTheme(listener: () => void) {
	listeners.add(listener)
	return () => {
		listeners.delete(listener)
	}
}

export function useTheme() {
	const themeMode = useSyncExternalStore(subscribeTheme, getThemeMode, getThemeMode)
	const resolvedTheme = useSyncExternalStore(subscribeTheme, getResolvedTheme, getResolvedTheme)

	return {
		themeMode,
		resolvedTheme,
		setThemeMode,
	}
}
