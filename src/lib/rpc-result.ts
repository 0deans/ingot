import type { ApiError, TauRpcResult } from "@/bindings"

export class RpcError extends Error {
	readonly apiError: ApiError

	constructor(apiError: ApiError) {
		super(apiError.message)
		this.name = "RpcError"
		this.apiError = apiError
	}
}

export type ApiErrorKind = ApiError["kind"]

export const isRpcError = (error: unknown): error is RpcError => {
	return error instanceof RpcError
}

const isObject = (value: unknown): value is Record<string, unknown> => {
	return typeof value === "object" && value !== null
}

const isApiErrorKind = (kind: string): kind is ApiErrorKind => {
	return (
		kind === "invalidInput" ||
		kind === "notFound" ||
		kind === "conflict" ||
		kind === "unauthorized" ||
		kind === "cancelled" ||
		kind === "providerUnavailable" ||
		kind === "storageFailure" ||
		kind === "incompleteRecovery" ||
		kind === "internal"
	)
}

export const isApiError = (error: unknown): error is ApiError => {
	if (!isObject(error)) {
		return false
	}
	if (!("kind" in error) || !("message" in error)) {
		return false
	}
	return (
		typeof error.message === "string" &&
		typeof error.kind === "string" &&
		isApiErrorKind(error.kind)
	)
}

export const isRpcErrorKind = <K extends ApiErrorKind>(
	error: unknown,
	kind: K,
): error is RpcError & { apiError: Extract<ApiError, { kind: K }> } => {
	return isRpcError(error) && error.apiError.kind === kind
}

export const getRpcErrorMessage = (error: unknown): string => {
	if (isRpcError(error)) {
		return error.apiError.message
	}
	if (isApiError(error)) {
		return error.message
	}
	if (error instanceof Error) {
		return error.message
	}
	if (typeof error === "string") {
		return error
	}
	return String(error)
}

const isTauRpcResult = <T>(value: unknown): value is TauRpcResult<T, ApiError> => {
	if (!isObject(value)) {
		return false
	}
	if (!("status" in value)) {
		return false
	}
	if (value.status === "ok") {
		return "data" in value
	}
	if (value.status === "error") {
		return "error" in value && isApiError(value.error)
	}
	return false
}

const unwrapResult = <T>(res: TauRpcResult<T, ApiError>): T => {
	if (res.status === "error") {
		throw new RpcError(res.error)
	}
	return res.data
}

export function unwrapRpc<T>(result: TauRpcResult<T, ApiError>): T
export function unwrapRpc<T>(promise: Promise<TauRpcResult<T, ApiError>>): Promise<T>
export function unwrapRpc<T>(promise: Promise<T>): Promise<T>
export function unwrapRpc<T>(value: T): T
export function unwrapRpc<T>(
	input: Promise<TauRpcResult<T, ApiError>> | TauRpcResult<T, ApiError> | Promise<T> | T,
): T | Promise<T> {
	if (input instanceof Promise) {
		return input
			.then((res) => {
				if (isTauRpcResult<T>(res)) {
					return unwrapResult(res)
				}
				return res
			})
			.catch((err: unknown) => {
				if (isRpcError(err)) {
					throw err
				}
				if (isApiError(err)) {
					throw new RpcError(err)
				}
				throw err
			})
	}
	if (isTauRpcResult<T>(input)) {
		return unwrapResult(input)
	}
	return input
}
