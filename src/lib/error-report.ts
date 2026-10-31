export interface DiagnosticReportOptions {
	componentStack?: string
}

export interface DeviceDetails {
	os: string
	arch: string
	cpuCores: number | string
	deviceMemory?: string
	screenResolution: string
	windowSize: string
	pixelRatio: string
	gpuRenderer?: string
	isOnline: boolean
	language: string
	appMode: "mobile" | "desktop"
	jsHeapUsed?: string
	userAgent: string
}

export const isRecord = (value: unknown): value is Record<string, unknown> => {
	return typeof value === "object" && value !== null
}

export const getErrorMessage = (error: unknown): string => {
	if (error instanceof Error) {
		return error.message.length > 0 ? error.message : error.name
	}
	if (typeof error === "string") {
		return error
	}
	if (isRecord(error) && typeof error.message === "string") {
		return error.message
	}
	return "An unexpected error occurred"
}

export const getErrorStack = (error: unknown): string | null => {
	if (error instanceof Error && typeof error.stack === "string") {
		return error.stack
	}
	if (isRecord(error) && typeof error.stack === "string") {
		return error.stack
	}
	return null
}

export const getVersionString = (): string => {
	if (typeof APP_VERSION !== "undefined" && typeof APP_VERSION === "string") {
		return APP_VERSION
	}
	return "unknown"
}

export const detectOperatingSystem = (ua: string): string => {
	const lower = ua.toLowerCase()
	if (lower.includes("windows nt 10.0")) {
		return "Windows 10/11"
	}
	if (lower.includes("windows nt 6.3")) {
		return "Windows 8.1"
	}
	if (lower.includes("windows nt 6.2")) {
		return "Windows 8"
	}
	if (lower.includes("windows nt 6.1")) {
		return "Windows 7"
	}
	if (lower.includes("windows")) {
		return "Windows"
	}
	if (lower.includes("android")) {
		return "Android"
	}
	if (lower.includes("iphone") || lower.includes("ipad") || lower.includes("ipod")) {
		return "iOS"
	}
	if (lower.includes("mac os x") || lower.includes("macos")) {
		return "macOS"
	}
	if (lower.includes("linux")) {
		return "Linux"
	}
	return "Unknown OS"
}

export const detectArchitecture = (ua: string): string => {
	const lower = ua.toLowerCase()
	if (
		lower.includes("x86_64") ||
		lower.includes("x64") ||
		lower.includes("win64") ||
		lower.includes("wow64") ||
		lower.includes("amd64")
	) {
		return "x64"
	}
	if (lower.includes("arm64") || lower.includes("aarch64")) {
		return "arm64"
	}
	if (lower.includes("armv7") || lower.includes("arm")) {
		return "arm"
	}
	if (lower.includes("i686") || lower.includes("i386") || lower.includes("x86")) {
		return "x86"
	}
	return "Unknown Arch"
}

export const getCpuCores = (): number | string => {
	if (typeof navigator !== "undefined" && typeof navigator.hardwareConcurrency === "number") {
		return navigator.hardwareConcurrency
	}
	return "Unknown"
}

export const getDeviceMemory = (): string | undefined => {
	if (typeof navigator === "undefined" || !isRecord(navigator)) {
		return undefined
	}
	const memory = navigator.deviceMemory
	if (typeof memory === "number") {
		return `${memory} GB`
	}
	return undefined
}

export const getScreenResolution = (): string => {
	if (
		typeof screen !== "undefined" &&
		typeof screen.width === "number" &&
		typeof screen.height === "number"
	) {
		return `${screen.width}x${screen.height}`
	}
	return "Unknown"
}

export const getWindowSize = (): string => {
	if (
		typeof window !== "undefined" &&
		typeof window.innerWidth === "number" &&
		typeof window.innerHeight === "number"
	) {
		return `${window.innerWidth}x${window.innerHeight}`
	}
	return "Unknown"
}

export const getPixelRatio = (): string => {
	if (typeof window !== "undefined" && typeof window.devicePixelRatio === "number") {
		return `${window.devicePixelRatio}x`
	}
	return "1x"
}

export const getGpuRenderer = (): string | undefined => {
	if (typeof document === "undefined") {
		return undefined
	}
	try {
		const canvas = document.createElement("canvas")
		const gl = canvas.getContext("webgl") ?? canvas.getContext("experimental-webgl")
		if (!gl || !isRecord(gl)) {
			return undefined
		}

		const getExtension = gl.getExtension
		if (typeof getExtension !== "function") {
			return undefined
		}

		const debugInfo = getExtension.call(gl, "WEBGL_debug_renderer_info")
		if (!debugInfo || !isRecord(debugInfo)) {
			return undefined
		}

		const getParameter = gl.getParameter
		if (typeof getParameter !== "function") {
			return undefined
		}

		const renderer = getParameter.call(gl, 0x9246)
		const vendor = getParameter.call(gl, 0x9245)

		if (typeof renderer === "string" && typeof vendor === "string") {
			return `${vendor} - ${renderer}`
		}
		if (typeof renderer === "string") {
			return renderer
		}
	} catch {
		// Context creation or extension retrieval may fail in restricted sandboxes
	}
	return undefined
}

export const getJsHeapUsed = (): string | undefined => {
	if (typeof performance === "undefined" || !isRecord(performance)) {
		return undefined
	}
	const memory = performance.memory
	if (!isRecord(memory) || typeof memory.usedJSHeapSize !== "number") {
		return undefined
	}
	const mb = Math.round(memory.usedJSHeapSize / (1024 * 1024))
	return `${mb} MB`
}

export const detectAppMode = (): "mobile" | "desktop" => {
	if (typeof window === "undefined") {
		return "desktop"
	}
	const urlParams = new URLSearchParams(window.location.search)
	if (urlParams.get("mobile") === "true") {
		return "mobile"
	}
	if (urlParams.get("desktop") === "true") {
		return "desktop"
	}

	if (typeof navigator === "undefined") {
		return "desktop"
	}
	const ua = navigator.userAgent.toLowerCase()
	const isMobileUa =
		ua.includes("android") || ua.includes("iphone") || ua.includes("ipad") || ua.includes("mobile")
	const isTauriMobile = isRecord(window) && window.__TAURI_MOBILE__ !== undefined

	if (isMobileUa || isTauriMobile) {
		return "mobile"
	}
	return "desktop"
}

export const collectDeviceDetails = (): DeviceDetails => {
	const ua = typeof navigator !== "undefined" ? navigator.userAgent : "Unknown"
	const isOnline =
		typeof navigator !== "undefined" && typeof navigator.onLine === "boolean"
			? navigator.onLine
			: true
	const language =
		typeof navigator !== "undefined" && typeof navigator.language === "string"
			? navigator.language
			: "Unknown"

	return {
		os: detectOperatingSystem(ua),
		arch: detectArchitecture(ua),
		cpuCores: getCpuCores(),
		deviceMemory: getDeviceMemory(),
		screenResolution: getScreenResolution(),
		windowSize: getWindowSize(),
		pixelRatio: getPixelRatio(),
		gpuRenderer: getGpuRenderer(),
		isOnline,
		language,
		appMode: detectAppMode(),
		jsHeapUsed: getJsHeapUsed(),
		userAgent: ua,
	}
}

export const buildDiagnosticReport = (
	error: unknown,
	options?: DiagnosticReportOptions,
): string => {
	const message = getErrorMessage(error)
	const stack = getErrorStack(error)
	const componentStack = options?.componentStack
	const timestamp = new Date().toISOString()
	const appVer = getVersionString()
	const device = collectDeviceDetails()

	const lines: string[] = [
		"Ingot Crash Report",
		"------------------",
		`Version: v${appVer}`,
		`Time: ${timestamp}`,
		"",
		"Device & Environment:",
		`- OS: ${device.os} (${device.arch})`,
		`- CPU: ${device.cpuCores} cores`,
	]

	if (device.deviceMemory) {
		lines.push(`- RAM: ${device.deviceMemory}`)
	}

	lines.push(
		`- Display: ${device.screenResolution} (Window: ${device.windowSize}, Scale: ${device.pixelRatio})`,
	)

	if (device.gpuRenderer) {
		lines.push(`- GPU: ${device.gpuRenderer}`)
	}

	if (device.jsHeapUsed) {
		lines.push(`- JS Heap: ${device.jsHeapUsed}`)
	}

	lines.push(
		`- Mode: ${device.appMode}`,
		`- Network: ${device.isOnline ? "Online" : "Offline"}`,
		`- Language: ${device.language}`,
		`- User Agent: ${device.userAgent}`,
		"",
		"Error Message:",
		message,
	)

	if (stack) {
		lines.push("", "Stack Trace:", stack)
	}

	if (componentStack) {
		lines.push("", "Component Stack:", componentStack)
	}

	return lines.join("\n")
}

export const buildGitHubIssueBody = (error: unknown, options?: DiagnosticReportOptions): string => {
	const message = getErrorMessage(error)
	const rawStack = getErrorStack(error)
	const componentStack = options?.componentStack
	const timestamp = new Date().toISOString()
	const appVer = getVersionString()
	const device = collectDeviceDetails()

	const sections: string[] = [
		"### What happened?",
		"An unexpected error occurred in Ingot:",
		"```text",
		message,
		"```",
		"",
		"### Device & Environment",
		`- **Ingot Version**: v${appVer}`,
		`- **Timestamp**: ${timestamp}`,
		`- **Operating System**: ${device.os} (${device.arch})`,
		`- **CPU Cores**: ${device.cpuCores}`,
	]

	if (device.deviceMemory) {
		sections.push(`- **Device Memory**: ${device.deviceMemory}`)
	}

	sections.push(
		`- **Display**: ${device.screenResolution} (Window: ${device.windowSize}, Scale: ${device.pixelRatio})`,
	)

	if (device.gpuRenderer) {
		sections.push(`- **GPU**: ${device.gpuRenderer}`)
	}

	if (device.jsHeapUsed) {
		sections.push(`- **JS Heap Used**: ${device.jsHeapUsed}`)
	}

	sections.push(
		`- **App Mode**: ${device.appMode}`,
		`- **Network**: ${device.isOnline ? "Online" : "Offline"}`,
		`- **Language**: ${device.language}`,
		`- **User Agent**: \`${device.userAgent}\``,
	)

	if (rawStack) {
		const trimmedStack =
			rawStack.length > 2000
				? `${rawStack.slice(0, 2000)}\n... (truncated for URL length)`
				: rawStack

		sections.push(
			"",
			"### Stack Trace",
			"<details>",
			"<summary>Click to view stack trace</summary>",
			"",
			"```text",
			trimmedStack,
			"```",
			"</details>",
		)
	}

	if (componentStack) {
		const trimmedComponentStack =
			componentStack.length > 1500
				? `${componentStack.slice(0, 1500)}\n... (truncated for URL length)`
				: componentStack

		sections.push(
			"",
			"### Component Stack",
			"<details>",
			"<summary>Click to view component stack</summary>",
			"",
			"```text",
			trimmedComponentStack,
			"```",
			"</details>",
		)
	}

	return sections.join("\n")
}

export const buildGitHubIssueUrl = (error: unknown, options?: DiagnosticReportOptions): string => {
	const message = getErrorMessage(error)
	const title = `[Bug]: ${message.slice(0, 100)}`
	const body = buildGitHubIssueBody(error, options)

	const searchParams = new URLSearchParams()
	searchParams.set("title", title)
	searchParams.set("body", body)

	return `https://github.com/0deans/ingot/issues/new?${searchParams.toString()}`
}
