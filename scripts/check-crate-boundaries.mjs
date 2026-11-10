import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const repoRoot = path.resolve(__dirname, "..")
const srcTauriSrc = path.join(repoRoot, "src-tauri", "src")
const srcTauriCrates = path.join(repoRoot, "src-tauri", "crates")

const moduleToCrate = {
	core: "core",
	process_tree: "core",
	settings: "settings",
	account: "accounts",
	auth: "accounts",
	keyring_store: "accounts",
	artifacts: "artifacts",
	minecraft: "minecraft",
	server: "server",
	version_change: "version_change",
	app: "root",
	ipc: "root",
	system: "root",
	running: "root",
	tray: "root",
	lib: "root",
	main: "root",
	ipc_contract: "root",
}

const allowedDependencies = {
	core: new Set(),
	settings: new Set(["core"]),
	artifacts: new Set(["core"]),
	accounts: new Set(["core"]),
	minecraft: new Set(["core", "settings", "artifacts", "accounts"]),
	server: new Set(["core", "artifacts"]),
	version_change: new Set(["core", "artifacts", "minecraft", "server"]),
	root: new Set([
		"core",
		"settings",
		"artifacts",
		"accounts",
		"minecraft",
		"server",
		"version_change",
	]),
}

// Couplings identified in workspace_split_plan.md to be removed in W2-W4
const knownViolations = new Set([
	// C1, C4, C5: Domain calling app / tauri helpers
	"accounts->root",
	"minecraft->root",
	"server->root",
	"version_change->root",
	"settings->root",
	// C2, C8: server using minecraft (metadata, downloader, java, instance_content)
	"server->minecraft",
	// C8, C9, C10: minecraft using version_change
	"minecraft->version_change",
	// C9: server using version_change (crash, plugin tracking models) to move to artifacts in W3
	"server->version_change",
	// C3: minecraft using running (pure process identity)
	"minecraft->root",
])

const scanDirectory = (dir, files = []) => {
	if (!fs.existsSync(dir)) {
		return files
	}
	const entries = fs.readdirSync(dir, { withFileTypes: true })
	for (const entry of entries) {
		const fullPath = path.join(dir, entry.name)
		if (entry.isDirectory()) {
			scanDirectory(fullPath, files)
		} else if (entry.isFile() && entry.name.endsWith(".rs")) {
			files.push(fullPath)
		}
	}
	return files
}

const checkBoundaries = () => {
	const sourceFiles = scanDirectory(srcTauriSrc)
	if (fs.existsSync(srcTauriCrates)) {
		scanDirectory(srcTauriCrates, sourceFiles)
	}

	const observedDisallowed = new Map()

	for (const filePath of sourceFiles) {
		let relative = path.relative(srcTauriSrc, filePath)
		let sourceModule = relative.split(path.sep)[0]
		if (relative.startsWith("..")) {
			// Inside crates/<name>/src/...
			const relFromCrates = path.relative(srcTauriCrates, filePath)
			const crateName = relFromCrates.split(path.sep)[0]
			sourceModule = crateName
		}
		if (sourceModule.endsWith(".rs")) {
			sourceModule = sourceModule.slice(0, -3)
		}

		const sourceCrate = moduleToCrate[sourceModule] || "root"
		const allowed = allowedDependencies[sourceCrate] || new Set()

		const content = fs.readFileSync(filePath, "utf-8")
		const lines = content.split("\n")

		for (let i = 0; i < lines.length; i++) {
			const line = lines[i]
			// Look for crate::<module> references
			const matches = line.matchAll(/crate::([a-zA-Z0-9_]+)/g)
			for (const match of matches) {
				const targetModule = match[1]
				if (targetModule === sourceModule) {
					continue
				}
				const targetCrate = moduleToCrate[targetModule]
				if (!targetCrate || targetCrate === sourceCrate) {
					continue
				}
				if (!allowed.has(targetCrate)) {
					const key = `${sourceCrate}->${targetCrate}`
					if (!observedDisallowed.has(key)) {
						observedDisallowed.set(key, [])
					}
					observedDisallowed.get(key).push(`${relative}:${i + 1}`)
				}
			}
		}
	}

	let hasUnexpectedViolations = false
	for (const [key, occurrences] of observedDisallowed.entries()) {
		if (!knownViolations.has(key)) {
			console.error(
				`[check-crate-boundaries] UNEXPECTED VIOLATION: ${key} (${occurrences.length} occurrences)`,
			)
			for (const sample of occurrences.slice(0, 5)) {
				console.error(`  - ${sample}`)
			}
			hasUnexpectedViolations = true
		}
	}

	if (hasUnexpectedViolations) {
		console.error("[check-crate-boundaries] FAILED: Found disallowed crate dependencies.")
		process.exit(1)
	} else {
		console.log(
			`[check-crate-boundaries] OK: All ${observedDisallowed.size} cross-boundary couplings match known migration allowlist.`,
		)
	}
}

checkBoundaries()
