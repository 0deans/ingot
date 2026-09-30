import { History, Layers, Palette, RefreshCw, Server, Sliders, UploadCloud } from "lucide-react"
import { memo, useCallback, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { SharedSyncStatus, SyncSettings as SyncSettingsType } from "@/bindings"
import { SectionCardHeader } from "@/components/common/section-card"
import SyncMasterDialog, {
	type InitialSyncCategoryTarget,
	type SyncSourceChoice,
} from "@/components/settings/sync-master-dialog"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Switch } from "@/components/ui/switch"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { settingsService, useSyncSettings } from "@/services/settings-service"

type SyncItemKey =
	| "syncOptions"
	| "syncServers"
	| "syncResourcePacks"
	| "syncCommandHistory"
	| "syncCreativeHotbars"

interface SyncItemConfig {
	key: SyncItemKey
	categoryName: string
	file: string
	icon: typeof Sliders
	color: string
}

const SYNC_ITEMS: SyncItemConfig[] = [
	{
		key: "syncOptions",
		categoryName: "options",
		file: "options.txt",
		icon: Sliders,
		color: "text-amber-400",
	},
	{
		key: "syncServers",
		categoryName: "servers",
		file: "servers.dat",
		icon: Server,
		color: "text-emerald-400",
	},
	{
		key: "syncResourcePacks",
		categoryName: "resourcepacks",
		file: "resourcepacks/",
		icon: Layers,
		color: "text-sky-400",
	},
	{
		key: "syncCommandHistory",
		categoryName: "command_history",
		file: "command_history.txt",
		icon: History,
		color: "text-purple-400",
	},
	{
		key: "syncCreativeHotbars",
		categoryName: "hotbar",
		file: "hotbar.nbt",
		icon: Palette,
		color: "text-pink-400",
	},
]

export const SyncSettings = () => {
	const { t } = useTranslation()
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
		const itemTitle = t(`settings.sync.items.${item.key}.title`)
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
				title: itemTitle,
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
		<Card>
			<SectionCardHeader
				icon={RefreshCw}
				title={t("settings.sync.title")}
				description={t("settings.sync.description")}
			/>
			<CardContent className="flex flex-col gap-4">
				{/* Synchronization Toggles */}
				<div className="flex flex-col divide-y divide-border/30 overflow-hidden rounded-lg border border-border/30 bg-background/60">
					{SYNC_ITEMS.map((item) => {
						const Icon = item.icon
						const isChecked = Boolean(syncSettings[item.key])
						const itemTitle = t(`settings.sync.items.${item.key}.title`)
						const itemDesc = t(`settings.sync.items.${item.key}.description`)
						return (
							<div
								key={item.key}
								className="flex items-center justify-between p-3.5 transition-colors hover:bg-card/30"
							>
								<div className="flex items-center gap-3">
									<div
										className={`flex size-8 items-center justify-center rounded-md bg-card ${item.color}`}
									>
										<Icon className="size-4" />
									</div>
									<div>
										<div className="flex items-center gap-2">
											<span className="font-medium text-foreground text-xs sm:text-sm">
												{itemTitle}
											</span>
											<code className="rounded bg-card px-1.5 py-0.5 font-mono text-3xs text-muted-foreground">
												{item.file}
											</code>
										</div>
										<div className="text-2xs text-muted-foreground">{itemDesc}</div>
									</div>
								</div>

								<div className="flex items-center gap-2">
									{isChecked && (
										<Tooltip>
											<TooltipTrigger
												render={
													<Button
														type="button"
														variant="ghost"
														size="icon"
														onClick={() => {
															setActiveCategoryTarget({
																key: item.key,
																categoryName: item.categoryName,
																title: itemTitle,
																file: item.file,
																hasSharedData: hasCategorySharedData(item.key),
															})
															setIsMasterOpen(true)
														}}
														className="size-7 text-muted-foreground"
														aria-label={t("settings.sync.reinitialize", { title: itemTitle })}
													/>
												}
											>
												<UploadCloud className="size-3.5" />
											</TooltipTrigger>
											<TooltipContent>
												{t("settings.sync.reinitialize", { title: itemTitle })}
											</TooltipContent>
										</Tooltip>
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
			</CardContent>
		</Card>
	)
}

SyncSettings.displayName = "SyncSettings"
export default memo(SyncSettings)
