import { createTauRPCProxy } from "@/bindings"

export type { ApiErrorKind } from "./rpc-result"
export { getRpcErrorMessage, isRpcError, isRpcErrorKind, RpcError, unwrapRpc } from "./rpc-result"

export const rpc = createTauRPCProxy()
