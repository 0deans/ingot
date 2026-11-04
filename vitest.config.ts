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
			include: ["tests/**/*.test.tsx", "tests/**/*.test.ts"],
			exclude: ["tests/**/*.test.mjs", "node_modules/**"],
		},
	}
})
