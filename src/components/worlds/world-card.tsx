import { save as saveDialog } from "@tauri-apps/plugin-dialog"
import {
	Archive,
	Check,
	Clock,
	Copy,
	Download,
	Flame,
	Folder,
	FolderOpen,
	Globe,
	HardDrive,
	MoreVertical,
	RefreshCw,
	Sparkles,
} from "lucide-react"
import { memo, useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { InstanceConfig, WorldDimension, WorldSummary } from "@/bindings"
import { getColorTagDefinition } from "@/components/instances/instance-icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { formatBytes, formatRelative } from "@/lib/format"
import { worldService } from "@/services/world-service"

interface WorldCardProps {
	world: WorldSummary
	instance?: InstanceConfig
	onResetDimension: (world: WorldSummary, dimension: WorldDimension) => void
	onViewBackups: (world: WorldSummary) => void
	onRefetch: () => void
}

const WorldCard = ({
	world,
	instance,
	onResetDimension,
	onViewBackups,
	onRefetch,
}: WorldCardProps) => {
	const { t } = useTranslation()

	const [hasCopiedSeed, setHasCopiedSeed] = useState(false)
	const [isBackingUp, setIsBackingUp] = useState(false)
	const [isExporting, setIsExporting] = useState(false)

	const colorDef = useMemo(() => {
		return getColorTagDefinition(instance?.colorTag)
	}, [instance?.colorTag])

	const gameModeBadge = useMemo(() => {
		switch (world.gameMode.toLowerCase()) {
			case "hardcore":
				return (
					<Badge variant="destructive" className="gap-1 text-xs font-semibold">
						<Flame className="size-3" />
						{t("worlds.hardcore")}
					</Badge>
				)
			case "creative":
				return (
					<Badge
						variant="default"
						className="gap-1 border-sky-500/30 bg-sky-500/15 text-xs text-sky-400"
					>
						<Sparkles className="size-3" />
						{t("worlds.creative")}
					</Badge>
				)
			case "adventure":
				return (
					<Badge variant="outline" className="border-amber-500/30 text-xs text-amber-400">
						{t("worlds.adventure")}
					</Badge>
				)
			case "spectator":
				return (
					<Badge variant="outline" className="border-purple-500/30 text-xs text-purple-400">
						{t("worlds.spectator")}
					</Badge>
				)
			default:
				return (
					<Badge variant="outline" className="border-emerald-500/30 text-xs text-emerald-400">
						{t("worlds.survival")}
					</Badge>
				)
		}
	}, [world.gameMode, t])

	const handleCopySeed = useCallback(async () => {
		if (!world.seed) {
			toast.info(t("worlds.seedUnknown"))
			return
		}
		try {
			await navigator.clipboard.writeText(world.seed)
			setHasCopiedSeed(true)
			toast.success(t("worlds.seedCopied"))
			setTimeout(() => {
				setHasCopiedSeed(false)
			}, 2000)
		} catch (error) {
			console.error("Failed to copy seed:", error)
		}
	}, [world.seed, t])

	const handleOpenFolder = useCallback(async () => {
		try {
			await worldService.openWorldFolder(world.instanceId, world.folderName)
		} catch (error) {
			console.error("Failed to open world folder:", error)
			const message = error instanceof Error ? error.message : String(error)
			toast.error(message)
		}
	}, [world.instanceId, world.folderName])

	const handleCreateBackup = useCallback(async () => {
		try {
			setIsBackingUp(true)
			await worldService.backupWorld(world.instanceId, world.folderName)
			toast.success(t("worlds.backupSuccess"))
			onRefetch()
		} catch (error) {
			console.error("Failed to backup world:", error)
			const message = error instanceof Error ? error.message : String(error)
			toast.error(message)
		} finally {
			setIsBackingUp(false)
		}
	}, [world.instanceId, world.folderName, onRefetch, t])

	const handleExport = useCallback(async () => {
		try {
			const defaultName = `${world.displayName.replace(/[^\w-]+/g, "_")}.zip`
			const dest = await saveDialog({
				defaultPath: defaultName,
				filters: [{ name: "ZIP Archive", extensions: ["zip"] }],
			})
			if (!dest) {
				return
			}
			setIsExporting(true)
			const path = await worldService.exportWorld(world.instanceId, world.folderName, dest)
			toast.success(t("worlds.exportSuccess", { path }))
		} catch (error) {
			console.error("Failed to export world:", error)
			const message = error instanceof Error ? error.message : String(error)
			toast.error(message)
		} finally {
			setIsExporting(false)
		}
	}, [world.displayName, world.instanceId, world.folderName, t])

	return (
		<div className="group relative flex flex-col justify-between overflow-hidden rounded-xl border border-border/50 bg-card/60 p-4 backdrop-blur-sm transition-all hover:border-border hover:bg-card/90 hover:shadow-md">
			<div className="space-y-3">
				{/* Top bar with icon, title, and dropdown menu */}
				<div className="flex items-start justify-between gap-3">
					<div className="flex min-w-0 items-center gap-3">
						{world.icon ? (
							<img
								src={world.icon}
								alt={world.displayName}
								className="size-12 rounded-lg object-cover ring-1 ring-border/50"
							/>
						) : (
							<div className="flex size-12 items-center justify-center rounded-lg border border-border/60 bg-muted/40 text-muted-foreground">
								<Globe className="size-6 stroke-[1.5]" />
							</div>
						)}

						<div className="min-w-0 flex-1">
							<h3 className="truncate text-base leading-snug font-semibold text-foreground">
								{world.displayName}
							</h3>
							<p className="truncate font-mono text-xs text-muted-foreground">{world.folderName}</p>
						</div>
					</div>

					<DropdownMenu>
						<DropdownMenuTrigger
							render={
								<Button
									size="icon-sm"
									variant="ghost"
									className="text-muted-foreground hover:text-foreground"
								>
									<MoreVertical className="size-4" />
								</Button>
							}
						/>
						<DropdownMenuContent align="end" className="w-52">
							<DropdownMenuItem onClick={handleOpenFolder}>
								<FolderOpen className="mr-2 size-4 text-muted-foreground" />
								{t("worlds.openFolder")}
							</DropdownMenuItem>
							<DropdownMenuItem onClick={handleCreateBackup} disabled={isBackingUp}>
								<Archive className="mr-2 size-4 text-muted-foreground" />
								{t("worlds.createBackup")}
							</DropdownMenuItem>
							<DropdownMenuItem onClick={handleExport} disabled={isExporting}>
								<Download className="mr-2 size-4 text-muted-foreground" />
								{t("worlds.exportZip")}
							</DropdownMenuItem>
							<DropdownMenuItem
								onClick={() => {
									return onViewBackups(world)
								}}
							>
								<Archive className="mr-2 size-4 text-muted-foreground" />
								{t("worlds.viewBackups")}
							</DropdownMenuItem>

							<DropdownMenuSeparator />

							<DropdownMenuItem
								onClick={() => {
									return onResetDimension(world, "nether")
								}}
								disabled={!world.hasNether}
								className="text-amber-500 focus:text-amber-500"
							>
								<RefreshCw className="mr-2 size-4" />
								{t("worlds.resetNether")}
							</DropdownMenuItem>

							<DropdownMenuItem
								onClick={() => {
									return onResetDimension(world, "the-end")
								}}
								disabled={!world.hasTheEnd}
								className="text-purple-400 focus:text-purple-400"
							>
								<RefreshCw className="mr-2 size-4" />
								{t("worlds.resetTheEnd")}
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				</div>

				{/* Badges: Instance, Group, Version, Game Mode */}
				<div className="flex flex-wrap items-center gap-1.5">
					{gameModeBadge}

					<Badge variant="secondary" className="gap-1.5 text-xs">
						{colorDef && (
							<span
								className="size-2 shrink-0 rounded-full"
								style={{ backgroundColor: colorDef.hex }}
								title={colorDef.defaultLabel}
							/>
						)}
						{world.instanceName}
					</Badge>

					{instance?.group && (
						<Badge variant="outline" className="gap-1 text-xs text-muted-foreground">
							<Folder className="size-3 shrink-0" />
							{instance.group}
						</Badge>
					)}

					{world.versionName && (
						<Badge variant="outline" className="text-xs text-muted-foreground">
							{world.versionName}
						</Badge>
					)}
				</div>

				{/* Metadata Stats: Last Played & Size */}
				<div className="grid grid-cols-2 gap-2 border-t border-border/40 pt-2.5 text-xs text-muted-foreground">
					<div className="flex items-center gap-1.5 truncate">
						<Clock className="size-3.5 shrink-0" />
						<span className="truncate">
							{world.lastPlayed ? formatRelative(world.lastPlayed * 1000) : t("worlds.lastPlayed")}
						</span>
					</div>

					<div className="flex items-center gap-1.5 truncate">
						<HardDrive className="size-3.5 shrink-0" />
						<span>{formatBytes(world.sizeBytes)}</span>
					</div>
				</div>

				{/* Seed with 1-click Copy */}
				<div className="flex items-center justify-between rounded-lg border border-border/40 bg-muted/30 px-2.5 py-1.5">
					<div className="min-w-0 flex-1">
						<span className="block text-[10px] font-bold tracking-wider text-muted-foreground/70 uppercase">
							{t("worlds.seed")}
						</span>
						<span className="block truncate font-mono text-xs text-foreground select-all">
							{world.seed || t("worlds.seedUnknown")}
						</span>
					</div>

					{world.seed && (
						<Tooltip>
							<TooltipTrigger
								render={
									<Button
										size="icon-xs"
										variant="ghost"
										onClick={handleCopySeed}
										className="shrink-0 text-muted-foreground hover:text-foreground"
										aria-label={t("worlds.copySeed")}
									>
										{hasCopiedSeed ? (
											<Check className="size-3.5 text-emerald-400" />
										) : (
											<Copy className="size-3.5" />
										)}
									</Button>
								}
							/>
							<TooltipContent side="bottom">{t("worlds.copySeed")}</TooltipContent>
						</Tooltip>
					)}
				</div>
			</div>

			{/* Dimension indicators at bottom */}
			<div className="mt-3 flex items-center gap-2 border-t border-border/40 pt-2 text-[11px] text-muted-foreground">
				<span className="text-muted-foreground/70">{t("worlds.dimensions")}:</span>
				<span
					className={
						world.hasNether ? "font-medium text-amber-500" : "text-muted-foreground/40 line-through"
					}
				>
					{t("worlds.hasNether")}
				</span>
				<span>•</span>
				<span
					className={
						world.hasTheEnd
							? "font-medium text-purple-400"
							: "text-muted-foreground/40 line-through"
					}
				>
					{t("worlds.hasTheEnd")}
				</span>
			</div>
		</div>
	)
}

WorldCard.displayName = "WorldCard"

export default memo(WorldCard)
