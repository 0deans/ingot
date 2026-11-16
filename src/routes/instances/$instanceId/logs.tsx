import { createFileRoute } from "@tanstack/react-router"
import LiveLogViewer from "@/features/instance-detail/components/LiveLogViewer"
import { selectInstanceById, useInstancesStore } from "@/shared/stores/useInstancesStore"

const InstanceLogsRouteComponent = () => {
	const { instanceId } = Route.useParams()
	const instance = useInstancesStore(selectInstanceById(instanceId))

	return <LiveLogViewer instanceName={instance ? instance.name : instanceId} />
}

export const Route = createFileRoute("/instances/$instanceId/logs")({
	component: InstanceLogsRouteComponent,
})
