import { RouterProvider } from "@tanstack/react-router"
import { ThemeProvider } from "next-themes"
import React, { useCallback } from "react"
import ReactDOM from "react-dom/client"
import { useTauriDeepLinks } from "@/shared/hooks/useTauriDeepLinks"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import CustomThemeSync from "@/shared/theme/CustomThemeSync"
import { Toaster } from "@/shared/ui/sonner"
import { router } from "./router"
import "./styles/globals.css"

const APP_THEMES = ["light", "dark", "purple", "custom"]

const IngotApp = () => {
	const toastPosition = useLauncherPrefsStore((s) => {
		return s.toastPosition
	})

	const handlePreloadRoutes = useCallback(async () => {
		await router.load()
	}, [])

	const handleNavigatePath = useCallback((path: string) => {
		router.history.push(path)
	}, [])

	useTauriDeepLinks({
		onPreloadRoutes: handlePreloadRoutes,
		onNavigatePath: handleNavigatePath,
	})

	return (
		<ThemeProvider
			attribute="class"
			themes={APP_THEMES}
			defaultTheme="dark"
			enableSystem={false}
			storageKey="ingot-theme"
		>
			<CustomThemeSync />
			<RouterProvider router={router} />
			<Toaster position={toastPosition} />
		</ThemeProvider>
	)
}

const rootElement = document.getElementById("root")

if (!rootElement) {
	throw new Error("Root element #root not found in index.html")
}

if (!rootElement.innerHTML) {
	ReactDOM.createRoot(rootElement).render(
		<React.StrictMode>
			<IngotApp />
		</React.StrictMode>,
	)
}
