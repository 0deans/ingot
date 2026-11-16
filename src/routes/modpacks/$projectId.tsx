import { createFileRoute } from "@tanstack/react-router"
import ModpackDetailPage from "@/features/modpacks/ModpackDetailPage"

const ModpackDetailRouteComponent = () => {
	const { projectId } = Route.useParams()
	return <ModpackDetailPage projectId={projectId} />
}

export const Route = createFileRoute("/modpacks/$projectId")({
	component: ModpackDetailRouteComponent,
})
