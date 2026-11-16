import { motion } from "motion/react"
import type { KeyboardEvent, MouseEvent } from "react"
import type { GameInstance, InstanceEngine } from "@/features/play/types"
import { cn } from "@/shared/lib/utils"
import { COLOR_TAGS } from "@/shared/stores/useInstancesStore"

interface InstanceCardProps {
	instance: GameInstance
	isSelected: boolean
	isRunning: boolean
	onSelect: (id: string) => void
	onCycleTag: (id: string) => void
	onOpenDetails?: (id: string) => void
	onContextMenu?: (event: MouseEvent<HTMLDivElement>) => void
}

const ENGINE_BADGE_CLASS: Record<InstanceEngine, string> = {
	"Horizon (Rust)": "border-rose-500/30 bg-rose-500/15 text-rose-600 dark:text-rose-300",
	Bedrock: "border-cyan-500/30 bg-cyan-500/15 text-cyan-700 dark:text-cyan-300",
	Java: "border-primary/30 bg-primary/15 text-primary",
}

const InstanceCard = ({
	instance,
	isSelected,
	isRunning,
	onSelect,
	onCycleTag,
	onOpenDetails,
	onContextMenu,
}: InstanceCardProps) => {
	const tagStyle = COLOR_TAGS[instance.colorTag]
	const engineBadgeClass = ENGINE_BADGE_CLASS[instance.engine]
	const cardSurfaceClass = isSelected
		? `${tagStyle.activeRing} shadow-md`
		: `border-border bg-card/60 ${tagStyle.cardBorder} hover:bg-card`

	const handleCardClick = () => {
		onSelect(instance.id)
	}

	const handleCardDoubleClick = () => {
		if (onOpenDetails) {
			onOpenDetails(instance.id)
		}
	}

	const handleCardContextMenu = (e: MouseEvent<HTMLDivElement>) => {
		if (!onContextMenu) {
			return
		}
		e.preventDefault()
		onSelect(instance.id)
		onContextMenu(e)
	}

	const handleCardKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
		if (e.target !== e.currentTarget) {
			return
		}
		if (e.key === "Enter" || e.key === " ") {
			e.preventDefault()
			onSelect(instance.id)
		}
	}

	const handleCycleTagClick = (e: MouseEvent<HTMLButtonElement>) => {
		e.stopPropagation()
		onCycleTag(instance.id)
	}

	return (
		<motion.div
			layout="position"
			role="button"
			tabIndex={0}
			aria-label={`Select instance ${instance.name}`}
			aria-pressed={isSelected}
			onClick={handleCardClick}
			onDoubleClick={handleCardDoubleClick}
			onContextMenu={handleCardContextMenu}
			onKeyDown={handleCardKeyDown}
			className={cn(
				"group cursor-pointer rounded-2xl border p-3 transition-colors select-none focus:ring-2 focus:ring-ring focus:outline-none",
				cardSurfaceClass,
			)}
		>
			{/* Fixed h-5 header prevents selecting from jumping card height */}
			<div className="flex h-5 items-center justify-between gap-2">
				<div className="flex min-w-0 items-center gap-2">
					<button
						type="button"
						onClick={handleCycleTagClick}
						title="Click to cycle color tag"
						aria-label={`Cycle tag color for ${instance.name}`}
						className={cn(
							"h-3 w-3 shrink-0 cursor-pointer rounded-full ring-2 ring-border transition-transform hover:scale-125",
							tagStyle.dot,
						)}
					/>
					<span className="truncate text-xs leading-none font-bold text-foreground">
						{instance.name}
					</span>
				</div>

				<div className="flex h-5 shrink-0 items-center">
					{isRunning ? (
						<span className="flex h-4 items-center gap-1 rounded border border-primary/30 bg-primary/20 px-1.5 font-mono text-[9px] leading-none text-primary uppercase">
							<span className="h-1.5 w-1.5 animate-pulse rounded-full bg-primary" />
							Running
						</span>
					) : (
						isSelected && (
							<span className="flex h-4 items-center rounded border border-border bg-foreground/10 px-1.5 font-mono text-[9px] leading-none text-foreground uppercase">
								Selected
							</span>
						)
					)}
				</div>
			</div>

			<div className="mt-2 flex h-5 flex-wrap items-center gap-1.5">
				<span
					className={cn(
						"rounded border px-1.5 py-0.5 font-mono text-[10px] leading-none",
						engineBadgeClass,
					)}
				>
					{instance.engine}
				</span>
				<span className="font-mono text-[10px] leading-none text-muted-foreground">
					{instance.loader} • {instance.version}
				</span>
			</div>

			<div className="mt-2.5 flex items-center justify-between border-t border-border pt-2 font-mono text-[10px] leading-none text-muted-foreground">
				<span>{instance.playtime} played</span>
				<span>{instance.lastPlayed}</span>
			</div>
		</motion.div>
	)
}

export default InstanceCard
