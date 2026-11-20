import { useCallback } from "react"
import { useTranslation } from "react-i18next"
import ErrorFallbackCard from "@/shared/components/errors/ErrorFallbackCard"
import SafeTitlebar from "@/shared/components/window/SafeTitlebar"

interface RootErrorFallbackProps {
	error: unknown
	reset?: () => void
	routePath?: string
	onGoHome?: () => void
}

const RootErrorFallback = ({ error, reset, routePath, onGoHome }: RootErrorFallbackProps) => {
	const { t } = useTranslation()

	const handleReloadApp = useCallback(() => {
		window.location.reload()
	}, [])

	return (
		<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
			<SafeTitlebar statusLabel={t("shell.window.recoveryMode")} onReload={handleReloadApp} />

			<main className="relative flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-4 sm:p-6">
				<div
					aria-hidden="true"
					className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(244,63,94,0.08),transparent_55%)]"
				/>

				<div className="relative z-10 flex w-full justify-center">
					<ErrorFallbackCard
						error={error}
						scope="root"
						routePath={routePath}
						onRetry={reset}
						onGoHome={onGoHome}
						onReloadApp={handleReloadApp}
					/>
				</div>
			</main>
		</div>
	)
}

export default RootErrorFallback
