import path from "node:path"
import process from "node:process"
import tailwindcss from "@tailwindcss/vite"
import { tanstackRouter } from "@tanstack/router-plugin/vite"
import react from "@vitejs/plugin-react"
import { defineConfig, type Plugin } from "vite"
import pkg from "./package.json" with { type: "json" }

const i18nHmrPlugin = (localesDir = "public/locales"): Plugin => {
	return {
		name: "vite-plugin-i18n-hmr",
		handleHotUpdate({ file, server }) {
			if (!file.endsWith(".json")) return

			const dir = path.resolve(server.config.root, localesDir)
			if (path.dirname(path.resolve(file)) !== dir) return

			server.ws.send("i18n:update", { lang: path.basename(file, ".json") })
			return []
		},
	}
}

const host = process.env.TAURI_DEV_HOST
// Over USB (`--host 127.0.0.1`) tauri only runs `adb reverse` for port 1420, so HMR
// must share that port; a separate 1421 socket would be unreachable from the phone.
const isLoopback = host === "127.0.0.1" || host === "localhost"

// https://vite.dev/config/
export default defineConfig(() => ({
	define: {
		APP_VERSION: JSON.stringify(pkg.version),
	},
	plugins: [
		tanstackRouter({
			target: "react",
			autoCodeSplitting: true,
		}),
		react(),
		tailwindcss(),
		i18nHmrPlugin(),
	],
	resolve: {
		alias: {
			"@": path.resolve(import.meta.dirname, "./src"),
		},
	},

	// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
	//
	// 1. prevent Vite from obscuring rust errors
	clearScreen: false,
	// 2. tauri expects a fixed port, fail if that port is not available
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr:
			host && !isLoopback
				? {
						protocol: "ws",
						host,
						port: 1421,
					}
				: undefined,
		watch: {
			// 3. tell Vite to ignore watching `src-tauri`
			ignored: ["**/src-tauri/**"],
		},
	},
	build: {
		chunkSizeWarningLimit: 1000,
	},
}))
