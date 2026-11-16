import { Outlet, useRouterState } from "@tanstack/react-router"
import { AnimatePresence } from "motion/react"
import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from "react"
import { useShallow } from "zustand/react/shallow"
import DockedLaunchBar from "@/features/play/components/DockedLaunchBar"
import AccountSwitcherPopover, {
	type AccountPopoverAnchor,
	type PopoverAnchorRect,
} from "@/features/shell/components/AccountSwitcherPopover"
import ActiveProcessesPopover from "@/features/shell/components/ActiveProcessesPopover"
import DesktopSidebar from "@/features/shell/components/DesktopSidebar"
import MobileBottomBar from "@/features/shell/components/MobileBottomBar"
import MobileMoreSheet from "@/features/shell/components/MobileMoreSheet"
import MobileTopBar from "@/features/shell/components/MobileTopBar"
import WindowsTitlebar from "@/features/shell/components/WindowsTitlebar"
import { useBreakpoint } from "@/features/shell/hooks/useBreakpoint"
import {
	PRIMARY_MOBILE_PATHS,
	getPageTransitionKey,
	getVisibleNavItems,
} from "@/features/shell/navigation"
import PageTransition from "@/shared/components/motion/PageTransition"
import { useScrollRestoration } from "@/shared/hooks/useScrollRestoration"
import { cn } from "@/shared/lib/utils"
import { findActiveAccount, useAccountStore } from "@/shared/stores/useAccountStore"
import { selectSelectedInstance, useInstancesStore } from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

interface AnchoredPopoverState {
	layoutKey: string | null
	anchorRect: PopoverAnchorRect | null
}

const CLOSED_POPOVER_STATE: AnchoredPopoverState = {
	layoutKey: null,
	anchorRect: null,
}

const AppShell = () => {
	const pathname = useRouterState({
		select: (state) => {
			return state.location.pathname
		},
	})
	const breakpoint = useBreakpoint()

	const mainRef = useRef<HTMLElement | null>(null)
	useScrollRestoration(mainRef, pathname)

	const { navVariant, showDeveloperPages, ramGb, railExpanded, toggleRailExpanded } =
		useLauncherPrefsStore(
			useShallow((s) => {
				return {
					navVariant: s.navVariant,
					showDeveloperPages: s.showDeveloperPages,
					ramGb: s.ramGb,
					railExpanded: s.railExpanded,
					toggleRailExpanded: s.toggleRailExpanded,
				}
			}),
		)

	const { accounts, activeAccountId, setActiveAccountId, addAccount } = useAccountStore(
		useShallow((s) => {
			return {
				accounts: s.accounts,
				activeAccountId: s.activeAccountId,
				setActiveAccountId: s.setActiveAccountId,
				addAccount: s.addAccount,
			}
		}),
	)

	const selectedInstance = useInstancesStore(selectSelectedInstance)

	const {
		runningProcesses,
		isLaunching,
		launchInstance,
		stopProcess,
		stopAllProcesses,
		spawnMockProcess,
	} = useProcessStore(
		useShallow((s) => {
			return {
				runningProcesses: s.runningProcesses,
				isLaunching: s.isLaunching,
				launchInstance: s.launchInstance,
				stopProcess: s.stopProcess,
				stopAllProcesses: s.stopAllProcesses,
				spawnMockProcess: s.spawnMockProcess,
			}
		}),
	)

	const [processesPopover, setProcessesPopover] =
		useState<AnchoredPopoverState>(CLOSED_POPOVER_STATE)
	const [accountPopover, setAccountPopover] = useState<AnchoredPopoverState>(CLOSED_POPOVER_STATE)
	const [moreOpen, setMoreOpen] = useState(false)

	const activeAccount = useMemo(() => {
		return findActiveAccount(accounts, activeAccountId)
	}, [accounts, activeAccountId])

	const totalProcessRamMb = useMemo(() => {
		return runningProcesses.reduce((sum, proc) => {
			const match = /^([\d.]+)\s*(GB|MB)/i.exec(proc.ram)
			if (!match) {
				return sum
			}
			const numeric = Number(match[1])
			if (Number.isNaN(numeric)) {
				return sum
			}
			const unit = match[2]?.toUpperCase()
			if (unit === "GB") {
				return sum + Math.round(numeric * 1024)
			}

			return sum + Math.round(numeric)
		}, 0)
	}, [runningProcesses])

	const visibleNavItems = useMemo(() => {
		return getVisibleNavItems(showDeveloperPages)
	}, [showDeveloperPages])

	const secondaryMobileItems = useMemo(() => {
		return visibleNavItems.filter((item) => {
			return !PRIMARY_MOBILE_PATHS.includes(item.path)
		})
	}, [visibleNavItems])

	const isMobile = breakpoint === "mobile"
	const isTablet = breakpoint === "tablet"
	const currentLayoutKey = `${navVariant}:${railExpanded ? "1" : "0"}:${isMobile ? "1" : "0"}`
	const processesOpen = processesPopover.layoutKey === currentLayoutKey
	const accountOpen = accountPopover.layoutKey === currentLayoutKey

	const accountAnchorMode: AccountPopoverAnchor = useMemo(() => {
		if (isMobile) {
			return "mobile"
		}
		if (navVariant === "capsule") {
			return "capsule"
		}
		if (navVariant === "rail" && !railExpanded) {
			return "rail"
		}
		return "sidebar"
	}, [isMobile, navVariant, railExpanded])

	const handleToggleProcesses = useCallback(
		(event: MouseEvent<HTMLButtonElement>) => {
			const rect = event.currentTarget.getBoundingClientRect()
			const anchorRect: PopoverAnchorRect = {
				top: rect.top,
				bottom: rect.bottom,
				left: rect.left,
				right: rect.right,
			}
			setAccountPopover(CLOSED_POPOVER_STATE)
			setMoreOpen(false)
			setProcessesPopover((prev) => {
				if (prev.layoutKey === currentLayoutKey) {
					return CLOSED_POPOVER_STATE
				}
				return { layoutKey: currentLayoutKey, anchorRect }
			})
		},
		[currentLayoutKey],
	)

	const handleToggleAccount = useCallback(
		(event: MouseEvent<HTMLButtonElement>) => {
			const rect = event.currentTarget.getBoundingClientRect()
			const anchorRect: PopoverAnchorRect = {
				top: rect.top,
				bottom: rect.bottom,
				left: rect.left,
				right: rect.right,
			}
			setProcessesPopover(CLOSED_POPOVER_STATE)
			setMoreOpen(false)
			setAccountPopover((prev) => {
				if (prev.layoutKey === currentLayoutKey) {
					return CLOSED_POPOVER_STATE
				}
				return { layoutKey: currentLayoutKey, anchorRect }
			})
		},
		[currentLayoutKey],
	)

	const handleToggleMore = useCallback(() => {
		setProcessesPopover(CLOSED_POPOVER_STATE)
		setAccountPopover(CLOSED_POPOVER_STATE)
		setMoreOpen((prev) => {
			return !prev
		})
	}, [])

	const handleCloseProcesses = useCallback(() => {
		setProcessesPopover(CLOSED_POPOVER_STATE)
	}, [])

	const handleCloseAccount = useCallback(() => {
		setAccountPopover(CLOSED_POPOVER_STATE)
	}, [])

	const handleCloseMore = useCallback(() => {
		setMoreOpen(false)
	}, [])

	useEffect(() => {
		if (!processesOpen && !accountOpen) {
			return
		}
		const handleResize = () => {
			setProcessesPopover(CLOSED_POPOVER_STATE)
			setAccountPopover(CLOSED_POPOVER_STATE)
		}
		window.addEventListener("resize", handleResize)
		return () => {
			window.removeEventListener("resize", handleResize)
		}
	}, [accountOpen, processesOpen])

	const handleLaunchSelected = useCallback(() => {
		if (!selectedInstance) {
			return
		}
		launchInstance({
			instance: selectedInstance,
			username: activeAccount.username,
			ramGb,
		})
	}, [activeAccount.username, launchInstance, ramGb, selectedInstance])

	const handleSpawnTestProcess = useCallback(() => {
		spawnMockProcess()
	}, [spawnMockProcess])

	const showDesktopSidebar = !isMobile && navVariant !== "capsule"
	const isSequoiaDesktop = showDesktopSidebar && navVariant === "sequoia"
	const pageTransitionKey = getPageTransitionKey(pathname)
	const baseColumnClass =
		"flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden border transition-[margin,border-radius,border-color,background-color] duration-[240ms] ease-[cubic-bezier(0.22,1,0.36,1)]"
	const sequoiaColumnClass = cn(
		baseColumnClass,
		"m-2.5 rounded-2xl border-border bg-card/75 shadow-xl backdrop-blur-xl",
	)
	const standardColumnClass = cn(baseColumnClass, "rounded-none border-transparent bg-transparent")
	const contentColumnClass = isSequoiaDesktop ? sequoiaColumnClass : standardColumnClass

	return (
		<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
			<div className="relative z-10 flex min-h-0 flex-1 flex-col overflow-hidden">
				{isMobile ? (
					<MobileTopBar
						currentPath={pathname}
						activeProcessesCount={runningProcesses.length}
						activeAccount={activeAccount}
						onToggleProcesses={handleToggleProcesses}
						onToggleAccount={handleToggleAccount}
					/>
				) : (
					<WindowsTitlebar
						navVariant={navVariant}
						showDeveloperPages={showDeveloperPages}
						currentPath={pathname}
						activeProcessesCount={runningProcesses.length}
						totalProcessRamMb={totalProcessRamMb}
						processesOpen={processesOpen}
						accountOpen={accountOpen}
						activeAccount={activeAccount}
						onToggleProcesses={handleToggleProcesses}
						onToggleAccount={handleToggleAccount}
					/>
				)}

				<div className="flex min-h-0 flex-1 overflow-hidden">
					<AnimatePresence initial={false}>
						{showDesktopSidebar ? (
							<DesktopSidebar
								key="desktop-sidebar"
								variant={navVariant}
								compact={isTablet}
								railExpanded={railExpanded}
								showDeveloperPages={showDeveloperPages}
								currentPath={pathname}
								activeAccount={activeAccount}
								ramGb={ramGb}
								runningCount={runningProcesses.length}
								accountOpen={accountOpen}
								onToggleRailExpanded={toggleRailExpanded}
								onToggleAccount={handleToggleAccount}
							/>
						) : null}
					</AnimatePresence>

					<div className={contentColumnClass}>
						<main
							ref={mainRef}
							style={{ overflowAnchor: "none", scrollbarGutter: "stable" }}
							className="custom-scrollbar min-h-0 min-w-0 flex-1 scroll-fade-y overflow-x-clip overflow-y-auto px-4 py-4 md:px-6 md:py-5"
						>
							<PageTransition routeKey={pageTransitionKey}>
								<Outlet />
							</PageTransition>
						</main>

						{!isMobile ? (
							<AnimatePresence initial={false}>
								{pathname === "/" && selectedInstance ? (
									<DockedLaunchBar
										instance={selectedInstance}
										username={activeAccount.username}
										ramGb={ramGb}
										isLaunching={isLaunching}
										onLaunch={handleLaunchSelected}
									/>
								) : null}
							</AnimatePresence>
						) : null}
					</div>
				</div>

				{isMobile ? (
					<MobileBottomBar
						currentPath={pathname}
						allItems={visibleNavItems}
						moreOpen={moreOpen}
						selectedInstance={selectedInstance}
						ramGb={ramGb}
						isLaunching={isLaunching}
						onLaunch={handleLaunchSelected}
						onToggleMore={handleToggleMore}
					/>
				) : null}
			</div>

			<ActiveProcessesPopover
				open={processesOpen}
				anchorRect={processesPopover.anchorRect}
				processes={runningProcesses}
				onClose={handleCloseProcesses}
				onStopProcess={stopProcess}
				onStopAll={stopAllProcesses}
				onSpawnTest={handleSpawnTestProcess}
			/>

			<AccountSwitcherPopover
				open={accountOpen}
				anchorMode={accountAnchorMode}
				anchorRect={accountPopover.anchorRect}
				accounts={accounts}
				activeAccountId={activeAccount.id}
				onClose={handleCloseAccount}
				onSelectAccount={setActiveAccountId}
				onAddAccount={addAccount}
			/>

			<MobileMoreSheet
				open={moreOpen}
				currentPath={pathname}
				items={secondaryMobileItems}
				onClose={handleCloseMore}
			/>
		</div>
	)
}

export default AppShell
