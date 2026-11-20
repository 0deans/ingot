import {
	Cloud,
	History,
	Layers,
	Palette,
	RefreshCw,
	Server,
	SlidersHorizontal,
	type LucideIcon,
} from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore, type SyncPrefs } from "@/shared/stores/useLauncherPrefsStore"
import { Switch } from "@/shared/ui/switch"

interface SyncItemDef {
	key: keyof SyncPrefs
	icon: LucideIcon
	iconClassName: string
	fileBadge: string
	titleKey:
		| "settings.general.sync.items.options.title"
		| "settings.general.sync.items.servers.title"
		| "settings.general.sync.items.resourcePacks.title"
		| "settings.general.sync.items.commandHistory.title"
		| "settings.general.sync.items.creativeHotbars.title"
	descriptionKey:
		| "settings.general.sync.items.options.description"
		| "settings.general.sync.items.servers.description"
		| "settings.general.sync.items.resourcePacks.description"
		| "settings.general.sync.items.commandHistory.description"
		| "settings.general.sync.items.creativeHotbars.description"
}

const SYNC_ITEMS: readonly SyncItemDef[] = [
	{
		key: "syncOptions",
		icon: SlidersHorizontal,
		iconClassName: "bg-amber-500/12 text-amber-400 border-amber-500/20",
		fileBadge: "options.txt",
		titleKey: "settings.general.sync.items.options.title",
		descriptionKey: "settings.general.sync.items.options.description",
	},
	{
		key: "syncServers",
		icon: Server,
		iconClassName: "bg-emerald-500/12 text-emerald-400 border-emerald-500/20",
		fileBadge: "servers.dat",
		titleKey: "settings.general.sync.items.servers.title",
		descriptionKey: "settings.general.sync.items.servers.description",
	},
	{
		key: "syncResourcePacks",
		icon: Layers,
		iconClassName: "bg-sky-500/12 text-sky-400 border-sky-500/20",
		fileBadge: "resourcepacks/",
		titleKey: "settings.general.sync.items.resourcePacks.title",
		descriptionKey: "settings.general.sync.items.resourcePacks.description",
	},
	{
		key: "syncCommandHistory",
		icon: History,
		iconClassName: "bg-violet-500/12 text-violet-400 border-violet-500/20",
		fileBadge: "command_history.txt",
		titleKey: "settings.general.sync.items.commandHistory.title",
		descriptionKey: "settings.general.sync.items.commandHistory.description",
	},
	{
		key: "syncCreativeHotbars",
		icon: Palette,
		iconClassName: "bg-pink-500/12 text-pink-400 border-pink-500/20",
		fileBadge: "hotbar.nbt",
		titleKey: "settings.general.sync.items.creativeHotbars.title",
		descriptionKey: "settings.general.sync.items.creativeHotbars.description",
	},
]

const GameDataSyncCard = () => {
	const { t } = useTranslation()
	const { syncSettings, setSyncSetting } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				syncSettings: s.syncSettings,
				setSyncSetting: s.setSyncSetting,
			}
		}),
	)

	return (
		<section className="@container flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-3.5 backdrop-blur-md sm:p-4">
			<div className="flex items-start gap-2.5">
				<div className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg border border-border bg-foreground/5 text-primary">
					<RefreshCw className="size-4" />
				</div>
				<div className="min-w-0">
					<h2 className="text-[13.5px] font-semibold text-foreground">
						{t("settings.general.sync.title")}
					</h2>
					<p className="text-[11.5px] leading-relaxed text-muted-foreground">
						{t("settings.general.sync.description")}
					</p>
				</div>
			</div>

			<div className="flex flex-col divide-y divide-border/60 rounded-xl border border-border bg-foreground/3">
				{SYNC_ITEMS.map((item) => {
					const Icon = item.icon
					const isEnabled = syncSettings[item.key]

					return (
						<div
							key={item.key}
							className="flex items-start justify-between gap-2.5 px-3 py-2.5 @sm:items-center @sm:gap-3 @sm:px-3.5"
						>
							<div className="flex min-w-0 flex-1 items-start gap-2.5 @sm:items-center @sm:gap-3">
								<div
									className={cn(
										"mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border @sm:mt-0",
										item.iconClassName,
									)}
								>
									<Icon className="size-4" />
								</div>

								<div className="min-w-0 flex-1">
									<div className="flex flex-wrap items-center gap-1.5 @sm:gap-2">
										<p className="text-[12.5px] font-medium text-foreground">{t(item.titleKey)}</p>
										<span className="rounded-md border border-border bg-foreground/5 px-1.5 py-0.5 font-mono text-[10.5px] text-muted-foreground">
											{item.fileBadge}
										</span>
									</div>
									<p className="text-[11px] leading-relaxed text-muted-foreground">
										{t(item.descriptionKey)}
									</p>
								</div>
							</div>

							<div className="flex shrink-0 items-center gap-2.5 pt-0.5 @sm:pt-0">
								<Cloud
									aria-hidden="true"
									className={cn(
										"hidden size-4 transition-colors @sm:block",
										isEnabled ? "text-primary/80" : "text-muted-foreground/45",
									)}
								/>
								<Switch
									checked={isEnabled}
									onCheckedChange={(checked) => {
										setSyncSetting(item.key, checked)
									}}
									aria-label={t(item.titleKey)}
								/>
							</div>
						</div>
					)
				})}
			</div>
		</section>
	)
}

export default GameDataSyncCard
