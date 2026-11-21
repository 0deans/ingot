import { type ApiError } from "@/bindings"

const API_ERROR_KINDS = new Set<ApiError["kind"]>([
	"invalidInput",
	"notFound",
	"conflict",
	"unauthorized",
	"cancelled",
	"providerUnavailable",
	"storageFailure",
	"incompleteRecovery",
	"internal",
])

const isApiErrorKind = (value: unknown): value is ApiError["kind"] => {
	if (typeof value !== "string") {
		return false
	}
	for (const kind of API_ERROR_KINDS) {
		if (kind === value) {
			return true
		}
	}
	return false
}

export const isApiError = (error: unknown): error is ApiError => {
	if (typeof error !== "object" || error === null) {
		return false
	}
	if (!("kind" in error) || !("message" in error)) {
		return false
	}
	return isApiErrorKind(error.kind) && typeof error.message === "string"
}

export const extractApiError = (error: unknown): ApiError | null => {
	if (isApiError(error)) {
		return error
	}
	return null
}

export const getApiErrorMessage = (error: unknown, fallbackMessage: string): string => {
	if (isApiError(error)) {
		const trimmed = error.message.trim()
		return trimmed.length > 0 ? trimmed : fallbackMessage
	}
	if (error instanceof Error) {
		const trimmed = error.message.trim()
		return trimmed.length > 0 ? trimmed : fallbackMessage
	}
	if (typeof error === "string") {
		const trimmed = error.trim()
		return trimmed.length > 0 ? trimmed : fallbackMessage
	}
	return fallbackMessage
}
