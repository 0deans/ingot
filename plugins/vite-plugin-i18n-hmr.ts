import path from "node:path"
import type { Plugin } from "vite"

export const i18nHmrPlugin = (localesDir = "public/locales"): Plugin => {
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
