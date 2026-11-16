import { Component, type ErrorInfo, type ReactNode } from "react"

export interface ErrorFallbackRenderProps {
	error: unknown
	reset: () => void
}

interface ErrorBoundaryProps {
	children: ReactNode
	fallback: ReactNode | ((props: ErrorFallbackRenderProps) => ReactNode)
	resetKey?: string
	onError?: (error: unknown, errorInfo: ErrorInfo) => void
}

interface ErrorBoundaryState {
	hasError: boolean
	error: unknown
	previousResetKey: string | undefined
}

class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
	constructor(props: ErrorBoundaryProps) {
		super(props)
		this.state = {
			hasError: false,
			error: null,
			previousResetKey: props.resetKey,
		}
	}

	static getDerivedStateFromError(error: unknown): Partial<ErrorBoundaryState> {
		return {
			hasError: true,
			error,
		}
	}

	static getDerivedStateFromProps(
		nextProps: ErrorBoundaryProps,
		prevState: ErrorBoundaryState,
	): Partial<ErrorBoundaryState> | null {
		if (nextProps.resetKey === prevState.previousResetKey) {
			return null
		}
		return {
			hasError: false,
			error: null,
			previousResetKey: nextProps.resetKey,
		}
	}

	componentDidCatch(error: unknown, errorInfo: ErrorInfo): void {
		console.error("[Ingot] ErrorBoundary caught an error:", error, errorInfo)
		this.props.onError?.(error, errorInfo)
	}

	private readonly handleReset = (): void => {
		this.setState({
			hasError: false,
			error: null,
		})
	}

	render(): ReactNode {
		if (!this.state.hasError) {
			return this.props.children
		}

		const { fallback } = this.props
		if (typeof fallback === "function") {
			return fallback({
				error: this.state.error,
				reset: this.handleReset,
			})
		}

		return fallback
	}
}

export default ErrorBoundary
