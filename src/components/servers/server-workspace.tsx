import i18n from "i18next"
import {
	ArrowLeft,
	Gauge,
	Map as MapIcon,
	Puzzle,
	Server,
	Settings2,
	Terminal,
	Users,
} from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import type { PlayerDetails, ServerConfig } from "@/bindings"
import { ScrollArea } from "@/components/common/scroll-area"
import { Button } from "@/components/ui/button"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { useServerIcon, useServerStatus } from "@/services/server-data"
import { ConsolePanel } from "./panels/console-panel"
import { MapPanel } from "./panels/map-panel"
import { OverviewPanel, StatusPill, type WorkspaceTab } from "./panels/overview-panel"
import { PlayersPanel } from "./panels/players-panel"
import { addonKind, PluginsPanel } from "./panels/plugins-panel"
import { SettingsPanel } from "./panels/settings-panel"

export const WORKSPACE_TABS: { id: WorkspaceTab; label: string; icon: typeof Gauge }[] = [
	{ id: "overview", label: "Overview", icon: Gauge },
	{ id: "console", label: "Console", icon: Terminal },
	{ id: "players", label: "Players", icon: Users },
	{ id: "map", label: "Map", icon: MapIcon },
	{ id: "plugins", label: "Plugins", icon: Puzzle },
	{ id: "settings", label: "Settings", icon: Settings2 },
]

/** Tab label for this server ("Mods" instead of "Plugins" on Fabric), in the user's language */
export function tabLabel(id: WorkspaceTab, _label: string, core: ServerConfig["core"]): string {
	const key = id === "plugins" && addonKind(core)?.kind === "mod" ? "mods" : id
	return i18n.t(`serverTabs.${key}`)
}

/** Content of one workspace tab; shared by the desktop page and the mobile app */
export function WorkspaceContent({
	server,
	tab,
	onTabChange,
	onDeleted,
	mapFocus,
	onMapFocus,
}: {
	server: ServerConfig
	tab: WorkspaceTab
	onTabChange: (tab: WorkspaceTab) => void
	onDeleted?: () => void
	/** Lifted so it survives the layout switch between tabs */
	mapFocus: PlayerDetails | null
	onMapFocus: (player: PlayerDetails) => void
}) {
	switch (tab) {
		case "overview":
			return <OverviewPanel server={server} onNavigate={onTabChange} />
		case "console":
			return <ConsolePanel server={server} className="h-full" />
		case "players":
			return (
				<PlayersPanel
					server={server}
					onShowOnMap={(p) => {
						onMapFocus(p)
						onTabChange("map")
					}}
				/>
			)
		case "map":
			return <MapPanel server={server} focus={mapFocus} className="h-full" />
		case "plugins":
			return <PluginsPanel server={server} />
		case "settings":
			return <SettingsPanel server={server} onDeleted={onDeleted} />
	}
}

/** Full-height tabs (console, map) manage their own scrolling */
export const FILL_TABS: WorkspaceTab[] = ["console", "map"]

export function ServerWorkspace({
	server,
	tab,
	onTabChange,
	onBack,
}: {
	server: ServerConfig
	tab: WorkspaceTab
	onTabChange: (tab: WorkspaceTab) => void
	onBack: () => void
}) {
	const { t } = useTranslation()
	const { status } = useServerStatus(server.id)
	const { data: icon } = useServerIcon(server.id)
	const fill = FILL_TABS.includes(tab)
	const [mapFocus, setMapFocus] = useState<PlayerDetails | null>(null)
	const content = (
		<WorkspaceContent
			server={server}
			tab={tab}
			onTabChange={onTabChange}
			onDeleted={onBack}
			mapFocus={mapFocus}
			onMapFocus={setMapFocus}
		/>
	)

	return (
		<div className="flex size-full min-h-0 flex-col">
			<header className="flex shrink-0 flex-col gap-3 border-border/60 border-b px-5 pt-4 lg:px-6">
				<div className="flex items-center gap-3">
					<Button
						variant="ghost"
						size="icon"
						onClick={onBack}
						aria-label={t("serverWorkspace.back")}
						className="text-muted-foreground"
					>
						<ArrowLeft className="size-4" />
					</Button>
					<div className="flex size-9 items-center justify-center overflow-hidden rounded-xl border border-border bg-card">
						{icon ? (
							<img
								src={icon}
								alt=""
								className="size-full object-cover [image-rendering:pixelated]"
							/>
						) : (
							<Server className="size-4 text-muted-foreground" />
						)}
					</div>
					<div className="min-w-0">
						<div className="flex items-center gap-2">
							<h1 className="truncate font-semibold text-base text-foreground">{server.name}</h1>
							<StatusPill status={status} />
						</div>
						<p className="text-2xs text-muted-foreground capitalize">
							{server.core} {server.gameVersion}
						</p>
					</div>
				</div>
				<Tabs value={tab} onValueChange={(value) => onTabChange(value as WorkspaceTab)}>
					<TabsList
						variant="line"
						className="overflow-x-auto overflow-y-hidden [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
					>
						{WORKSPACE_TABS.map(({ id, label, icon: Icon }) => (
							<TabsTrigger key={id} value={id}>
								<Icon />
								{tabLabel(id, label, server.core)}
							</TabsTrigger>
						))}
					</TabsList>
				</Tabs>
			</header>

			{fill ? (
				<div className="min-h-0 flex-1">{content}</div>
			) : (
				<ScrollArea className="min-h-0 flex-1" scrollFade>
					<div className="mx-auto w-full max-w-5xl p-5 pb-12 lg:p-6">{content}</div>
				</ScrollArea>
			)}
		</div>
	)
}
