import { createTauRPCProxy } from "@/bindings"

/** All services and direct callers share the same typed backend proxy. */
export const rpc = createTauRPCProxy()
