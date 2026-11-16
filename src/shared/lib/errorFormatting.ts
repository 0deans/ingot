export type ErrorScope = "route" | "shell" | "root"

export interface ExtractedErrorDetails {
	name: string
	message: string
	summary: string
	stack: string | undefined
	isBackendLike: boolean
}

const BACKEND_ERROR_PATTERN =
	/\b(tauri|ipc|rpc|invoke|backend|network|timeout|timed out|failed to fetch|econnrefused)\b/i

const readErrorObjectField = (value: object, key: string): string | undefined => {
	if (!(key in value)) {
		return undefined
	}
	const entry = Object.getOwnPropertyDescriptor(value, key)?.value
	if (typeof entry !== "string") {
		return undefined
	}
	const trimmed = entry.trim()
	return trimmed.length > 0 ? trimmed : undefined
}

export const extractErrorDetails = (error: unknown): ExtractedErrorDetails => {
	if (error instanceof Error) {
		const name = error.name.trim() || "Error"
		const message = error.message.trim() || "An unexpected error occurred."
		const summary = message.startsWith(`${name}:`) ? message : `${name}: ${message}`
		const stack = error.stack?.trim()
		return {
			name,
			message,
			summary,
			stack: stack && stack.length > 0 ? stack : undefined,
			isBackendLike: BACKEND_ERROR_PATTERN.test(summary),
		}
	}

	if (typeof error === "string") {
		const trimmed = error.trim()
		const message = trimmed.length > 0 ? trimmed : "An unexpected error occurred."
		return {
			name: "Error",
			message,
			summary: message,
			stack: undefined,
			isBackendLike: BACKEND_ERROR_PATTERN.test(message),
		}
	}

	if (typeof error === "object" && error !== null) {
		const name = readErrorObjectField(error, "name") ?? "Error"
		const message =
			readErrorObjectField(error, "message") ??
			readErrorObjectField(error, "reason") ??
			"An unexpected error occurred."
		const stack = readErrorObjectField(error, "stack")
		const summary = message.startsWith(`${name}:`) ? message : `${name}: ${message}`
		return {
			name,
			message,
			summary,
			stack,
			isBackendLike: BACKEND_ERROR_PATTERN.test(summary),
		}
	}

	return {
		name: "Error",
		message: "An unexpected error occurred.",
		summary: "An unexpected error occurred.",
		stack: undefined,
		isBackendLike: false,
	}
}

export const getSafeAppVersion = (): string | undefined => {
	return typeof APP_VERSION === "string" && APP_VERSION.length > 0 ? APP_VERSION : undefined
}

interface DiagnosticReportInput {
	details: ExtractedErrorDetails
	scope: ErrorScope
	routePath?: string
}

export const buildErrorDiagnosticReport = ({
	details,
	scope,
	routePath,
}: DiagnosticReportInput): string => {
	const appVersion = getSafeAppVersion()
	const lines: string[] = [
		`Ingot Diagnostic Report`,
		`Timestamp: ${new Date().toISOString()}`,
		`Scope: ${scope}`,
	]

	if (appVersion) {
		lines.push(`App Version: v${appVersion}`)
	}

	if (routePath) {
		lines.push(`Route: ${routePath}`)
	}

	lines.push(`Error: ${details.summary}`)

	if (details.stack) {
		lines.push("", "Stack Trace:", details.stack)
	}

	return lines.join("\n")
}
