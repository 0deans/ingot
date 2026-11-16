import { createFileRoute } from "@tanstack/react-router"
import InstanceDetailLayout from "@/features/instance-detail/InstanceDetailLayout"

const InstanceDetailRouteComponent = () => {
	const { instanceId } = Route.useParams()
	return <InstanceDetailLayout instanceId={instanceId} />
}

export const Route = createFileRoute("/instances/$instanceId")({
	component: InstanceDetailRouteComponent,
})
