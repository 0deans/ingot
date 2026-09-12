import {
	FolderOpen,
	Gamepad2,
	HardDrive,
	Play,
	Plus,
	Search,
	Settings,
	Sparkles,
	User,
} from "lucide-react";
import { useState } from "react";
import { Titlebar } from "@/components/titlebar";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

const App = () => {
	const [searchQuery, setSearchQuery] = useState("");
	const [isNewInstanceOpen, setIsNewInstanceOpen] = useState(false);
	const [instanceName, setInstanceName] = useState("");

	return (
		<TooltipProvider>
			<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-radial-[at_top_center] from-zinc-900/60 via-zinc-950 to-black text-foreground antialiased selection:bg-primary/20">
				{/* Custom Window Titlebar */}
				<Titlebar title="Ingot" />

				{/* App Body */}
				<div className="flex flex-1 overflow-hidden">
					{/* Sidebar Navigation */}
					<aside className="flex w-16 flex-col items-center justify-between border-border/40 border-r bg-zinc-950/40 py-4 backdrop-blur-sm">
						<div className="flex flex-col items-center gap-3">
							<Tooltip>
								<TooltipTrigger
									aria-label="Instances"
									className="inline-flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary transition-all hover:bg-primary/20"
								>
									<Gamepad2 className="size-5" />
								</TooltipTrigger>
								<TooltipContent side="right">Instances</TooltipContent>
							</Tooltip>

							<Tooltip>
								<TooltipTrigger
									aria-label="Storage & Mods"
									className="inline-flex size-10 items-center justify-center rounded-xl text-muted-foreground transition-all hover:bg-muted hover:text-foreground"
								>
									<HardDrive className="size-5" />
								</TooltipTrigger>
								<TooltipContent side="right">Storage</TooltipContent>
							</Tooltip>

							<Tooltip>
								<TooltipTrigger
									aria-label="Browse Modpacks"
									className="inline-flex size-10 items-center justify-center rounded-xl text-muted-foreground transition-all hover:bg-muted hover:text-foreground"
								>
									<Sparkles className="size-5" />
								</TooltipTrigger>
								<TooltipContent side="right">Modpacks</TooltipContent>
							</Tooltip>
						</div>

						<div className="flex flex-col items-center gap-3">
							<Tooltip>
								<TooltipTrigger
									aria-label="Settings"
									className="inline-flex size-10 items-center justify-center rounded-xl text-muted-foreground transition-all hover:bg-muted hover:text-foreground"
								>
									<Settings className="size-5" />
								</TooltipTrigger>
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
										<span className="font-normal text-muted-foreground text-xs">
											Microsoft Account
										</span>
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

					{/* Main Launcher Content Area */}
					<main className="flex flex-1 flex-col overflow-y-auto p-6">
						{/* Top search and actions header */}
						<div className="flex items-center justify-between gap-4">
							<div className="relative max-w-sm flex-1">
								<Search className="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
								<Input
									value={searchQuery}
									onChange={(e) => setSearchQuery(e.target.value)}
									placeholder="Search instances..."
									className="border-border/50 bg-zinc-900/50 pl-9 focus-visible:ring-1"
								/>
							</div>

							{/* New Instance Dialog */}
							<Dialog open={isNewInstanceOpen} onOpenChange={setIsNewInstanceOpen}>
								<DialogTrigger render={<Button className="gap-2" />}>
									<Plus className="size-4" />
									New Instance
								</DialogTrigger>
								<DialogContent className="sm:max-w-md">
									<DialogHeader>
										<DialogTitle>Create New Instance</DialogTitle>
										<DialogDescription>
											Configure a new Minecraft installation with your preferred version and mod
											loader.
										</DialogDescription>
									</DialogHeader>
									<div className="grid gap-4 py-4">
										<div className="grid gap-2">
											<label
												htmlFor="instance-name"
												className="font-medium text-muted-foreground text-xs"
											>
												Instance Name
											</label>
											<Input
												id="instance-name"
												value={instanceName}
												onChange={(e) => setInstanceName(e.target.value)}
												placeholder="e.g. Vanilla 1.21.4"
											/>
										</div>
									</div>
									<DialogFooter>
										<Button variant="outline" onClick={() => setIsNewInstanceOpen(false)}>
											Cancel
										</Button>
										<Button
											onClick={() => {
												setIsNewInstanceOpen(false);
												setInstanceName("");
											}}
										>
											Create
										</Button>
									</DialogFooter>
								</DialogContent>
							</Dialog>
						</div>

						{/* Hero Instance Showcase Card */}
						<div className="mt-6 flex flex-1 flex-col justify-between rounded-2xl border border-border/40 bg-gradient-to-br from-zinc-900/80 via-zinc-900/40 to-zinc-950/90 p-8 shadow-2xl backdrop-blur-md">
							<div className="flex flex-col gap-2">
								<div className="flex items-center gap-2">
									<span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2.5 py-0.5 font-medium text-emerald-400 text-xs">
										<span className="size-1.5 animate-pulse rounded-full bg-emerald-400" />
										Ready to Play
									</span>
									<span className="text-muted-foreground text-xs">Fabric 0.16.9 • 1.21.4</span>
								</div>

								<h1 className="font-bold text-3xl text-white tracking-tight sm:text-4xl">
									Minecraft 1.21.4
								</h1>
								<p className="max-w-xl text-muted-foreground text-sm leading-relaxed">
									Fast, lightweight Minecraft launcher built with Tauri and React. High-performance
									gaming environment powered by Ingot.
								</p>
							</div>

							{/* Bottom Action Bar */}
							<div className="mt-8 flex items-center justify-between border-border/30 border-t pt-6">
								<div className="flex items-center gap-6">
									<div>
										<div className="text-muted-foreground text-xs uppercase tracking-wider">
											Memory
										</div>
										<div className="font-semibold text-foreground text-sm">4.0 GB / 16.0 GB</div>
									</div>
									<div className="h-8 w-px bg-border/40" />
									<div>
										<div className="text-muted-foreground text-xs uppercase tracking-wider">
											Java Runtime
										</div>
										<div className="font-semibold text-foreground text-sm">Java 21 (Temurin)</div>
									</div>
								</div>

								<Button
									size="lg"
									className="gap-2.5 px-8 font-semibold text-base shadow-lg shadow-primary/20 transition-all hover:scale-[1.02] active:scale-[0.98]"
								>
									<Play className="size-5 fill-current" />
									Play
								</Button>
							</div>
						</div>
					</main>
				</div>
			</div>
		</TooltipProvider>
	);
};

export default App;
