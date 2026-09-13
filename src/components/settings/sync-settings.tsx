import {
	History,
	Info,
	Layers,
	Palette,
	RefreshCw,
	Server,
	Sliders,
	UploadCloud,
} from "lucide-react"
import { memo, useCallback, useEffect, useState } from "react"
import type { SharedSyncStatus, SyncSettings as SyncSettingsType } from "@/bindings"
import SyncMasterDialog, {
	type InitialSyncCategoryTarget,
	type SyncSourceChoice,
} from "@/components/settings/sync-master-dialog"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import { settingsService, useSyncSettings } from "@/services/settings-service"

interface SyncItemConfig {
	key: keyof SyncSettingsType
	categoryName: string
	title: string
	description: string
	file: string
	icon: typeof Sliders
	color: string
}

const SYNC_ITEMS: SyncItemConfig[] = [
	{
		key: "syncOptions",
		categoryName: "options",
		title: "Sync Game Options",
		description: "Video settings, audio volume, FOV, keybindings, and accessibility",
		file: "options.txt",
		icon: Sliders,
		color: "text-amber-400",
	},
	{
		key: "syncServers",
		categoryName: "servers",
		title: "Sync Multiplayer Servers",
		description: "Keep your saved server list unified across all instances",
		file: "servers.dat",
		icon: Server,
		color: "text-emerald-400",
	},
	{
		key: "syncResourcePacks",
		categoryName: "resourcepacks",
		title: "Sync Resource Packs",
		description: "Share all downloaded texture and resource packs across instances",
		file: "resourcepacks/",
		icon: Layers,
		color: "text-sky-400",
	},
	{
		key: "syncCommandHistory",
		categoryName: "command_history",
		title: "Sync Command History",
		description: "Preserve chat and console command history across launches",
		file: "command_history.txt",
		icon: History,
		color: "text-purple-400",
	},
	{
		key: "syncCreativeHotbars",
		categoryName: "hotbar",
		title: "Sync Saved Creative Hotbars",
		description: "Share saved hotbar layouts in creative mode across worlds",
		file: "hotbar.nbt",
		icon: Palette,
		color: "text-pink-400",
	},
]

export const SyncSettings = () => {
	const { syncSettings, setSyncSettings } = useSyncSettings()
	const [sharedStatus, setSharedStatus] = useState<SharedSyncStatus | null>(null)
	const [activeCategoryTarget, setActiveCategoryTarget] =
		useState<InitialSyncCategoryTarget | null>(null)
	const [isMasterOpen, setIsMasterOpen] = useState(false)

	const refreshStatus = useCallback(() => {
		settingsService
			.getSharedSyncStatus()
			.then((status) => setSharedStatus(status))
			.catch((err) => console.error("Failed to load shared sync status:", err))
	}, [])

	useEffect(() => {
		refreshStatus()
	}, [refreshStatus])

	const hasCategorySharedData = (key: keyof SyncSettingsType): boolean => {
		switch (key) {
			case "syncOptions":
				return Boolean(sharedStatus?.hasOptions)
			case "syncServers":
				return Boolean(sharedStatus?.hasServers)
			case "syncResourcePacks":
				return (sharedStatus?.resourcePacksCount ?? 0) > 0
			case "syncCommandHistory":
				return Boolean(sharedStatus?.hasCommandHistory)
			case "syncCreativeHotbars":
				return Boolean(sharedStatus?.hasCreativeHotbars)
			default:
				return false
		}
	}

	const handleToggle = async (item: SyncItemConfig, checked: boolean) => {
		if (!checked) {
			try {
				await setSyncSettings({
					...syncSettings,
					[item.key]: false,
				})
			} catch (error) {
				console.error(`Failed to disable ${item.key}:`, error)
			}
			return
		}

		// If turning ON, check if user has already initialized this category before
		const initialized = (syncSettings.initializedCategories ?? []).includes(item.key)

		if (!initialized) {
			setActiveCategoryTarget({
				key: item.key,
				categoryName: item.categoryName,
				title: item.title,
				file: item.file,
				hasSharedData: hasCategorySharedData(item.key),
			})
			setIsMasterOpen(true)
			return
		}

		// Already initialized: just turn on
		try {
			await setSyncSettings({
				...syncSettings,
				[item.key]: true,
			})
		} catch (error) {
			console.error(`Failed to enable ${item.key}:`, error)
		}
	}

	const handleSelectSource = async (choice: SyncSourceChoice) => {
		if (!activeCategoryTarget) return

		const key = activeCategoryTarget.key as keyof SyncSettingsType
		const categoryName = activeCategoryTarget.categoryName

		if (choice.type === "instance") {
			await settingsService.exportInstanceCategory(choice.instanceId, categoryName)
			refreshStatus()
		}

		const currentInitialized = syncSettings.initializedCategories ?? []
		const nextInitialized = currentInitialized.includes(key)
			? currentInitialized
			: [...currentInitialized, key]

		await setSyncSettings({
			...syncSettings,
			[key]: true,
			initializedCategories: nextInitialized,
		})

		setActiveCategoryTarget(null)
	}

	const handleDialogCancel = () => {
		setActiveCategoryTarget(null)
		setIsMasterOpen(false)
	}

	return (
		<div className="flex flex-col gap-4 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
			<div className="flex flex-col gap-1">
				<div className="flex items-center gap-2">
					<RefreshCw className="size-4 text-emerald-400" />
					<h3 className="font-semibold text-foreground text-sm">Game Data Synchronization</h3>
				</div>
				<p className="text-muted-foreground text-xs">
					Keep your Minecraft configurations, servers, and assets aligned across all instances.
				</p>
			</div>

			{/* How it works info callout */}
			<div className="flex flex-col gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3.5 text-emerald-300 text-xs">
				<div className="flex items-start gap-2.5">
					<Info className="mt-0.5 size-4 shrink-0 text-emerald-400" />
					<div className="flex flex-col gap-1 text-[11px] leading-relaxed">
						<span>
							<strong>Automated 2-way sync:</strong> Before game launch, updated shared settings are
							copied to your instance. When Minecraft closes, your latest changes are automatically
							saved back to shared storage.
						</span>
						{sharedStatus && (
							<span className="text-emerald-400/80">
								Shared storage active:{" "}
								{[
									sharedStatus.hasOptions && "options.txt",
									sharedStatus.hasServers && "servers.dat",
									sharedStatus.resourcePacksCount > 0 &&
										`${sharedStatus.resourcePacksCount} resource packs`,
									sharedStatus.hasCommandHistory && "command_history.txt",
									sharedStatus.hasCreativeHotbars && "hotbar.nbt",
								]
									.filter(Boolean)
									.join(", ") || "no shared files yet"}
							</span>
						)}
					</div>
				</div>
			</div>

			{/* Synchronization Toggles */}
			<div className="flex flex-col divide-y divide-border/30 overflow-hidden rounded-lg border border-border/30 bg-zinc-950/60">
				{SYNC_ITEMS.map((item) => {
					const Icon = item.icon
					const isChecked = Boolean(syncSettings[item.key])
					return (
						<div
							key={item.key}
							className="flex items-center justify-between p-3.5 transition-colors hover:bg-zinc-900/30"
						>
							<div className="flex items-center gap-3">
								<div
									className={`flex size-8 items-center justify-center rounded-md bg-zinc-900 ${item.color}`}
								>
									<Icon className="size-4" />
								</div>
								<div>
									<div className="flex items-center gap-2">
										<span className="font-medium text-foreground text-xs sm:text-sm">
											{item.title}
										</span>
										<code className="rounded bg-zinc-900 px-1.5 py-0.5 font-mono text-[10px] text-zinc-400">
											{item.file}
										</code>
									</div>
									<div className="text-[11px] text-muted-foreground">{item.description}</div>
								</div>
							</div>

							<div className="flex items-center gap-2">
								{isChecked && (
									<Button
										type="button"
										variant="ghost"
										size="icon"
										onClick={() => {
											setActiveCategoryTarget({
												key: item.key,
												categoryName: item.categoryName,
												title: item.title,
												file: item.file,
												hasSharedData: hasCategorySharedData(item.key),
											})
											setIsMasterOpen(true)
										}}
										className="size-7 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200"
										title={`Re-initialize ${item.title} source`}
									>
										<UploadCloud className="size-3.5" />
									</Button>
								)}
								<Switch
									checked={isChecked}
									onCheckedChange={(checked) => handleToggle(item, checked)}
								/>
							</div>
						</div>
					)
				})}
			</div>

			<SyncMasterDialog
				open={isMasterOpen}
				onOpenChange={setIsMasterOpen}
				category={activeCategoryTarget}
				onSelectSource={handleSelectSource}
				onCancel={handleDialogCancel}
				onSuccess={refreshStatus}
			/>
		</div>
	)
}

SyncSettings.displayName = "SyncSettings"
export default memo(SyncSettings)
