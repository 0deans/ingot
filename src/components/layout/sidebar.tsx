import { Link } from "@tanstack/react-router"
import { FolderOpen, Gamepad2, HardDrive, Settings, Sparkles, User } from "lucide-react"
import { memo } from "react"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"

const Sidebar = () => {
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
								to="/storage"
								aria-label="Storage & Mods"
								activeProps={{
									className: "bg-primary/15 text-primary",
								}}
								inactiveProps={{
									className: "text-muted-foreground hover:bg-muted hover:text-foreground",
								}}
								className="inline-flex size-10 items-center justify-center rounded-xl transition-all"
							>
								<HardDrive className="size-5" />
							</Link>
						}
					/>
					<TooltipContent side="right">Storage</TooltipContent>
				</Tooltip>

				<Tooltip>
					<TooltipTrigger
						render={
							<Link
								to="/modpacks"
								aria-label="Browse Modpacks"
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
					<TooltipContent side="right">Modpacks</TooltipContent>
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

				{/* User Profile Dropdown */}
				<DropdownMenu>
					<DropdownMenuTrigger aria-label="User profile" className="outline-none">
						<Avatar className="size-8.5 ring-2 ring-primary/20 transition-all hover:ring-primary/50">
							<AvatarImage
								src="https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=100&auto=format&fit=crop&q=80"
								alt="Player"
							/>
							<AvatarFallback>
								<User className="size-4" />
							</AvatarFallback>
						</Avatar>
					</DropdownMenuTrigger>
					<DropdownMenuContent align="end" side="right" className="w-48">
						<DropdownMenuLabel className="flex flex-col">
							<span className="font-semibold text-sm">PlayerOne</span>
							<span className="font-normal text-muted-foreground text-xs">Microsoft Account</span>
						</DropdownMenuLabel>
						<DropdownMenuSeparator />
						<DropdownMenuItem>
							<User className="mr-2 size-4" />
							Switch Account
						</DropdownMenuItem>
						<DropdownMenuItem>
							<FolderOpen className="mr-2 size-4" />
							Launcher Folder
						</DropdownMenuItem>
						<DropdownMenuSeparator />
						<DropdownMenuItem className="text-destructive focus:text-destructive">
							Sign Out
						</DropdownMenuItem>
					</DropdownMenuContent>
				</DropdownMenu>
			</div>
		</aside>
	)
}

Sidebar.displayName = "Sidebar"

export default memo(Sidebar)
