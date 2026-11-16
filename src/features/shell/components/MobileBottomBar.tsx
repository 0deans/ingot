import { Link } from "@tanstack/react-router"
import { MoreHorizontal, Play } from "lucide-react"
import { motion } from "motion/react"
import { PRIMARY_MOBILE_PATHS, isNavItemActive, type NavItem } from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"
import { COLOR_TAGS, type GameInstance } from "@/shared/stores/useInstancesStore"

interface MobileBottomBarProps {
	currentPath: string
	allItems: NavItem[]
	moreOpen: boolean
	selectedInstance: GameInstance | undefined
	ramGb: number
	isLaunching: boolean
	onLaunch: () => void
	onToggleMore: () => void
}

const MobileBottomBar = ({
	currentPath,
	allItems,
	moreOpen,
	selectedInstance,
	ramGb,
	isLaunching,
	onLaunch,
	onToggleMore,
}: MobileBottomBarProps) => {
	const matchedItems = PRIMARY_MOBILE_PATHS.map((path) => {
		return allItems.find((item) => {
			return item.path === path
		})
	})
	const primaryItems = matchedItems.filter((item): item is NavItem => {
		return item !== undefined
	})

	const isPrimaryActive = PRIMARY_MOBILE_PATHS.some((path) => {
		return isNavItemActive(currentPath, path)
	})
	const isMoreActive = moreOpen || !isPrimaryActive

	return (
		<div className="relative z-30 shrink-0 border-t border-border bg-sidebar/90 pb-[env(safe-area-inset-bottom)] backdrop-blur-2xl">
			{currentPath === "/" && selectedInstance ? (
				<div className="flex items-center justify-between gap-2.5 border-b border-border bg-foreground/3 px-3.5 py-2">
					<div className="flex min-w-0 items-center gap-2">
						<span
							className={cn(
								"size-2 shrink-0 rounded-full",
								COLOR_TAGS[selectedInstance.colorTag].dot,
							)}
						/>
						<div className="min-w-0">
							<p className="truncate text-[12.5px] font-semibold text-foreground">
								{selectedInstance.name}
							</p>
							<p className="truncate text-[10.5px] text-muted-foreground">
								{selectedInstance.version} · {ramGb} GB RAM
							</p>
						</div>
					</div>
					<button
						type="button"
						disabled={isLaunching}
						onClick={onLaunch}
						className="flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3.5 text-[12px] font-semibold text-primary-foreground shadow-md shadow-primary/20 transition-all hover:opacity-90 disabled:opacity-60"
					>
						<Play className="size-3.5 fill-current" />
						<span>{isLaunching ? "Launching..." : "Play"}</span>
					</button>
				</div>
			) : null}

			<nav aria-label="Mobile Navigation" className="grid grid-cols-5 gap-1 px-2 py-1.5">
				{primaryItems.map((item) => {
					const Icon = item.icon
					const isActive = isNavItemActive(currentPath, item.path) && !moreOpen
					return (
						<Link
							key={item.path}
							to={item.path}
							className={cn(
								"relative flex flex-col items-center justify-center gap-0.5 rounded-xl py-1.5 text-[10px] font-medium",
								isActive ? "text-primary" : "text-muted-foreground hover:text-foreground",
							)}
						>
							{isActive ? (
								<motion.span
									layoutId="mobile-tab-pill"
									transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
									className="absolute inset-0 rounded-xl bg-foreground/10"
								/>
							) : null}
							<Icon className="relative z-10 size-4" />
							<span className="relative z-10 truncate">{item.shortLabel}</span>
						</Link>
					)
				})}

				<button
					type="button"
					onClick={onToggleMore}
					aria-expanded={moreOpen}
					className={cn(
						"relative flex cursor-pointer flex-col items-center justify-center gap-0.5 rounded-xl py-1.5 text-[10px] font-medium",
						isMoreActive ? "text-primary" : "text-muted-foreground hover:text-foreground",
					)}
				>
					{isMoreActive ? (
						<motion.span
							layoutId="mobile-tab-pill"
							transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
							className="absolute inset-0 rounded-xl bg-foreground/10"
						/>
					) : null}
					<MoreHorizontal className="relative z-10 size-4" />
					<span className="relative z-10">More</span>
				</button>
			</nav>
		</div>
	)
}

export default MobileBottomBar
