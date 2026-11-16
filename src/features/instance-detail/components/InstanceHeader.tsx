import { Link } from "@tanstack/react-router"
import { ArrowLeft, Package, Play, Square } from "lucide-react"
import { cn } from "@/shared/lib/utils"
import { COLOR_TAGS, type GameInstance } from "@/shared/stores/useInstancesStore"

interface InstanceHeaderProps {
	instance: GameInstance
	isRunning: boolean
	onToggleLaunch: () => void
}

const InstanceHeader = ({ instance, isRunning, onToggleLaunch }: InstanceHeaderProps) => {
	const tagMeta = COLOR_TAGS[instance.colorTag]

	return (
		<div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="flex min-w-0 items-center gap-3">
				<Link
					to="/"
					aria-label="Back to Play"
					className="flex size-8 shrink-0 items-center justify-center rounded-xl border border-border bg-foreground/5 text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
				>
					<ArrowLeft className="size-4" />
				</Link>
				<div className="relative flex size-10 shrink-0 items-center justify-center rounded-xl border border-primary/30 bg-primary/15 text-primary">
					<Package className="size-5" />
					<span className={cn("absolute -top-0.5 -right-0.5 size-2.5 rounded-full", tagMeta.dot)} />
				</div>
				<div className="min-w-0">
					<div className="flex items-center gap-2">
						<h1 className="truncate text-base font-semibold text-foreground">{instance.name}</h1>
						<span className="rounded-md border border-border bg-foreground/6 px-1.5 py-0.5 font-mono text-[10px] text-foreground/80">
							{instance.engine}
						</span>
					</div>
					<p className="mt-0.5 truncate text-[11.5px] text-muted-foreground">
						{instance.loader} · {instance.version} · {instance.playtime} played
					</p>
				</div>
			</div>

			<button
				type="button"
				onClick={onToggleLaunch}
				className={cn(
					"flex h-9 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl px-4 text-xs font-semibold transition-all",
					isRunning
						? "border border-rose-500/35 bg-rose-500/18 text-rose-500 hover:bg-rose-500/28 dark:text-rose-200"
						: "bg-primary text-primary-foreground hover:opacity-90",
				)}
			>
				{isRunning ? (
					<>
						<Square className="size-3.5 fill-current" />
						<span>Stop Instance</span>
					</>
				) : (
					<>
						<Play className="size-3.5 fill-current" />
						<span>Launch Instance</span>
					</>
				)}
			</button>
		</div>
	)
}

export default InstanceHeader
