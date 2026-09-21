import { defineConfig } from "bumpp"

export default defineConfig({
	all: true,
	commit: "chore: release v%s",
	tag: "v%s",
	push: true,
	execute: "node scripts/sync-version.js",
})
