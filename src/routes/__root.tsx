import {
	createRootRoute,
	useRouter,
	useRouterState,
	type ErrorComponentProps,
} from "@tanstack/react-router"
import { useCallback } from "react"
import AppShell from "@/features/shell/components/AppShell"
import RootErrorFallback from "@/shared/components/RootErrorFallback"

const RootRouteErrorComponent = ({ error, reset }: ErrorComponentProps) => {
	const router = useRouter()
	const pathname = useRouterState({
		select: (state) => {
			return state.location.pathname
		},
	})

	const handleRetry = useCallback(() => {
		reset()
		void router.invalidate()
	}, [reset, router])

	const handleGoHome = useCallback(() => {
		reset()
		void router.navigate({ to: "/" })
	}, [reset, router])

	return (
		<RootErrorFallback
			error={error}
			routePath={pathname}
			onGoHome={pathname !== "/" ? handleGoHome : undefined}
			reset={handleRetry}
		/>
	)
}

export const Route = createRootRoute({
	component: AppShell,
	errorComponent: RootRouteErrorComponent,
})
