import path from "node:path"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vitest/config"

export default defineConfig(() => {
	return {
		plugins: [react()],
		resolve: {
			alias: {
				"@": path.resolve(import.meta.dirname, "./src"),
			},
		},
		test: {
			environment: "jsdom",
			globals: true,
			setupFiles: ["./tests/setup.ts"],
			include: ["**/*.test.{ts,tsx}"],
			exclude: ["node_modules/**", "dist/**", "src-tauri/**"],
		},
	}
})
