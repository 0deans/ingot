import { ArrowLeft, Box, Compass, Layers, Sparkles } from "lucide-react"
import { memo, useCallback } from "react"
import { useTranslation } from "react-i18next"
import type { InstanceConfig } from "@/bindings"
import InstanceIcon from "@/components/instances/instance-icon"
import LoaderIcon from "@/components/instances/loader-icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { useInstanceContent } from "@/services/instance-content-service"
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
	const { data: modsData, isLoading: isModsLoading } = useInstanceContent(instance.id, "mod")

	// 4. Memoization / Callbacks
	const handleValueChange = useCallback(
		(val: string) => {
			if (val === "mods" || val === "resourcepacks" || val === "shaders" || val === "datapacks") {
				onTabChange(val)
			}
		},
		[onTabChange],
	)

	return (
		<div className="flex h-full flex-1 flex-col gap-5 p-4 sm:p-6">
			{/* Top Bar Navigation & Instance Header */}
			<div className="flex flex-wrap items-center justify-between gap-4 border-b border-border/50 pb-4">
				<div className="flex items-center gap-3">
					<Button
						variant="ghost"
						size="icon-sm"
						onClick={onBack}
						aria-label={t("instanceContent.back")}
						className="text-muted-foreground hover:text-foreground"
					>
						<ArrowLeft className="size-4" />
					</Button>

					<div className="flex size-10 items-center justify-center rounded-lg border border-border/60 bg-muted/40">
						<InstanceIcon
							icon={instance.icon}
							loader={instance.loader}
							className="size-6 object-contain"
						/>
					</div>

					<div className="flex flex-col">
						<div className="flex items-center gap-2">
							<h1 className="text-base font-semibold text-foreground">{instance.name}</h1>
							{isInstanceRunning && (
								<Badge variant="default" className="text-[10px]">
									{t("instanceContent.running")}
								</Badge>
							)}
						</div>
						<div className="flex items-center gap-1.5 text-xs text-muted-foreground">
							<LoaderIcon loader={instance.loader} className="size-3.5" />
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
			</div>

			{/* Read-Only Running Alert Banner */}
			{isInstanceRunning && (
				<div className="flex items-center gap-2.5 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-xs text-amber-600 dark:text-amber-400">
					<span className="font-semibold">{t("instanceContent.readOnlyBannerTitle")}</span>
					<span>{t("instanceContent.readOnlyBannerDescription")}</span>
				</div>
			)}

			{/* Tabs Section */}
			<Tabs
				value={activeTab}
				onValueChange={handleValueChange}
				className="flex flex-1 flex-col gap-4"
			>
				<TabsList className="w-fit">
					<TabsTrigger value="mods" className="gap-2 text-xs">
						<Box className="size-3.5" />
						<span>{t("instanceContent.tabs.mods")}</span>
						{modsData?.items && (
							<span className="py-0.2 ml-1 rounded-full bg-muted-foreground/15 px-1.5 text-[10px]">
								{modsData.items.length}
							</span>
						)}
					</TabsTrigger>
					<TabsTrigger value="resourcepacks" className="gap-2 text-xs">
						<Layers className="size-3.5" />
						<span>{t("instanceContent.tabs.resourcePacks")}</span>
					</TabsTrigger>
					<TabsTrigger value="shaders" className="gap-2 text-xs">
						<Sparkles className="size-3.5" />
						<span>{t("instanceContent.tabs.shaders")}</span>
					</TabsTrigger>
					<TabsTrigger value="datapacks" className="gap-2 text-xs">
						<Compass className="size-3.5" />
						<span>{t("instanceContent.tabs.datapacks")}</span>
					</TabsTrigger>
				</TabsList>

				<TabsContent value="mods" className="flex flex-1 flex-col">
					<ModsPanel
						instanceId={instance.id}
						data={modsData}
						isLoading={isModsLoading}
						isInstanceRunning={isInstanceRunning}
						onOpenDiscoverMods={onOpenDiscoverMods}
					/>
				</TabsContent>

				<TabsContent value="resourcepacks" className="flex flex-1 flex-col">
					<div className="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border/60 p-12 text-xs text-muted-foreground">
						{t("instanceContent.resourcePacksComingInP3")}
					</div>
				</TabsContent>

				<TabsContent value="shaders" className="flex flex-1 flex-col">
					<div className="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border/60 p-12 text-xs text-muted-foreground">
						{t("instanceContent.shadersComingInP3")}
					</div>
				</TabsContent>

				<TabsContent value="datapacks" className="flex flex-1 flex-col">
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
