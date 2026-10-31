import { Component, type ErrorInfo, type ReactNode } from "react"
import AppErrorView from "@/components/layout/app-error-view"
import Titlebar from "@/components/layout/titlebar"
import { isMobileEnvironment } from "@/lib/platform"

interface RootErrorBoundaryProps {
	children: ReactNode
}

interface RootErrorBoundaryState {
	error: unknown | null
	info: ErrorInfo | null
}

export class RootErrorBoundary extends Component<RootErrorBoundaryProps, RootErrorBoundaryState> {
	override state: RootErrorBoundaryState = {
		error: null,
		info: null,
	}

	static getDerivedStateFromError(error: unknown): Partial<RootErrorBoundaryState> {
		return { error }
	}

	override componentDidCatch(error: unknown, info: ErrorInfo) {
		this.setState({ info })
		console.error("RootErrorBoundary caught uncaught error:", error, info)
	}

	private handleReset = () => {
		this.setState({ error: null, info: null })
		window.location.reload()
	}

	override render() {
		if (this.state.error !== null) {
			const isMobile = isMobileEnvironment()
			const info = this.state.info
				? { componentStack: this.state.info.componentStack ?? undefined }
				: undefined

			if (isMobile) {
				return (
					<main className="flex h-dvh min-h-0 w-full flex-1 flex-col overflow-hidden bg-background">
						<AppErrorView
							error={this.state.error}
							reset={this.handleReset}
							info={info}
							isRoot={true}
							onGoHome={this.handleReset}
						/>
					</main>
				)
			}

			return (
				<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-radial-[at_top_center] from-card/60 via-background to-background text-foreground antialiased selection:bg-primary/20 dark:to-black">
					<Titlebar title="Ingot" />
					<main className="flex flex-1 overflow-hidden">
						<AppErrorView
							error={this.state.error}
							reset={this.handleReset}
							info={info}
							isRoot={true}
							onGoHome={this.handleReset}
						/>
					</main>
				</div>
			)
		}

		return this.props.children
	}
}

export default RootErrorBoundary
