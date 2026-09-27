import fs from "node:fs"
import path from "node:path"
import type { HmrContext, Plugin, ViteDevServer } from "vite"

export interface LocaleMeta {
	code: string
	name: string
	nativeName: string
}

export interface I18nHmrPayload {
	type: "update" | "add" | "remove"
	lang: string
	resources?: Record<string, unknown>
	availableLocales: LocaleMeta[]
}

function resolveDisplayName(code: string, targetLocale: string): string {
	try {
		const displayNames = new Intl.DisplayNames([targetLocale], { type: "language" })
		const resolved = displayNames.of(code)
		if (resolved) {
			return resolved.charAt(0).toUpperCase() + resolved.slice(1)
		}
	} catch {
		// Ignore invalid locale tag
	}
	return code.toUpperCase()
}

function extractLocaleMeta(code: string, data: Record<string, unknown>): LocaleMeta {
	const meta =
		typeof data._meta === "object" && data._meta !== null
			? (data._meta as Record<string, unknown>)
			: undefined

	const name =
		typeof meta?.name === "string" && meta.name.trim().length > 0
			? meta.name.trim()
			: resolveDisplayName(code, "en")

	const nativeName =
		typeof meta?.nativeName === "string" && meta.nativeName.trim().length > 0
			? meta.nativeName.trim()
			: resolveDisplayName(code, code)

	return { code, name, nativeName }
}

function flattenKeys(obj: Record<string, unknown>, prefix = ""): string[] {
	const keys: string[] = []
	for (const [key, value] of Object.entries(obj)) {
		if (key === "_meta") continue
		const nextKey = prefix ? `${prefix}.${key}` : key
		if (typeof value === "object" && value !== null && !Array.isArray(value)) {
			keys.push(...flattenKeys(value as Record<string, unknown>, nextKey))
		} else {
			keys.push(nextKey)
		}
	}
	return keys
}

function discoverAvailableLocales(localesDir: string): LocaleMeta[] {
	if (!fs.existsSync(localesDir)) return []

	const entries = fs.readdirSync(localesDir, { withFileTypes: true })
	const locales: LocaleMeta[] = []

	for (const entry of entries) {
		if (!entry.isFile() || !entry.name.endsWith(".json")) continue
		const code = path.basename(entry.name, ".json")
		const filePath = path.join(localesDir, entry.name)
		try {
			const raw = fs.readFileSync(filePath, "utf-8")
			const parsed = JSON.parse(raw) as Record<string, unknown>
			locales.push(extractLocaleMeta(code, parsed))
		} catch {
			locales.push({
				code,
				name: resolveDisplayName(code, "en"),
				nativeName: resolveDisplayName(code, code),
			})
		}
	}

	return locales.sort((a, b) => {
		if (a.code === "en") return -1
		if (b.code === "en") return 1
		return a.name.localeCompare(b.name)
	})
}

function checkMissingKeys(
	localesDir: string,
	lang: string,
	targetResources: Record<string, unknown>,
	server: ViteDevServer,
) {
	if (lang === "en") return
	const enPath = path.join(localesDir, "en.json")
	if (!fs.existsSync(enPath)) return

	try {
		const enRaw = fs.readFileSync(enPath, "utf-8")
		const enJson = JSON.parse(enRaw) as Record<string, unknown>
		const baseKeys = flattenKeys(enJson)
		const targetKeySet = new Set(flattenKeys(targetResources))
		const missing = baseKeys.filter((k) => !targetKeySet.has(k))

		if (missing.length > 0) {
			const preview = missing.slice(0, 5).join(", ")
			const more = missing.length > 5 ? ` (+${missing.length - 5} more)` : ""
			server.config.logger.warn(
				`[i18n-hmr] Locale "${lang}.json" is missing ${missing.length} key(s) from en.json: ${preview}${more}`,
			)
		}
	} catch {
		// Ignore if en.json is temporarily invalid during editing
	}
}

export function i18nHmrPlugin(options?: { localesDir?: string }): Plugin {
	let resolvedLocalesDir = ""

	const isLocaleFile = (filePath: string): boolean => {
		const normalizedFile = path.normalize(filePath)
		const normalizedDir = path.normalize(resolvedLocalesDir)
		return (
			normalizedFile.startsWith(normalizedDir + path.sep) &&
			normalizedFile.endsWith(".json") &&
			path.dirname(normalizedFile) === normalizedDir
		)
	}

	return {
		name: "vite-plugin-i18n-hmr",

		configResolved(config) {
			resolvedLocalesDir = path.resolve(config.root, options?.localesDir ?? "src/locales")
		},

		configureServer(server) {
			if (fs.existsSync(resolvedLocalesDir)) {
				server.watcher.add(resolvedLocalesDir)
			}

			const broadcastAddOrUpdate = (filePath: string, type: "add" | "update") => {
				if (!isLocaleFile(filePath)) return
				const lang = path.basename(filePath, ".json")

				try {
					const raw = fs.readFileSync(filePath, "utf-8")
					const resources = JSON.parse(raw) as Record<string, unknown>
					checkMissingKeys(resolvedLocalesDir, lang, resources, server)

					const availableLocales = discoverAvailableLocales(resolvedLocalesDir)
					const payload: I18nHmrPayload = {
						type,
						lang,
						resources,
						availableLocales,
					}

					server.ws.send({
						type: "custom",
						event: "i18n:locale-update",
						data: payload,
					})

					server.config.logger.info(
						`[i18n-hmr] ${type === "add" ? "Added" : "Hot-reloaded"} locale "${lang}.json"`,
						{ timestamp: true },
					)
				} catch (error) {
					const message = error instanceof Error ? error.message : String(error)
					server.config.logger.error(`[i18n-hmr] Failed to parse "${lang}.json": ${message}`, {
						timestamp: true,
					})
				}
			}

			server.watcher.on("add", (filePath) => {
				broadcastAddOrUpdate(filePath, "add")
			})

			server.watcher.on("unlink", (filePath) => {
				if (!isLocaleFile(filePath)) return
				const lang = path.basename(filePath, ".json")
				const availableLocales = discoverAvailableLocales(resolvedLocalesDir)

				const payload: I18nHmrPayload = {
					type: "remove",
					lang,
					availableLocales,
				}

				server.ws.send({
					type: "custom",
					event: "i18n:locale-update",
					data: payload,
				})

				server.config.logger.info(`[i18n-hmr] Removed locale "${lang}.json"`, {
					timestamp: true,
				})
			})
		},

		handleHotUpdate(ctx: HmrContext) {
			if (!isLocaleFile(ctx.file)) return

			const lang = path.basename(ctx.file, ".json")
			try {
				const raw = fs.readFileSync(ctx.file, "utf-8")
				const resources = JSON.parse(raw) as Record<string, unknown>
				checkMissingKeys(resolvedLocalesDir, lang, resources, ctx.server)

				const availableLocales = discoverAvailableLocales(resolvedLocalesDir)
				const payload: I18nHmrPayload = {
					type: "update",
					lang,
					resources,
					availableLocales,
				}

				ctx.server.ws.send({
					type: "custom",
					event: "i18n:locale-update",
					data: payload,
				})

				ctx.server.config.logger.info(`[i18n-hmr] Hot-reloaded locale "${lang}.json"`, {
					timestamp: true,
				})
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error)
				ctx.server.config.logger.error(`[i18n-hmr] Invalid JSON in "${lang}.json": ${message}`, {
					timestamp: true,
				})
			}

			// Return empty array to prevent full page reload and keep React state intact
			return []
		},
	}
}
