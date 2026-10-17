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
	if (error instanceof Error) {
		return error.message
	}
	if (typeof error === "string") {
		return error
	}
	return String(error)
}

export function unwrapRpc<T>(result: TauRpcResult<T, ApiError>): T
export function unwrapRpc<T>(promise: Promise<TauRpcResult<T, ApiError>>): Promise<T>
export function unwrapRpc<T>(
	input: TauRpcResult<T, ApiError> | Promise<TauRpcResult<T, ApiError>>,
): T | Promise<T> {
	if (input instanceof Promise) {
		return input.then((res) => {
			if (res.status === "error") {
				throw new RpcError(res.error)
			}
			return res.data
		})
	}
	if (input.status === "error") {
		throw new RpcError(input.error)
	}
	return input.data
}
