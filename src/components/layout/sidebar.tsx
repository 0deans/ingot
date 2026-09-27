import { Link, useNavigate, useRouterState } from "@tanstack/react-router"
import { Camera, Check, Gamepad2, Globe, Server, Settings, Shirt, Sparkles } from "lucide-react"
import { type ComponentProps, memo, type ReactNode, useCallback } from "react"
import AccountSwitcher from "@/components/accounts/account-switcher"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useLanguage } from "@/i18n/use-language"
import { resetScroll } from "@/lib/scroll-restoration"
import { getTabDestination, getTopLevelSection } from "@/lib/tab-history"

interface SidebarTabProps {
	to: NonNullable<ComponentProps<typeof Link>["to"]>
	label: string
	children: ReactNode
}

const SidebarTab = memo(({ to, label, children }: SidebarTabProps) => {
	const navigate = useNavigate()
	const currentPathname = useRouterState({ select: (s) => s.location.pathname })

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
						aria-label={label}
						onClick={handleClick}
						activeProps={{
							className: "bg-primary/15 text-primary",
						}}
						inactiveProps={{
							className: "text-muted-foreground hover:bg-muted hover:text-foreground",
						}}
						className="inline-flex size-10 items-center justify-center rounded-xl transition-all"
					>
						{children}
					</Link>
				}
			/>
			<TooltipContent side="right">{label}</TooltipContent>
		</Tooltip>
	)
})

SidebarTab.displayName = "SidebarTab"

const Sidebar = () => {
	const navigate = useNavigate()
	const currentPathname = useRouterState({ select: (s) => s.location.pathname })
	const { language, locales, setLanguage, t } = useLanguage()

	const handleOpenSettings = useCallback(() => {
		const destination = getTabDestination("/settings", currentPathname)
		if (destination !== "/settings") {
			navigate({ href: destination })
		} else {
			navigate({ to: "/settings" })
		}
	}, [currentPathname, navigate])

	return (
		<aside className="flex w-16 flex-col items-center justify-between border-border/40 border-r bg-zinc-950/40 py-4 backdrop-blur-sm">
			<div className="flex flex-col items-center gap-3">
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
			</div>

			<div className="flex flex-col items-center gap-3">
				{/* Quick Language Switcher */}
				<DropdownMenu>
					<Tooltip>
						<TooltipTrigger
							render={
								<DropdownMenuTrigger
									aria-label={t("nav.language")}
									className="inline-flex size-10 items-center justify-center rounded-xl text-muted-foreground outline-none transition-all hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-primary/40"
								>
									<Globe className="size-5" />
								</DropdownMenuTrigger>
							}
						/>
						<TooltipContent side="right">{t("nav.language")}</TooltipContent>
					</Tooltip>
					<DropdownMenuContent
						align="end"
						side="right"
						sideOffset={8}
						className="max-h-80 w-48 overflow-y-auto border-border/60 bg-zinc-950/95 p-1.5 shadow-xl backdrop-blur-md"
					>
						<DropdownMenuGroup>
							<DropdownMenuLabel className="text-[11px]">{t("nav.language")}</DropdownMenuLabel>
							{locales.map((loc) => {
								const isSelected = language === loc.code
								return (
									<DropdownMenuItem
										key={loc.code}
										onClick={() => setLanguage(loc.code)}
										className="flex items-center justify-between gap-2 py-1.5"
									>
										<div className="flex items-center gap-2">
											<span className="rounded bg-zinc-900 px-1.5 py-0.5 font-mono text-[10px] text-zinc-400 uppercase">
												{loc.code}
											</span>
											<div className="flex flex-col">
												<span className="font-medium text-xs">{loc.nativeName}</span>
												<span className="text-[10px] text-muted-foreground">{loc.name}</span>
											</div>
										</div>
										{isSelected && <Check className="size-3.5 text-primary" />}
									</DropdownMenuItem>
								)
							})}
						</DropdownMenuGroup>
					</DropdownMenuContent>
				</DropdownMenu>

				<SidebarTab to="/settings" label={t("nav.settings")}>
					<Settings className="size-5" />
				</SidebarTab>

				{/* Interactive Account Switcher with OS Keyring protection */}
				<AccountSwitcher compact onOpenSettings={handleOpenSettings} />
			</div>
		</aside>
	)
}

Sidebar.displayName = "Sidebar"

export default memo(Sidebar)
