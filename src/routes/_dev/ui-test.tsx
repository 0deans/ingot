import { createFileRoute } from "@tanstack/react-router"
import UiTestPage from "@/features/ui-test/UiTestPage"
import NotFoundView from "@/shared/components/NotFoundView"

const UiTestRouteComponent = () => {
	if (!import.meta.env.DEV) {
		return <NotFoundView />
	}
	return <UiTestPage />
}

export const Route = createFileRoute("/_dev/ui-test")({
	component: UiTestRouteComponent,
})
