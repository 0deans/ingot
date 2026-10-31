import type { ErrorComponentProps } from "@tanstack/react-router"
import { useNavigate } from "@tanstack/react-router"
import { memo, useCallback } from "react"
import AppErrorView from "@/components/layout/app-error-view"
import Titlebar from "@/components/layout/titlebar"
import { isMobileEnvironment } from "@/lib/platform"

interface RootErrorViewProps extends ErrorComponentProps {}

const RootErrorView = ({ error, reset, info }: RootErrorViewProps) => {
	const navigate = useNavigate()
	const isMobile = isMobileEnvironment()

	const handleGoHome = useCallback(() => {
		try {
			navigate({ to: "/" })
		} catch {
			window.location.reload()
		}
	}, [navigate])

	if (isMobile) {
		return (
			<main className="flex h-dvh min-h-0 w-full flex-1 flex-col overflow-hidden bg-background">
				<AppErrorView
					error={error}
					reset={reset}
					info={info}
					isRoot={true}
					onGoHome={handleGoHome}
				/>
			</main>
		)
	}

	return (
		<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-radial-[at_top_center] from-card/60 via-background to-background text-foreground antialiased selection:bg-primary/20 dark:to-black">
			<Titlebar title="Ingot" />
			<main className="flex flex-1 overflow-hidden">
				<AppErrorView
					error={error}
					reset={reset}
					info={info}
					isRoot={true}
					onGoHome={handleGoHome}
				/>
			</main>
		</div>
	)
}

RootErrorView.displayName = "RootErrorView"

export default memo(RootErrorView)
