import { QueryClientProvider } from "@tanstack/react-query"
import { RouterProvider } from "@tanstack/react-router"
import React, { useCallback, useEffect } from "react"
import ReactDOM from "react-dom/client"
import { useShallow } from "zustand/react/shallow"
import { queryClient, useBackendEventBridge } from "@/shared/api"
import ErrorBoundary from "@/shared/components/errors/ErrorBoundary"
import RootErrorFallback from "@/shared/components/errors/RootErrorFallback"
import { useTauriDeepLinks } from "@/shared/hooks/useTauriDeepLinks"
import { initI18n, syncI18nLanguage } from "@/shared/i18n/i18n"
import { detectSystemLocale, resolveEffectiveLocale } from "@/shared/i18n/languages"
import { registerNotificationNavigator } from "@/shared/lib/notify"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { applyAppTheme } from "@/shared/theme/applyCustomTheme"
import { Toaster } from "@/shared/ui/sonner"
import { router } from "./router"
import "./styles/globals.css"

const IngotApp = () => {
	const { theme, customTheme, language, toastPosition } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				theme: s.theme,
				customTheme: s.customTheme,
				language: s.language,
				toastPosition: s.toastPosition,
			}
		}),
	)

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

	useEffect(() => {
		applyAppTheme(theme, customTheme)
	}, [customTheme, theme])

	useEffect(() => {
		const effectiveLocale = resolveEffectiveLocale(language, detectSystemLocale())
		void syncI18nLanguage(effectiveLocale).catch((error: unknown) => {
			console.error("[Ingot] Failed to sync i18n language:", error)
		})
	}, [language])

	useBackendEventBridge()

	useTauriDeepLinks({
		onPreloadRoutes: handlePreloadRoutes,
		onNavigatePath: handleNavigatePath,
	})

	return (
		<ErrorBoundary
			fallback={({ error, reset }) => {
				return <RootErrorFallback error={error} reset={reset} />
			}}
		>
			<RouterProvider router={router} />
			<Toaster position={toastPosition} />
		</ErrorBoundary>
	)
}

const rootElement = document.getElementById("root")

if (!rootElement) {
	throw new Error("Root element #root not found in index.html")
}

const renderApp = async () => {
	try {
		await initI18n()
	} catch (error: unknown) {
		console.error("[Ingot] Failed to initialize i18n:", error)
	}

	if (!rootElement.innerHTML) {
		ReactDOM.createRoot(rootElement).render(
			<React.StrictMode>
				<QueryClientProvider client={queryClient}>
					<IngotApp />
				</QueryClientProvider>
			</React.StrictMode>,
		)
	}
}

void renderApp()
