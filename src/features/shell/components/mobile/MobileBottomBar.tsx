import { MoreHorizontal, Play } from "lucide-react"
import { motion } from "motion/react"
import { useCallback, useMemo } from "react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import MobileBottomNavItem from "@/features/shell/components/mobile/MobileBottomNavItem"
import {
	PRIMARY_MOBILE_PATHS,
	getVisibleNavItems,
	isNavItemActive,
	type NavItem,
} from "@/features/shell/navigation"
import { useShellOverlayStore } from "@/features/shell/stores/useShellOverlayStore"
import { cn } from "@/shared/lib/utils"
import { selectActiveAccount, useAccountStore } from "@/shared/stores/useAccountStore"
import {
	COLOR_TAGS,
	selectSelectedInstance,
	useInstancesStore,
} from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

interface MobileBottomBarProps {
	currentPath: string
}

const MobileBottomBar = ({ currentPath }: MobileBottomBarProps) => {
	const { t } = useTranslation()
	const { showDeveloperPages, ramGb } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				showDeveloperPages: s.showDeveloperPages,
				ramGb: s.ramGb,
			}
		}),
	)
	const selectedInstance = useInstancesStore(selectSelectedInstance)
	const activeAccount = useAccountStore(selectActiveAccount)
	const { isLaunching, launchInstance } = useProcessStore(
		useShallow((s) => {
			return {
				isLaunching: s.isLaunching,
				launchInstance: s.launchInstance,
			}
		}),
	)
	const { moreOpen, toggleMore } = useShellOverlayStore(
		useShallow((s) => {
			return {
				moreOpen: s.activeOverlay.kind === "mobileMore",
				toggleMore: s.toggleMore,
			}
		}),
	)

	const allItems = useMemo(() => {
		return getVisibleNavItems(showDeveloperPages)
	}, [showDeveloperPages])

	const handleLaunch = useCallback(() => {
		if (!selectedInstance) {
			return
		}
		launchInstance({
			instance: selectedInstance,
			username: activeAccount.username,
			ramGb,
		})
	}, [activeAccount.username, launchInstance, ramGb, selectedInstance])

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
								{selectedInstance.version} · {t("shell.mobileRam", { ramGb })}
							</p>
						</div>
					</div>
					<button
						type="button"
						disabled={isLaunching}
						onClick={handleLaunch}
						className="flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3.5 text-[12px] font-semibold text-primary-foreground shadow-md shadow-primary/20 transition-all hover:opacity-90 disabled:opacity-60"
					>
						<Play className="size-3.5 fill-current" />
						<span>{isLaunching ? t("common.launching") : t("common.play")}</span>
					</button>
				</div>
			) : null}

			<nav
				aria-label={t("shell.mobileNavigationAria")}
				className="grid grid-cols-5 gap-1 px-2 py-1.5"
			>
				{primaryItems.map((item) => {
					const isActive = isNavItemActive(currentPath, item.path) && !moreOpen
					return <MobileBottomNavItem key={item.path} item={item} isActive={isActive} />
				})}

				<button
					type="button"
					onClick={toggleMore}
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
					<span className="relative z-10">{t("common.more")}</span>
				</button>
			</nav>
		</div>
	)
}

export default MobileBottomBar
