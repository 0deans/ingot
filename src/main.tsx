import { RouterProvider } from "@tanstack/react-router"
import React, { useEffect } from "react"
import ReactDOM from "react-dom/client"
import { router } from "./router"
import "./styles/globals.css"

const IngotApp = () => {
	useEffect(() => {
		const handleContextMenu = (e: MouseEvent) => {
			if (!import.meta.env.DEV) {
				e.preventDefault()
			}
		}

		document.addEventListener("contextmenu", handleContextMenu)

		let unlistenDeepLink: (() => void) | undefined

		import("@tauri-apps/plugin-deep-link")
			.then(({ onOpenUrl }) => {
				return onOpenUrl((urls) => {
					for (const rawUrl of urls) {
						try {
							const parsed = new URL(rawUrl)
							const targetPath = `/${parsed.host}${parsed.pathname}`.replace(/\/+$/, "") || "/"

							router.navigate({
								to: targetPath as never,
								search: Object.fromEntries(parsed.searchParams.entries()) as never,
							})
						} catch (err) {
							console.error("[Ingot] Invalid deep-link URL:", rawUrl, err)
						}
					}
				})
			})
			.then((unlisten) => {
				unlistenDeepLink = unlisten
			})
			.catch(() => {
				// Safely ignored when running in browser preview without Tauri runtime
			})

		return () => {
			document.removeEventListener("contextmenu", handleContextMenu)
			unlistenDeepLink?.()
		}
	}, [])

	return <RouterProvider router={router} />
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
