import { createFileRoute } from "@tanstack/react-router"
import ModListVirtual from "@/features/instance-detail/components/ModListVirtual"

export const Route = createFileRoute("/instances/$instanceId/mods")({
	component: ModListVirtual,
})
