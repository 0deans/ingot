import { useQueryClient } from "@tanstack/react-query"
import { Check, Map as MapIcon, Radio, RefreshCw } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import type { PlayerDetails, ServerConfig } from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { dimensionStyle } from "@/lib/minecraft"
import { cn } from "@/lib/utils"
import {
	serverKeys,
	useCompanionStatus,
	useMapDimensions,
	useOnlinePlayers,
	usePluginActions,
	useServerStatus,
} from "@/services/server-data"
import { EmptyState, PlayerAvatar } from "../shared/primitives"
import { PlayerSheet } from "./player-sheet"
import { addonKind } from "./plugins-panel"
import { type MapView, WorldMap } from "./world-map"

/** How often to look for changed regions: live chunks from Ingot's plugin/mod, else saves */
const LIVE_INTERVAL_MS = 3_000
const SAVED_INTERVAL_MS = 10_000

export function MapPanel({
	server,
	focus,
	className,
}: {
	server: ServerConfig
	/** Player to center on when opened from the player list */
	focus?: PlayerDetails | null
	className?: string
}) {
	const queryClient = useQueryClient()
	const { t } = useTranslation()
	const { isRunning } = useServerStatus(server.id)
	const { data: dimensions, isLoading, refetch } = useMapDimensions(server.id)
	const { data: online = [] } = useOnlinePlayers(server.id, isRunning, 1500)
	const [dimension, setDimension] = useState(focus?.dimension ?? "minecraft:overworld")
	const [view, setView] = useState<MapView | null>(null)
	const [refreshing, setRefreshing] = useState(false)
	/** What the last refresh found, shown briefly on the button */
	const [refreshNote, setRefreshNote] = useState<string | null>(null)
	const [selected, setSelected] = useState<string | null>(null)
	const { data: companion } = useCompanionStatus(server.id)
	const { installCompanion } = usePluginActions(server.id)
	// Installed while this server was running: it loads on the next start
	const [installedAt, setInstalledAt] = useState<string | null>(null)
	const companionReady = companion?.fileName != null && companion.enabled

	const current = dimensions?.find((d) => d.id === dimension) ?? dimensions?.[0]
	const players = useMemo(
		() => (isRunning ? online.filter((p) => p.dimension === current?.id) : []),
		[online, current?.id, isRunning],
	)

	// Initial view: the focused player, else the first player here, else the middle of the explored area
	const initialized = useRef<string | null>(null)
	useEffect(() => {
		if (!current || initialized.current === current.id) return
		initialized.current = current.id
		const target = focus?.dimension === current.id ? focus : players[0]
		if (target) {
			setView({ x: target.x, z: target.z, scale: 2 })
			return
		}
		const xs = current.regions.map((r) => r.x)
		const zs = current.regions.map((r) => r.z)
		setView({
			x: ((Math.min(...xs) + Math.max(...xs) + 1) / 2) * 512,
			z: ((Math.min(...zs) + Math.max(...zs) + 1) / 2) * 512,
			scale: 0.5,
		})
	}, [current, focus, players])

	useEffect(() => {
		if (focus) {
			setDimension(focus.dimension)
			initialized.current = null
		}
	}, [focus])

	// The map never makes the server save. It re-lists regions now and then and redraws
	// the ones that changed: live chunks Ingot's plugin (or Fabric mod) reads from memory, or whatever
	// the server saved on its own schedule.
	const live = isRunning && companionReady && installedAt !== server.id
	useEffect(() => {
		if (!isRunning) {
			setInstalledAt(null)
			return
		}
		const timer = setInterval(
			() => queryClient.invalidateQueries({ queryKey: serverKeys.dimensions(server.id) }),
			live ? LIVE_INTERVAL_MS : SAVED_INTERVAL_MS,
		)
		return () => clearInterval(timer)
	}, [isRunning, live, server.id, queryClient])

	// Re-reads what the server has written and says how much changed, so the button
	// visibly does something even when the answer is "nothing new"
	const refresh = async () => {
		setRefreshing(true)
		setRefreshNote(null)
		const before = current?.regions ?? []
		const [{ data: fresh }] = await Promise.all([refetch(), new Promise((r) => setTimeout(r, 600))])
		const after = fresh?.find((d) => d.id === current?.id)?.regions ?? []
		const changed = after.filter(
			(r) => !before.some((b) => b.x === r.x && b.z === r.z && b.modified === r.modified),
		).length
		setRefreshing(false)
		setRefreshNote(changed === 0 ? t("map.upToDate") : t("map.updatedAreas", { count: changed }))
	}
	useEffect(() => {
		if (!refreshNote) return
		const timer = setTimeout(() => setRefreshNote(null), 2500)
		return () => clearTimeout(timer)
	}, [refreshNote])

	if (server.core === "bedrock") {
		return (
			<div className={cn("flex items-center justify-center", className)}>
				<EmptyState
					icon={MapIcon}
					title={t("map.bedrockUnsupported")}
					description={t("map.bedrockUnsupportedHint")}
				/>
			</div>
		)
	}

	if (isLoading) {
		return (
			<div className={cn("flex items-center justify-center", className)}>
				<Spinner className="size-5 text-muted-foreground" />
			</div>
		)
	}

	if (!current || !view) {
		return (
			<div className={cn("flex items-center justify-center", className)}>
				<EmptyState icon={MapIcon} title={t("map.noWorld")} description={t("map.noWorldHint")} />
			</div>
		)
	}

	return (
		<div className={cn("relative flex flex-col overflow-hidden", className)}>
			<WorldMap
				serverId={server.id}
				dimension={current.id}
				regions={current.regions}
				players={players}
				view={view}
				onViewChange={setView}
				onSelectPlayer={setSelected}
				voidColor={dimensionStyle(current.id).void}
			/>

			{/* Top overlay: dimension switcher and refresh */}
			<div className="pointer-events-none absolute inset-x-3 top-3 flex items-start justify-between gap-2">
				<ToggleGroup
					size="sm"
					value={[current.id]}
					onValueChange={(value) => {
						const next = dimensions?.find((d) => d.id === value[0])
						if (!next) return
						setDimension(next.id)
						initialized.current = null
					}}
					className="pointer-events-auto max-w-full overflow-x-auto border bg-background/80 p-1 backdrop-blur-md [scrollbar-width:none]"
				>
					{dimensions?.map((d) => (
						<ToggleGroupItem key={d.id} value={d.id}>
							{dimensionStyle(d.id).label}
						</ToggleGroupItem>
					))}
				</ToggleGroup>
				<div className="pointer-events-auto flex shrink-0 items-center gap-1.5">
					{live && (
						<Badge
							variant="outline"
							title={t("map.liveTitle")}
							className="h-8 bg-background/80 text-primary backdrop-blur-md"
						>
							<span className="size-1.5 animate-pulse rounded-full bg-primary" />
							{t("map.live")}
						</Badge>
					)}
					{isRunning && installedAt === server.id && (
						<Badge
							variant="outline"
							className="h-8 border-warning/30 bg-background/80 text-warning backdrop-blur-md"
						>
							{t("map.restartToGoLive")}
						</Badge>
					)}
					{companion?.supported && companion.fileName === null && (
						<Tooltip>
							<TooltipTrigger
								render={
									<Button
										variant="outline"
										size="sm"
										onClick={async () => {
											await installCompanion.mutateAsync().catch(() => {})
											if (isRunning) setInstalledAt(server.id)
										}}
										disabled={installCompanion.isPending}
										className="bg-background/80 backdrop-blur-md"
									/>
								}
							>
								{installCompanion.isPending ? (
									<Spinner className="size-3.5" />
								) : (
									<Radio className="size-3.5 text-primary" />
								)}
								{t("map.makeLive")}
							</TooltipTrigger>
							<TooltipContent>
								{t("map.makeLiveTitle", { context: addonKind(server.core)?.kind ?? "plugin" })}
							</TooltipContent>
						</Tooltip>
					)}
					{/* Live maps update themselves; otherwise re-read what the server saved */}
					{!live && (
						<Tooltip>
							<TooltipTrigger
								render={
									<Button
										variant="outline"
										size="sm"
										onClick={refresh}
										disabled={refreshing}
										className="bg-background/80 backdrop-blur-md"
									/>
								}
							>
								{refreshNote ? (
									<Check className="size-3.5 text-primary" />
								) : (
									<RefreshCw className={cn("size-3.5", refreshing && "animate-spin")} />
								)}
								{refreshNote ? (
									<span>{refreshNote}</span>
								) : (
									<span className="hidden sm:inline">
										{refreshing ? t("map.checking") : t("common.refresh")}
									</span>
								)}
							</TooltipTrigger>
							<TooltipContent>{t("map.refreshTitle")}</TooltipContent>
						</Tooltip>
					)}
				</div>
			</div>

			{/* Players in this dimension */}
			{players.length > 0 && (
				<div className="absolute top-16 left-3 flex max-w-[60%] flex-col gap-1.5">
					{players.map((p) => (
						<Button
							key={p.name}
							variant="outline"
							size="sm"
							onClick={() =>
								setView((v) => ({ x: p.x, z: p.z, scale: Math.max(v?.scale ?? 2, 2) }))
							}
							className="justify-start bg-background/80 pl-1 backdrop-blur-md"
						>
							<PlayerAvatar name={p.name} size={20} />
							<span className="truncate">{p.name}</span>
						</Button>
					))}
				</div>
			)}

			<PlayerSheet
				serverId={server.id}
				name={selected}
				isRunning={isRunning}
				onClose={() => setSelected(null)}
			/>
		</div>
	)
}
