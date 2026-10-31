import type { ErrorComponentProps } from "@tanstack/react-router"
import { useNavigate } from "@tanstack/react-router"
import { memo, useCallback } from "react"
import AppErrorView from "@/components/layout/app-error-view"

interface RouteErrorViewProps extends ErrorComponentProps {}

const RouteErrorView = ({ error, reset, info }: RouteErrorViewProps) => {
	const navigate = useNavigate()

	const handleGoHome = useCallback(() => {
		navigate({ to: "/" })
	}, [navigate])

	return (
		<div className="flex size-full min-h-0 flex-1 flex-col overflow-hidden bg-background/50">
			<AppErrorView error={error} reset={reset} info={info} onGoHome={handleGoHome} />
		</div>
	)
}

RouteErrorView.displayName = "RouteErrorView"

export default memo(RouteErrorView)
