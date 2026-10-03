import {
	Download,
	ExternalLink,
	FolderOpen,
	Gamepad2,
	MoreVertical,
	Play,
	RefreshCw,
	Sparkles,
} from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import LoaderIcon from "@/components/instances/loader-icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Spinner } from "@/components/ui/spinner"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { useBedrockStatus } from "@/services/bedrock-service"

export function BedrockCard({ className }: { className?: string }) {
	const { t } = useTranslation()
	const { status, isLoading, isLaunching, launch, install, openFolder, refresh } =
		useBedrockStatus()
	const [isRefreshing, setIsRefreshing] = useState(false)

	const handleRefresh = async () => {
		setIsRefreshing(true)
		try {
			await refresh()
		} finally {
			setIsRefreshing(false)
		}
	}

	if (status && status.platform !== "windows" && status.platform !== "android") {
		return null
	}

	const isInstalled = status?.isInstalled ?? false
	const version = status?.version
	const edition = status?.edition

	return (
		<div
			className={cn(
				"relative flex flex-col justify-between overflow-hidden rounded-2xl border p-4 transition-all duration-300 sm:p-5",
				isInstalled
					? "border-primary/30 bg-linear-to-br from-card/80 via-card/50 to-primary/5 shadow-md hover:border-primary/50"
					: "border-border/60 bg-linear-to-br from-card/60 via-card/30 to-muted/10 hover:border-border/90",
				className,
			)}
		>
			{/* Subtle decorative glow */}
			<div className="pointer-events-none absolute -top-12 -right-12 size-36 rounded-full bg-primary/10 blur-3xl" />

			{/* Top Header */}
			<div className="flex items-start justify-between gap-3">
				<div className="flex items-center gap-3">
					<div className="flex size-12 shrink-0 items-center justify-center rounded-2xl border border-border/80 bg-card shadow-sm backdrop-blur-sm">
						<LoaderIcon loader="bedrock" size={30} />
					</div>
					<div className="min-w-0">
						<div className="flex items-center gap-2">
							<h3 className="truncate font-semibold text-foreground text-sm sm:text-base">
								{t("bedrock.title", { defaultValue: "Minecraft: Bedrock Edition" })}
							</h3>
							<Badge
								variant={isInstalled ? "default" : "secondary"}
								className={cn(
									"text-3xs uppercase tracking-wider",
									isInstalled
										? "border-primary/20 bg-primary/15 text-primary"
										: "bg-muted text-muted-foreground",
								)}
							>
								{isInstalled
									? t("bedrock.installed", { defaultValue: "Installed" })
									: t("bedrock.notInstalled", { defaultValue: "Not Installed" })}
							</Badge>
						</div>
						<p className="mt-0.5 truncate text-2xs text-muted-foreground">
							{isInstalled ? (
								<>
									{edition === "preview"
										? t("bedrock.editionPreview", { defaultValue: "Preview Edition" })
										: t("bedrock.editionRetail", { defaultValue: "Retail Edition" })}
									{version && ` · v${version}`}
								</>
							) : (
								t("bedrock.subtitle", {
									defaultValue: "Cross-platform Windows & console edition",
								})
							)}
						</p>
					</div>
				</div>

				<div className="flex items-center gap-1">
					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="ghost"
									size="icon-xs"
									onClick={handleRefresh}
									disabled={isLoading || isRefreshing}
									className="text-muted-foreground hover:text-foreground"
								>
									<RefreshCw
										className={cn("size-3.5", (isLoading || isRefreshing) && "animate-spin")}
									/>
								</Button>
							}
						/>
						<TooltipContent>{t("common.refresh")}</TooltipContent>
					</Tooltip>

					{isInstalled && (
						<DropdownMenu>
							<DropdownMenuTrigger
								render={
									<Button
										variant="ghost"
										size="icon-xs"
										className="text-muted-foreground hover:text-foreground"
									>
										<MoreVertical className="size-3.5" />
									</Button>
								}
							/>
							<DropdownMenuContent align="end" className="w-52">
								<DropdownMenuGroup>
									<DropdownMenuLabel className="text-2xs text-muted-foreground uppercase">
										{t("instances.openFolder", { defaultValue: "Directories" })}
									</DropdownMenuLabel>
									<DropdownMenuItem onClick={() => openFolder("worlds")}>
										<FolderOpen className="mr-2 size-4 text-primary" />
										{t("bedrock.openWorlds", { defaultValue: "Worlds Folder" })}
									</DropdownMenuItem>
									<DropdownMenuItem onClick={() => openFolder("resource_packs")}>
										<FolderOpen className="mr-2 size-4 text-muted-foreground" />
										{t("bedrock.openResourcePacks", { defaultValue: "Resource Packs" })}
									</DropdownMenuItem>
									<DropdownMenuItem onClick={() => openFolder("behavior_packs")}>
										<FolderOpen className="mr-2 size-4 text-muted-foreground" />
										{t("bedrock.openBehaviorPacks", { defaultValue: "Behavior Packs" })}
									</DropdownMenuItem>
									<DropdownMenuItem onClick={() => openFolder("screenshots")}>
										<FolderOpen className="mr-2 size-4 text-muted-foreground" />
										{t("bedrock.openScreenshots", { defaultValue: "Screenshots Folder" })}
									</DropdownMenuItem>
								</DropdownMenuGroup>
								<DropdownMenuSeparator />
								<DropdownMenuItem onClick={install}>
									<ExternalLink className="mr-2 size-4 text-muted-foreground" />
									{t("bedrock.store", { defaultValue: "Microsoft Store Page" })}
								</DropdownMenuItem>
							</DropdownMenuContent>
						</DropdownMenu>
					)}
				</div>
			</div>

			{/* Center description when not installed */}
			{!isInstalled && (
				<p className="my-3 line-clamp-2 text-muted-foreground text-xs leading-relaxed">
					{t("bedrock.description", {
						defaultValue:
							"Minecraft Bedrock Edition is available through the Microsoft Store. Install it directly from the Store or launch it from Ingot.",
					})}
				</p>
			)}

			{/* Bottom Action Footer */}
			<div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-border/40 border-t pt-3">
				{isInstalled ? (
					<>
						<div className="flex items-center gap-1.5 text-2xs text-muted-foreground">
							<Sparkles className="size-3.5 text-primary" />
							<span>{t("bedrock.readyToPlay", { defaultValue: "Ready to launch" })}</span>
						</div>

						<div className="flex items-center gap-2">
							<Button
								onClick={launch}
								disabled={isLaunching}
								className="h-9 gap-2 rounded-xl px-4 font-semibold text-xs shadow-sm"
							>
								{isLaunching ? (
									<>
										<Spinner className="size-3.5" />
										{t("bedrock.launching", { defaultValue: "Starting..." })}
									</>
								) : (
									<>
										<Play className="size-3.5 fill-current" />
										{t("bedrock.play", { defaultValue: "Play Bedrock" })}
									</>
								)}
							</Button>
						</div>
					</>
				) : (
					<>
						<div className="flex items-center gap-1.5 text-2xs text-muted-foreground">
							<Gamepad2 className="size-3.5 text-muted-foreground" />
							<span>{t("bedrock.msStoreEdition", { defaultValue: "Windows Store Edition" })}</span>
						</div>

						<Button
							onClick={install}
							variant="default"
							className="h-9 gap-2 rounded-xl px-4 font-semibold text-xs shadow-sm"
						>
							<Download className="size-3.5" />
							{t("bedrock.installStore", { defaultValue: "Install via Microsoft Store" })}
						</Button>
					</>
				)}
			</div>
		</div>
	)
}

export default BedrockCard
