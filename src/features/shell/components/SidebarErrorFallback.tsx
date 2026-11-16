import { Link } from "@tanstack/react-router"
import { AlertTriangle, Home, RotateCcw } from "lucide-react"

interface SidebarErrorFallbackProps {
	onRetry: () => void
}

const SidebarErrorFallback = ({ onRetry }: SidebarErrorFallbackProps) => {
	return (
		<aside
			aria-label="Sidebar recovery"
			className="flex w-14 shrink-0 flex-col items-center justify-between border-r border-sidebar-border bg-sidebar/85 py-3 backdrop-blur-xl select-none"
		>
			<div className="flex flex-col items-center gap-2">
				<div
					title="Navigation sidebar failed to render"
					className="flex size-8 items-center justify-center rounded-xl border border-rose-500/25 bg-rose-500/12 text-rose-500"
				>
					<AlertTriangle className="size-4" />
				</div>

				<Link
					to="/"
					title="Go to Play"
					aria-label="Go to Play"
					className="flex size-8 items-center justify-center rounded-xl border border-sidebar-border bg-foreground/4 text-muted-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-foreground"
				>
					<Home className="size-4" />
				</Link>
			</div>

			<button
				type="button"
				onClick={onRetry}
				title="Retry navigation sidebar"
				aria-label="Retry navigation sidebar"
				className="flex size-8 cursor-pointer items-center justify-center rounded-xl border border-sidebar-border bg-foreground/5 text-foreground transition-colors hover:bg-foreground/10"
			>
				<RotateCcw className="size-3.5" />
			</button>
		</aside>
	)
}

export default SidebarErrorFallback
