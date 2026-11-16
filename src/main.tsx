import { RouterProvider } from "@tanstack/react-router"
import { ThemeProvider } from "next-themes"
import React, { useCallback, useEffect } from "react"
import ReactDOM from "react-dom/client"
import ErrorBoundary from "@/shared/components/ErrorBoundary"
import RootErrorFallback from "@/shared/components/RootErrorFallback"
import { useTauriDeepLinks } from "@/shared/hooks/useTauriDeepLinks"
import { registerNotificationNavigator } from "@/shared/lib/notify"
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

	useEffect(() => {
		registerNotificationNavigator(handleNavigatePath)
		return () => {
			registerNotificationNavigator(null)
		}
	}, [handleNavigatePath])

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
			<ErrorBoundary
				fallback={({ error, reset }) => {
					return <RootErrorFallback error={error} reset={reset} />
				}}
			>
				<CustomThemeSync />
				<RouterProvider router={router} />
				<Toaster position={toastPosition} />
			</ErrorBoundary>
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
