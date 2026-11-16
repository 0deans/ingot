import { createFileRoute } from "@tanstack/react-router"
import JvmOverridesForm from "@/features/instance-detail/components/JvmOverridesForm"
import { selectInstanceById, useInstancesStore } from "@/shared/stores/useInstancesStore"

const InstanceSettingsRouteComponent = () => {
	const { instanceId } = Route.useParams()
	const instance = useInstancesStore(selectInstanceById(instanceId))

	return <JvmOverridesForm defaultArgs={instance ? instance.jvmArgs : "G1GC • 6GB"} />
}

export const Route = createFileRoute("/instances/$instanceId/settings")({
	component: InstanceSettingsRouteComponent,
})
