import { ArrowLeft, Box, Compass, FolderOpen, Layers, Sparkles } from "lucide-react"
import { memo, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { InstanceConfig } from "@/bindings"
import { supportsMods } from "@/components/content/install-dialog"
import InstanceIcon from "@/components/instances/instance-icon"
import LoaderIcon from "@/components/instances/loader-icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { instanceContentService, useInstanceContent } from "@/services/instance-content-service"
import ModsPanel from "./mods-panel"

export interface InstanceContentViewProps {
	instance: InstanceConfig
	activeTab: "mods" | "resourcepacks" | "shaders" | "datapacks"
	isInstanceRunning: boolean
	onTabChange: (tab: "mods" | "resourcepacks" | "shaders" | "datapacks") => void
	onBack: () => void
	onOpenDiscoverMods?: () => void
}

const InstanceContentView = ({
	instance,
	activeTab,
	isInstanceRunning,
	onTabChange,
	onBack,
	onOpenDiscoverMods,
}: InstanceContentViewProps) => {
	// 1. Context & Global
	const { t } = useTranslation()

	// 2. Refs (none)

	// 3. State / Queries
	const isLoaderSupported = supportsMods(instance.loader)
	const { data: modsData, isLoading: isModsLoading } = useInstanceContent(instance.id, "mod", {
		enabled: isLoaderSupported,
	})

	// 4. Memoization / Callbacks
	const handleValueChange = useCallback(
		(val: string) => {
			if (val === "mods" || val === "resourcepacks" || val === "shaders" || val === "datapacks") {
				onTabChange(val)
			}
		},
		[onTabChange],
	)

	const handleOpenFolder = useCallback(async () => {
		try {
			const category =
				activeTab === "mods"
					? "mod"
					: activeTab === "resourcepacks"
						? "resourcePack"
						: activeTab === "shaders"
							? "shaderPack"
							: "dataPack"
			await instanceContentService.openContentFolder(instance.id, category)
		} catch (error) {
			toast.error(error instanceof Error ? error.message : String(error))
		}
	}, [instance.id, activeTab])

	return (
		<div className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-background">
			{/* Top Header Bar - Matches Instance Logs Header */}
			<div className="flex shrink-0 items-center justify-between gap-2.5 border-b border-border/50 bg-card/40 px-3 py-2.5 sm:gap-3 sm:px-6 sm:py-3">
				<div className="flex min-w-0 items-center gap-2 sm:gap-3">
					<Button
						variant="ghost"
						size="sm"
						onClick={onBack}
						className="gap-1.5 px-2 text-xs font-medium text-muted-foreground hover:text-foreground sm:px-2.5"
						aria-label={t("instanceContent.back")}
					>
						<ArrowLeft className="size-4" />
						<span className="hidden sm:inline">{t("instanceContent.back")}</span>
					</Button>

					<div className="h-5 w-px bg-border/60" />

					<div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border bg-card">
						<InstanceIcon
							icon={instance.icon}
							loader={instance.loader}
							className="size-5 object-contain"
						/>
					</div>

					<div className="flex min-w-0 flex-col">
						<div className="flex min-w-0 items-center gap-2">
							<h1 className="truncate text-sm font-semibold text-foreground">{instance.name}</h1>
							{isInstanceRunning ? (
								<Badge
									variant="outline"
									className="shrink-0 border-primary/40 text-2xs text-primary"
								>
									<span className="mr-1 size-1.5 animate-pulse rounded-full bg-primary" />
									{t("instanceContent.running")}
								</Badge>
							) : (
								<Badge
									variant="outline"
									className="shrink-0 text-2xs text-muted-foreground uppercase"
								>
									{instance.loader}
								</Badge>
							)}
						</div>
						<div className="flex items-center gap-1.5 text-2xs text-muted-foreground">
							<LoaderIcon loader={instance.loader} className="size-3" />
							<span>{instance.gameVersion}</span>
							{instance.loaderVersion && (
								<>
									<span>•</span>
									<span>{instance.loaderVersion}</span>
								</>
							)}
						</div>
					</div>
				</div>

				<div className="flex shrink-0 items-center gap-2">
					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="outline"
									size="sm"
									onClick={handleOpenFolder}
									className="gap-1.5 text-xs max-sm:size-8 max-sm:p-0"
									aria-label={t("instanceContent.openFolder")}
								>
									<FolderOpen className="size-3.5" />
									<span className="hidden sm:inline">{t("instanceContent.openFolder")}</span>
								</Button>
							}
						/>
						<TooltipContent side="bottom" align="end">
							{t("instanceContent.openFolder")}
						</TooltipContent>
					</Tooltip>
				</div>
			</div>

			{/* Read-Only Running Alert Banner */}
			{isInstanceRunning && (
				<div className="mx-4 mt-3 flex items-center gap-2.5 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-xs text-amber-600 sm:mx-6 dark:text-amber-400">
					<span className="font-semibold">{t("instanceContent.readOnlyBannerTitle")}</span>
					<span className="ml-1 text-muted-foreground">
						{t("instanceContent.readOnlyBannerDescription")}
					</span>
				</div>
			)}

			{/* Tabs Section */}
			<Tabs
				value={activeTab}
				onValueChange={handleValueChange}
				className="flex min-h-0 flex-1 flex-col"
			>
				{/* Navigation Tabs Bar */}
				<div className="flex shrink-0 items-center justify-between border-b border-border/40 bg-background/60 px-4 py-1.5 sm:px-6">
					<TabsList className="bg-muted/60">
						<TabsTrigger value="mods" className="gap-1.5 px-3 text-xs">
							<Box className="size-3.5" />
							<span>{t("instanceContent.tabs.mods")}</span>
							{isLoaderSupported && modsData?.items && (
								<span className="py-0.2 ml-1 rounded-full bg-muted-foreground/15 px-1.5 text-[10px]">
									{modsData.items.length}
								</span>
							)}
						</TabsTrigger>
						<TabsTrigger value="resourcepacks" className="gap-1.5 px-3 text-xs">
							<Layers className="size-3.5" />
							<span>{t("instanceContent.tabs.resourcePacks")}</span>
						</TabsTrigger>
						<TabsTrigger value="shaders" className="gap-1.5 px-3 text-xs">
							<Sparkles className="size-3.5" />
							<span>{t("instanceContent.tabs.shaders")}</span>
						</TabsTrigger>
						<TabsTrigger value="datapacks" className="gap-1.5 px-3 text-xs">
							<Compass className="size-3.5" />
							<span>{t("instanceContent.tabs.datapacks")}</span>
						</TabsTrigger>
					</TabsList>
				</div>

				<TabsContent
					value="mods"
					className="flex min-h-0 flex-1 flex-col overflow-hidden outline-none"
				>
					<ModsPanel
						instanceId={instance.id}
						loader={instance.loader}
						data={modsData}
						isLoading={isModsLoading}
						isInstanceRunning={isInstanceRunning}
						onOpenDiscoverMods={onOpenDiscoverMods}
						onSwitchTab={onTabChange}
					/>
				</TabsContent>

				<TabsContent value="resourcepacks" className="flex min-h-0 flex-1 flex-col p-4 sm:p-6">
					<div className="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border/60 p-12 text-xs text-muted-foreground">
						{t("instanceContent.resourcePacksComingInP3")}
					</div>
				</TabsContent>

				<TabsContent value="shaders" className="flex min-h-0 flex-1 flex-col p-4 sm:p-6">
					<div className="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border/60 p-12 text-xs text-muted-foreground">
						{t("instanceContent.shadersComingInP3")}
					</div>
				</TabsContent>

				<TabsContent value="datapacks" className="flex min-h-0 flex-1 flex-col p-4 sm:p-6">
					<div className="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border/60 p-12 text-xs text-muted-foreground">
						{t("instanceContent.datapacksComingInP4")}
					</div>
				</TabsContent>
			</Tabs>
		</div>
	)
}

InstanceContentView.displayName = "InstanceContentView"

export default memo(InstanceContentView)
