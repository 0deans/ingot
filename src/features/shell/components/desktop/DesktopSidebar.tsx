import { ChevronLeft, ChevronRight, ChevronsUpDown, Settings } from "lucide-react"
import { LayoutGroup, motion } from "motion/react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import SidebarNavItem from "@/features/shell/components/desktop/SidebarNavItem"
import {
	NAV_GROUP_LABEL_KEYS,
	NAV_GROUP_ORDER,
	getSidebarNavItems,
	isNavItemActive,
	type NavGroupId,
	type NavItem,
} from "@/features/shell/navigation"
import {
	isAnchoredOverlayOpen,
	useShellOverlayStore,
} from "@/features/shell/stores/useShellOverlayStore"
import { cn } from "@/shared/lib/utils"
import {
	ACCOUNT_TYPE_LABEL_KEYS,
	getAccountInitials,
	selectActiveAccount,
	useAccountStore,
} from "@/shared/stores/useAccountStore"
import { useLauncherPrefsStore, type NavVariant } from "@/shared/stores/useLauncherPrefsStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

interface DesktopSidebarProps {
	variant: Exclude<NavVariant, "capsule">
	compact: boolean
	currentPath: string
	layoutKey: string
}

const DesktopSidebar = ({ variant, compact, currentPath, layoutKey }: DesktopSidebarProps) => {
	const { t } = useTranslation()
	const { railExpanded, showDeveloperPages, toggleRailExpanded } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				railExpanded: s.railExpanded,
				showDeveloperPages: s.showDeveloperPages,
				toggleRailExpanded: s.toggleRailExpanded,
			}
		}),
	)
	const activeAccount = useAccountStore(selectActiveAccount)
	const runningCount = useProcessStore((s) => {
		return s.runningProcesses.length
	})
	const { activeOverlay, toggleAnchored } = useShellOverlayStore(
		useShallow((s) => {
			return {
				activeOverlay: s.activeOverlay,
				toggleAnchored: s.toggleAnchored,
			}
		}),
	)

	const accountOpen = isAnchoredOverlayOpen(activeOverlay, "account", layoutKey)
	const items = getSidebarNavItems(showDeveloperPages)
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
				aria-label={t("shell.primarySidebarAria")}
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
								{t("nav.groups.navigation")}
							</span>
						) : null}
						<button
							type="button"
							onClick={toggleRailExpanded}
							aria-label={
								railExpanded ? t("shell.collapseSidebarAria") : t("shell.expandSidebarAria")
							}
							title={railExpanded ? t("shell.collapseSidebar") : t("shell.expandSidebar")}
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
											{t(NAV_GROUP_LABEL_KEYS[section.id])}
										</div>
									) : (
										<div className="mx-auto my-1 h-px w-6 bg-sidebar-border first:hidden" />
									)}

									{section.items.map((item) => {
										const showPlayRunningCount = item.path === "/" && runningCount > 0
										let badgeText = item.badge
										if (item.badgeKey) {
											badgeText = t(item.badgeKey)
										}
										if (showPlayRunningCount) {
											badgeText = String(runningCount)
										}

										return (
											<SidebarNavItem
												key={item.path}
												path={item.path}
												label={t(item.labelKey)}
												icon={item.icon}
												isActive={isNavItemActive(currentPath, item.path)}
												isIconOnly={isIconOnly}
												badgeText={badgeText}
												badgeHighlight={showPlayRunningCount}
												showCompactDot={showPlayRunningCount}
											/>
										)
									})}
								</div>
							)
						})}

						<div className="mt-auto border-t border-sidebar-border pt-2">
							<SidebarNavItem
								path="/settings"
								label={t("nav.items.settings.label")}
								icon={Settings}
								isActive={isSettingsActive}
								isIconOnly={isIconOnly}
							/>
						</div>
					</motion.nav>
				</LayoutGroup>

				<div className="mt-1 flex flex-col pt-1">
					<button
						type="button"
						aria-expanded={accountOpen}
						aria-haspopup="dialog"
						onClick={(event) => {
							toggleAnchored("account", layoutKey, event.currentTarget)
						}}
						title={
							isIconOnly
								? `${activeAccount.username} (${t(ACCOUNT_TYPE_LABEL_KEYS[activeAccount.type])})`
								: undefined
						}
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
										{t(ACCOUNT_TYPE_LABEL_KEYS[activeAccount.type])}
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
