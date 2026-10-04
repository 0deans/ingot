import { Link, useNavigate, useRouter, useRouterState } from "@tanstack/react-router"
import { cn } from "cn"
import { Camera, Gamepad2, Palette, Server, Settings, Shirt, Sparkles } from "lucide-react"
import { type ComponentProps, memo, type ReactNode, useCallback } from "react"
import AccountSwitcher from "@/components/accounts/account-switcher"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useLanguage } from "@/i18n/use-language"
import { resetScroll } from "@/lib/scroll-restoration"
import { getTabDestination, getTopLevelSection } from "@/lib/tab-history"

interface SidebarTabProps {
	to: NonNullable<ComponentProps<typeof Link>["to"]>
	label: string
	children: ReactNode
	showActivePill?: boolean
}

const SidebarTab = memo(({ to, label, children, showActivePill = false }: SidebarTabProps) => {
	const navigate = useNavigate()
	const router = useRouter()
	const currentPathname = useRouterState({ select: (s) => s.location.pathname })
	const currentSection = getTopLevelSection(currentPathname)
	const isActive = currentSection === to

	const handleIntent = useCallback(() => {
		const destination = getTabDestination(to, currentPathname)
		if (destination !== to) {
			router
				.preloadRoute({ href: destination } as unknown as Parameters<typeof router.preloadRoute>[0])
				.catch(() => {})
		}
	}, [router, to, currentPathname])

	const handleClick = useCallback(
		(e: React.MouseEvent) => {
			e.preventDefault()
			const currentSection = getTopLevelSection(currentPathname)
			if (currentSection === to) {
				resetScroll(to)
				const activeViewport = document.querySelector<HTMLElement>(
					'[data-slot="scroll-area-viewport"], .overflow-y-auto',
				)
				if (activeViewport) {
					activeViewport.scrollTo({ top: 0, behavior: "smooth" })
				}
				if (currentPathname !== to || window.location.search) {
					navigate({ to })
				}
			} else {
				const destination = getTabDestination(to, currentPathname)
				navigate({ href: destination })
			}
		},
		[to, currentPathname, navigate],
	)

	return (
		<Tooltip>
			<TooltipTrigger
				render={
					<Link
						to={to}
						preload="intent"
						preloadDelay={50}
						onMouseEnter={handleIntent}
						onFocus={handleIntent}
						aria-label={label}
						onClick={handleClick}
						className={cn(
							"group relative inline-flex size-10 items-center justify-center rounded-xl transition-colors",
							isActive
								? "text-primary"
								: "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
						)}
					>
						{showActivePill && isActive && (
							<div className="absolute inset-0 rounded-xl bg-primary/15" />
						)}
						<span className="relative z-10 flex items-center justify-center transition-transform active:scale-90">
							{children}
						</span>
					</Link>
				}
			/>
			<TooltipContent side="right">{label}</TooltipContent>
		</Tooltip>
	)
})

SidebarTab.displayName = "SidebarTab"

const TOP_NAV_TABS = [
	"/",
	"/modpacks",
	"/skins",
	"/screenshots",
	"/servers",
	...(import.meta.env.DEV ? ["/ui-test"] : []),
]

const Sidebar = () => {
	const navigate = useNavigate()
	const router = useRouter()
	const currentPathname = useRouterState({ select: (s) => s.location.pathname })
	const { t } = useLanguage()

	const currentSection = getTopLevelSection(currentPathname)
	const activeIndex = TOP_NAV_TABS.indexOf(currentSection)

	const handleOpenSettings = useCallback(() => {
		const destination = getTabDestination("/settings", currentPathname)
		if (destination !== "/settings") {
			navigate({ href: destination })
		} else {
			navigate({ to: "/settings" })
		}
	}, [currentPathname, navigate])

	const handlePreloadSettings = useCallback(() => {
		const destination = getTabDestination("/settings", currentPathname)
		if (destination !== "/settings") {
			router
				.preloadRoute({ href: destination } as unknown as Parameters<typeof router.preloadRoute>[0])
				.catch(() => {})
		} else {
			router.preloadRoute({ to: "/settings" }).catch(() => {})
		}
	}, [router, currentPathname])

	return (
		<aside className="flex w-16 flex-col items-center justify-between border-border/40 border-r bg-background/40 py-4 backdrop-blur-sm">
			<div className="relative flex flex-col items-center gap-3">
				{/* GPU-accelerated gliding active pill indicator */}
				<div
					aria-hidden="true"
					className={cn(
						"pointer-events-none absolute top-0 left-0 size-10 transform-gpu rounded-xl bg-primary/15 transition-all duration-200 ease-[cubic-bezier(0.16,1,0.3,1)]",
						activeIndex >= 0 ? "opacity-100" : "scale-90 opacity-0",
					)}
					style={{
						transform: activeIndex >= 0 ? `translateY(${activeIndex * 52}px)` : undefined,
					}}
				/>

				<SidebarTab to="/" label={t("nav.instances")}>
					<Gamepad2 className="size-5" />
				</SidebarTab>

				<SidebarTab to="/modpacks" label={t("nav.discover")}>
					<Sparkles className="size-5" />
				</SidebarTab>

				<SidebarTab to="/skins" label={t("nav.skins")}>
					<Shirt className="size-5" />
				</SidebarTab>

				<SidebarTab to="/screenshots" label={t("nav.screenshots")}>
					<Camera className="size-5" />
				</SidebarTab>

				<SidebarTab to="/servers" label={t("nav.servers")}>
					<Server className="size-5" />
				</SidebarTab>

				{import.meta.env.DEV && (
					<SidebarTab to="/ui-test" label="UI Showcase">
						<Palette className="size-5" />
					</SidebarTab>
				)}
			</div>

			<div className="flex flex-col items-center gap-3">
				<SidebarTab to="/settings" label={t("nav.settings")} showActivePill>
					<Settings className="size-5" />
				</SidebarTab>

				{/* Interactive Account Switcher with OS Keyring protection */}
				<AccountSwitcher
					compact
					onOpenSettings={handleOpenSettings}
					onPreloadSettings={handlePreloadSettings}
				/>
			</div>
		</aside>
	)
}

Sidebar.displayName = "Sidebar"

export default memo(Sidebar)
