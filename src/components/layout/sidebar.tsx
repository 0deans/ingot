import { Link, useNavigate } from "@tanstack/react-router"
import { Camera, Gamepad2, Settings, Shirt, Sparkles } from "lucide-react"
import { memo } from "react"
import AccountSwitcher from "@/components/accounts/account-switcher"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"

const Sidebar = () => {
	const navigate = useNavigate()

	return (
		<aside className="flex w-16 flex-col items-center justify-between border-border/40 border-r bg-zinc-950/40 py-4 backdrop-blur-sm">
			<div className="flex flex-col items-center gap-3">
				<Tooltip>
					<TooltipTrigger
						render={
							<Link
								to="/"
								aria-label="Instances"
								activeProps={{
									className: "bg-primary/15 text-primary",
								}}
								inactiveProps={{
									className: "text-muted-foreground hover:bg-muted hover:text-foreground",
								}}
								className="inline-flex size-10 items-center justify-center rounded-xl transition-all"
							>
								<Gamepad2 className="size-5" />
							</Link>
						}
					/>
					<TooltipContent side="right">Instances</TooltipContent>
				</Tooltip>

				<Tooltip>
					<TooltipTrigger
						render={
							<Link
								to="/modpacks"
								aria-label="Discover Content"
								activeProps={{
									className: "bg-primary/15 text-primary",
								}}
								inactiveProps={{
									className: "text-muted-foreground hover:bg-muted hover:text-foreground",
								}}
								className="inline-flex size-10 items-center justify-center rounded-xl transition-all"
							>
								<Sparkles className="size-5" />
							</Link>
						}
					/>
					<TooltipContent side="right">Discover</TooltipContent>
				</Tooltip>

				<Tooltip>
					<TooltipTrigger
						render={
							<Link
								to="/skins"
								aria-label="Skins Catalog"
								activeProps={{
									className: "bg-primary/15 text-primary",
								}}
								inactiveProps={{
									className: "text-muted-foreground hover:bg-muted hover:text-foreground",
								}}
								className="inline-flex size-10 items-center justify-center rounded-xl transition-all"
							>
								<Shirt className="size-5" />
							</Link>
						}
					/>
					<TooltipContent side="right">Skins Catalog</TooltipContent>
				</Tooltip>

				<Tooltip>
					<TooltipTrigger
						render={
							<Link
								to="/screenshots"
								aria-label="Screenshots"
								activeProps={{
									className: "bg-primary/15 text-primary",
								}}
								inactiveProps={{
									className: "text-muted-foreground hover:bg-muted hover:text-foreground",
								}}
								className="inline-flex size-10 items-center justify-center rounded-xl transition-all"
							>
								<Camera className="size-5" />
							</Link>
						}
					/>
					<TooltipContent side="right">Screenshots</TooltipContent>
				</Tooltip>
			</div>

			<div className="flex flex-col items-center gap-3">
				<Tooltip>
					<TooltipTrigger
						render={
							<Link
								to="/settings"
								aria-label="Settings"
								activeProps={{
									className: "bg-primary/15 text-primary",
								}}
								inactiveProps={{
									className: "text-muted-foreground hover:bg-muted hover:text-foreground",
								}}
								className="inline-flex size-10 items-center justify-center rounded-xl transition-all"
							>
								<Settings className="size-5" />
							</Link>
						}
					/>
					<TooltipContent side="right">Settings</TooltipContent>
				</Tooltip>

				{/* Interactive Account Switcher with OS Keyring protection */}
				<AccountSwitcher compact onOpenSettings={() => navigate({ to: "/settings" })} />
			</div>
		</aside>
	)
}

Sidebar.displayName = "Sidebar"

export default memo(Sidebar)
