import { createRootRoute } from "@tanstack/react-router"
import AppShell from "@/features/shell/components/AppShell"

export const Route = createRootRoute({
	component: AppShell,
})
