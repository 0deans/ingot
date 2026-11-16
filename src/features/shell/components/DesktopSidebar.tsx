import { Link } from "@tanstack/react-router"
import { ChevronLeft, ChevronRight, ChevronsUpDown, Settings } from "lucide-react"
import { LayoutGroup, motion } from "motion/react"
import { type MouseEvent } from "react"
import {
	NAV_GROUP_LABELS,
	NAV_GROUP_ORDER,
	getSidebarNavItems,
	isNavItemActive,
	type NavGroupId,
	type NavItem,
} from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"
import { getAccountInitials, type AccountProfile } from "@/shared/stores/useAccountStore"
import { type NavVariant } from "@/shared/stores/useLauncherPrefsStore"

interface DesktopSidebarProps {
	variant: Exclude<NavVariant, "capsule">
	compact: boolean
	railExpanded: boolean
	currentPath: string
	activeAccount: AccountProfile
	ramGb: number
	runningCount: number
	accountOpen: boolean
	onToggleRailExpanded: () => void
	onToggleAccount: (event: MouseEvent<HTMLButtonElement>) => void
}

const DesktopSidebar = ({
	variant,
	compact,
	railExpanded,
	currentPath,
	activeAccount,
	ramGb,
	runningCount,
	accountOpen,
	onToggleRailExpanded,
	onToggleAccount,
}: DesktopSidebarProps) => {
	const items = getSidebarNavItems(import.meta.env.DEV)
	const isIconOnly = variant === "rail" && !railExpanded
	const isSequoia = variant === "sequoia"
	const targetWidth = isIconOnly ? 64 : compact ? 196 : 224

	const sectionsByGroup = NAV_GROUP_ORDER.map((groupId): { id: NavGroupId; items: NavItem[] } => {
		const groupItems = items.filter((item) => {
			return item.group === groupId
		})

		return {
			id: groupId,
			items: groupItems,
		}
	})
	const groupedSections = sectionsByGroup.filter((section) => {
		return section.items.length > 0
	})

	const baseContainerClass =
		"flex shrink-0 flex-col overflow-hidden border transition-[border-radius,border-color,background-color,padding] duration-200"
	const sequoiaContainerClass = cn(
		baseContainerClass,
		"rounded-2xl border-sidebar-border bg-sidebar/90 p-2 shadow-xl backdrop-blur-2xl",
	)
	const dockedContainerClass = cn(
		baseContainerClass,
		"rounded-none border-transparent border-r-sidebar-border bg-sidebar/80 backdrop-blur-xl",
		isIconOnly ? "p-2" : "p-2.5",
	)
	const containerClass = isSequoia ? sequoiaContainerClass : dockedContainerClass

	const isSettingsActive = isNavItemActive(currentPath, "/settings")
	const outerTargetWidth = isSequoia ? targetWidth + 10 : targetWidth
	const transitionSpec = { duration: 0.24, ease: [0.22, 1, 0.36, 1] as const }

	return (
		<motion.div
			initial={{ width: 0, opacity: 0 }}
			animate={{ width: outerTargetWidth, opacity: 1 }}
			exit={{ width: 0, opacity: 0 }}
			transition={transitionSpec}
			className="flex shrink-0 overflow-hidden"
		>
			<motion.aside
				aria-label="Primary Sidebar"
				initial={false}
				animate={{
					width: targetWidth,
					marginTop: isSequoia ? 10 : 0,
					marginBottom: isSequoia ? 10 : 0,
					marginLeft: isSequoia ? 10 : 0,
				}}
				transition={transitionSpec}
				className={containerClass}
			>
				{variant === "rail" ? (
					<div
						className={cn(
							"mb-2 flex items-center",
							isIconOnly ? "justify-center" : "justify-between px-1.5",
						)}
					>
						{!isIconOnly ? (
							<span className="overflow-hidden text-[10.5px] font-semibold tracking-wider whitespace-nowrap text-muted-foreground uppercase">
								Navigation
							</span>
						) : null}
						<button
							type="button"
							onClick={onToggleRailExpanded}
							aria-label={railExpanded ? "Collapse sidebar rail" : "Expand sidebar rail"}
							title={railExpanded ? "Collapse sidebar" : "Expand sidebar"}
							className="flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-lg border border-sidebar-border bg-foreground/4 text-muted-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-foreground"
						>
							{railExpanded ? (
								<ChevronLeft className="size-3.5" />
							) : (
								<ChevronRight className="size-3.5" />
							)}
						</button>
					</div>
				) : null}

				<LayoutGroup id="desktop-sidebar">
					<motion.nav
						layoutScroll
						className="scroll-fade-y-sm no-scrollbar flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto py-2"
					>
						{groupedSections.map((section) => {
							return (
								<div key={section.id} className="flex flex-col gap-0.5">
									{!isIconOnly ? (
										<div className="overflow-hidden px-2.5 pt-1 pb-1 text-[10px] font-semibold tracking-wider whitespace-nowrap text-muted-foreground uppercase">
											{NAV_GROUP_LABELS[section.id]}
										</div>
									) : (
										<div className="mx-auto my-1 h-px w-6 bg-sidebar-border first:hidden" />
									)}

									{section.items.map((item) => {
										const Icon = item.icon
										const isActive = isNavItemActive(currentPath, item.path)
										const showPlayRunningCount = item.path === "/" && runningCount > 0

										return (
											<Link
												key={item.path}
												to={item.path}
												title={isIconOnly ? item.label : undefined}
												className={cn(
													"relative flex h-9 items-center rounded-xl",
													isIconOnly ? "justify-center px-2" : "gap-2.5 px-2.5",
													isActive
														? "text-sidebar-foreground"
														: "text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
												)}
											>
												{isActive ? (
													<motion.span
														layoutId="sidebar-active-pill"
														transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
														className="absolute inset-0 rounded-xl border border-foreground/12 bg-foreground/10 shadow-xs"
													/>
												) : null}
												<Icon
													className={cn(
														"relative z-10 size-4 shrink-0",
														isActive && "text-sidebar-primary",
													)}
												/>
												{!isIconOnly ? (
													<span className="relative z-10 min-w-0 flex-1 truncate text-[12.5px] font-medium whitespace-nowrap">
														{item.label}
													</span>
												) : null}

												{!isIconOnly && showPlayRunningCount ? (
													<span className="relative z-10 shrink-0 rounded-full bg-primary/20 px-1.5 py-0.5 text-[10px] font-semibold text-primary tabular-nums">
														{runningCount}
													</span>
												) : null}

												{!isIconOnly && !showPlayRunningCount && item.badge ? (
													<span className="relative z-10 shrink-0 rounded-full bg-foreground/10 px-1.5 py-0.5 text-[9.5px] font-semibold text-foreground/80">
														{item.badge}
													</span>
												) : null}

												{isIconOnly && showPlayRunningCount ? (
													<span className="absolute top-1.5 right-1.5 z-10 size-1.5 rounded-full bg-primary" />
												) : null}
											</Link>
										)
									})}
								</div>
							)
						})}

						<div className="mt-auto border-t border-sidebar-border pt-2">
							<Link
								to="/settings"
								title={isIconOnly ? "Settings" : undefined}
								className={cn(
									"relative flex h-9 items-center rounded-xl",
									isIconOnly ? "justify-center px-2" : "gap-2.5 px-2.5",
									isSettingsActive
										? "text-sidebar-foreground"
										: "text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
								)}
							>
								{isSettingsActive ? (
									<motion.span
										layoutId="sidebar-active-pill"
										transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
										className="absolute inset-0 rounded-xl border border-foreground/12 bg-foreground/10 shadow-xs"
									/>
								) : null}
								<Settings
									className={cn(
										"relative z-10 size-4 shrink-0",
										isSettingsActive && "text-sidebar-primary",
									)}
								/>
								{!isIconOnly ? (
									<>
										<span className="relative z-10 min-w-0 flex-1 truncate text-[12.5px] font-medium whitespace-nowrap">
											Settings
										</span>
										<span className="relative z-10 shrink-0 rounded-full bg-foreground/8 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground tabular-nums">
											{ramGb}GB
										</span>
									</>
								) : null}
							</Link>
						</div>
					</motion.nav>
				</LayoutGroup>

				<div className="mt-1 flex flex-col pt-1">
					<button
						type="button"
						aria-expanded={accountOpen}
						aria-haspopup="dialog"
						onClick={onToggleAccount}
						title={isIconOnly ? `${activeAccount.username} (${activeAccount.type})` : undefined}
						className={cn(
							"flex cursor-pointer items-center rounded-xl border border-sidebar-border bg-foreground/4 transition-colors hover:bg-foreground/8",
							isIconOnly ? "justify-center p-1.5" : "gap-2.5 px-2 py-1.5 text-left",
						)}
					>
						<div
							className={cn(
								"flex size-7 shrink-0 items-center justify-center rounded-lg bg-linear-to-br text-[11px] font-bold text-white shadow-inner",
								activeAccount.avatarBg,
							)}
						>
							{getAccountInitials(activeAccount.username)}
						</div>
						{!isIconOnly ? (
							<>
								<div className="min-w-0 flex-1 overflow-hidden">
									<p className="truncate text-[12px] font-medium whitespace-nowrap text-sidebar-foreground">
										{activeAccount.username}
									</p>
									<p className="truncate text-[10px] whitespace-nowrap text-muted-foreground">
										{activeAccount.type}
									</p>
								</div>
								<ChevronsUpDown className="size-3.5 shrink-0 text-muted-foreground" />
							</>
						) : null}
					</button>
				</div>
			</motion.aside>
		</motion.div>
	)
}

export default DesktopSidebar
