import { Outlet, useRouterState } from "@tanstack/react-router"
import { AnimatePresence } from "motion/react"
import { useCallback, useMemo, useRef } from "react"
import { useShallow } from "zustand/react/shallow"
import OnboardingFlow from "@/features/onboarding/components/OnboardingFlow"
import DockedLaunchBar from "@/features/play/components/DockedLaunchBar"
import DesktopSidebar from "@/features/shell/components/desktop/DesktopSidebar"
import SidebarErrorFallback from "@/features/shell/components/desktop/SidebarErrorFallback"
import WindowsTitlebar from "@/features/shell/components/desktop/WindowsTitlebar"
import MobileBottomBar from "@/features/shell/components/mobile/MobileBottomBar"
import MobileTopBar from "@/features/shell/components/mobile/MobileTopBar"
import ShellOverlays from "@/features/shell/components/ShellOverlays"
import { useBreakpoint } from "@/features/shell/hooks/useBreakpoint"
import { getPageTransitionKey } from "@/features/shell/navigation"
import ErrorBoundary from "@/shared/components/errors/ErrorBoundary"
import ErrorFallbackView from "@/shared/components/errors/ErrorFallbackView"
import PageTransition from "@/shared/components/motion/PageTransition"
import SafeTitlebar from "@/shared/components/window/SafeTitlebar"
import { useScrollRestoration } from "@/shared/hooks/useScrollRestoration"
import { cn } from "@/shared/lib/utils"
import { findActiveAccount, useAccountStore } from "@/shared/stores/useAccountStore"
import { selectSelectedInstance, useInstancesStore } from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

const AppShell = () => {
	const pathname = useRouterState({
		select: (state) => {
			return state.location.pathname
		},
	})
	const breakpoint = useBreakpoint()
	const isMobile = breakpoint === "mobile"
	const isTablet = breakpoint === "tablet"

	const { navVariant, ramGb, railExpanded, onboardingCompleted } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				navVariant: s.navVariant,
				ramGb: s.ramGb,
				railExpanded: s.railExpanded,
				onboardingCompleted: s.onboardingCompleted,
			}
		}),
	)
	const { accounts, activeAccountId } = useAccountStore(
		useShallow((s) => {
			return {
				accounts: s.accounts,
				activeAccountId: s.activeAccountId,
			}
		}),
	)
	const selectedInstance = useInstancesStore(selectSelectedInstance)
	const { isLaunching, launchInstance } = useProcessStore(
		useShallow((s) => {
			return {
				isLaunching: s.isLaunching,
				launchInstance: s.launchInstance,
			}
		}),
	)

	const mainRef = useRef<HTMLElement | null>(null)
	useScrollRestoration(mainRef, pathname)

	const activeAccount = useMemo(() => {
		return findActiveAccount(accounts, activeAccountId)
	}, [accounts, activeAccountId])

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

	const currentLayoutKey = `${navVariant}:${railExpanded ? "1" : "0"}:${isMobile ? "1" : "0"}`
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
					<ErrorBoundary
						resetKey={currentLayoutKey}
						fallback={<SafeTitlebar statusLabel="Recovery" />}
					>
						<MobileTopBar currentPath={pathname} layoutKey={currentLayoutKey} />
					</ErrorBoundary>
				) : (
					<ErrorBoundary
						resetKey={currentLayoutKey}
						fallback={<SafeTitlebar statusLabel="Titlebar Fallback" />}
					>
						<WindowsTitlebar currentPath={pathname} layoutKey={currentLayoutKey} />
					</ErrorBoundary>
				)}

				<div className="flex min-h-0 flex-1 overflow-hidden">
					<ErrorBoundary
						resetKey={currentLayoutKey}
						fallback={({ reset }) => {
							return <SidebarErrorFallback onRetry={reset} />
						}}
					>
						<AnimatePresence initial={false}>
							{showDesktopSidebar ? (
								<DesktopSidebar
									key="desktop-sidebar"
									variant={navVariant}
									compact={isTablet}
									currentPath={pathname}
									layoutKey={currentLayoutKey}
								/>
							) : null}
						</AnimatePresence>
					</ErrorBoundary>

					<div className={contentColumnClass}>
						<main
							ref={mainRef}
							style={{ overflowAnchor: "none", scrollbarGutter: "stable" }}
							className="custom-scrollbar min-h-0 min-w-0 flex-1 scroll-fade-y overflow-x-clip overflow-y-auto px-4 py-4 md:px-6 md:py-5"
						>
							<ErrorBoundary
								resetKey={pathname}
								fallback={({ error, reset }) => {
									return <ErrorFallbackView error={error} reset={reset} />
								}}
							>
								<PageTransition routeKey={pageTransitionKey}>
									<Outlet />
								</PageTransition>
							</ErrorBoundary>
						</main>

						{!isMobile ? (
							<ErrorBoundary resetKey={currentLayoutKey} fallback={null}>
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
							</ErrorBoundary>
						) : null}
					</div>
				</div>

				{isMobile ? (
					<ErrorBoundary resetKey={pathname} fallback={null}>
						<MobileBottomBar currentPath={pathname} />
					</ErrorBoundary>
				) : null}
			</div>

			<ErrorBoundary resetKey={currentLayoutKey} fallback={null}>
				<ShellOverlays currentPath={pathname} layoutKey={currentLayoutKey} isMobile={isMobile} />
			</ErrorBoundary>

			<ErrorBoundary resetKey={onboardingCompleted ? "done" : "open"} fallback={null}>
				<AnimatePresence>
					{!onboardingCompleted ? (
						<OnboardingFlow key="onboarding-flow" isMobile={isMobile} />
					) : null}
				</AnimatePresence>
			</ErrorBoundary>
		</div>
	)
}

export default AppShell
