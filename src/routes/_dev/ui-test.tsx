import { createFileRoute } from "@tanstack/react-router"
import UiTestPage from "@/features/ui-test/UiTestPage"
import NotFoundView from "@/shared/components/NotFoundView"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

const UiTestRouteComponent = () => {
	const showDeveloperPages = useLauncherPrefsStore((s) => {
		return s.showDeveloperPages
	})

	if (!showDeveloperPages) {
		return <NotFoundView />
	}
	return <UiTestPage />
}

export const Route = createFileRoute("/_dev/ui-test")({
	component: UiTestRouteComponent,
})
