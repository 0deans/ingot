import { useRouter, useRouterState, type ErrorComponentProps } from "@tanstack/react-router"
import { useCallback } from "react"
import ErrorFallbackCard from "@/shared/components/ErrorFallbackCard"

const ErrorFallbackView = ({ error, reset }: ErrorComponentProps) => {
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

	const canNavigateHome = pathname !== "/"

	return (
		<div className="flex h-full min-h-80 w-full items-center justify-center p-4 md:p-6">
			<ErrorFallbackCard
				error={error}
				scope="route"
				routePath={pathname}
				onRetry={handleRetry}
				onGoHome={canNavigateHome ? handleGoHome : undefined}
			/>
		</div>
	)
}

export default ErrorFallbackView
