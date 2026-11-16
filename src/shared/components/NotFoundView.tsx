import { Link } from "@tanstack/react-router"
import { ArrowLeft, Compass } from "lucide-react"

const NotFoundView = () => {
	return (
		<div className="flex h-full w-full flex-col items-center justify-center p-6 text-center select-none">
			<div className="ingot-glass flex max-w-md flex-col items-center rounded-2xl p-6">
				<div className="flex h-11 w-11 items-center justify-center rounded-2xl border border-amber-500/25 bg-amber-500/15 text-amber-500">
					<Compass className="h-5 w-5" />
				</div>

				<span className="mt-4 rounded-full bg-black/5 px-2.5 py-0.5 font-mono text-[11px] font-semibold text-muted-foreground dark:bg-white/6">
					404 • ROUTE NOT FOUND
				</span>

				<h1 className="mt-2.5 text-base font-semibold text-foreground">
					This chunk hasn&apos;t been generated
				</h1>

				<p className="mt-1.5 text-xs leading-relaxed text-muted-foreground">
					The page or instance you tried to open doesn&apos;t exist or may have been moved.
				</p>

				<Link
					to="/"
					className="mt-5 inline-flex h-8 items-center gap-1.5 rounded-xl bg-primary px-3.5 text-xs font-semibold text-primary-foreground shadow-sm transition-opacity hover:opacity-90"
				>
					<ArrowLeft className="h-3.5 w-3.5" />
					<span>Return to Play</span>
				</Link>
			</div>
		</div>
	)
}

export default NotFoundView
