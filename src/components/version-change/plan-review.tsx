import { openUrl } from "@tauri-apps/plugin-opener"
import { cn } from "cn"
import i18n from "i18next"
import {
	AlertTriangle,
	CheckCircle2,
	CircleHelp,
	CircleSlash,
	Download,
	ExternalLink,
	Package,
	PlusCircle,
	ShieldAlert,
} from "lucide-react"
import { useMemo } from "react"
import { useTranslation } from "react-i18next"
import type { ItemAction, ItemStatus, PlanItem, Source, VersionPlan } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { Label } from "@/components/ui/label"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { translatePlanNote } from "@/lib/backend-text"
import { formatBytes } from "@/lib/minecraft"

/** Most urgent first: that's what needs a decision */
const STATUS_ORDER: ItemStatus[] = [
	"conflict",
	"missing",
	"unknown",
	"newDependency",
	"update",
	"works",
]

const STATUS: Record<ItemStatus, { icon: typeof CheckCircle2; className: string; chip: string }> = {
	conflict: {
		icon: ShieldAlert,
		className: "text-destructive",
		chip: "border-destructive/30 bg-destructive/10 text-destructive",
	},
	missing: {
		icon: CircleSlash,
		className: "text-warning",
		chip: "border-warning/30 bg-warning/10 text-warning",
	},
	unknown: {
		icon: CircleHelp,
		className: "text-muted-foreground",
		chip: "border-input/40 bg-muted/40 text-foreground/80",
	},
	newDependency: {
		icon: PlusCircle,
		className: "text-info",
		chip: "border-info/30 bg-info/10 text-info",
	},
	update: {
		icon: Download,
		className: "text-primary",
		chip: "border-primary/30 bg-primary/10 text-primary",
	},
	works: {
		icon: CheckCircle2,
		className: "text-primary/70",
		chip: "border-input/60 bg-muted/40 text-foreground/80",
	},
}

/** Resource packs and shaders get a label; mods and plugins are the default */
const LABELED_KINDS: PlanItem["kind"][] = ["resourcePack", "shader"]

const SOURCE_LABEL: Record<Source, string> = {
	modrinth: "Modrinth",
	curseforge: "CurseForge",
	hangar: "Hangar",
}

const LOADER_LABEL: Record<string, string> = {
	neoforge: "NeoForge",
}

/** "fabric" -> "Fabric", "neoforge" -> "NeoForge", "vanilla" in the user's language */
export function loaderName(id: string): string {
	if (id === "vanilla") return i18n.t("versionChange.vanilla")
	return LOADER_LABEL[id] ?? id.charAt(0).toUpperCase() + id.slice(1)
}

/** "1.21.4" or, when the loader changes too, "Fabric 1.21.4" */
export function planSide(plan: VersionPlan, side: "from" | "to"): string {
	const version = side === "from" ? plan.fromGameVersion : plan.toGameVersion
	if (plan.fromLoader === plan.toLoader) return version
	return `${loaderName(side === "from" ? plan.fromLoader : plan.toLoader)} ${version}`
}

/** Going to an older version, "updates" are older versions */
function statusLabel(status: ItemStatus, downgrade: boolean): string {
	return status === "update" && downgrade
		? i18n.t("versionChange.status.olderVersions")
		: i18n.t(`versionChange.status.${status}`)
}

/** Footer line: what applying will do */
export function planSummary(plan: VersionPlan, choices: ItemAction[]): string {
	const downloads = plan.items.filter((_, i) => choices[i] === "update" || choices[i] === "add")
	const bytes = downloads.reduce((sum, item) => sum + (item.target?.bytes ?? 0), 0)
	const turnedOff = choices.filter((c) => c === "disable").length
	// Keeping a conflicting mod is allowed, but the user should know it may not start
	const keptConflicts = plan.items.filter(
		(item, i) => item.status === "conflict" && choices[i] === "keep",
	).length
	return [
		downloads.length > 0 &&
			i18n.t("versionChange.summary.download", {
				count: downloads.length,
				size: formatBytes(bytes),
			}),
		turnedOff > 0 && i18n.t("versionChange.summary.turnedOff", { count: turnedOff }),
		keptConflicts > 0 && i18n.t("versionChange.summary.conflictsKept", { count: keptConflicts }),
	]
		.filter(Boolean)
		.join(" · ")
}

/** The checked plan: warnings, world backup and every file grouped by what it needs */
export function PlanReview({
	plan,
	choices,
	onChoose,
	backupWorlds,
	onBackupWorlds,
	downgradeOk,
	onDowngradeOk,
	emptyText,
	rounded = false,
}: {
	plan: VersionPlan
	choices: ItemAction[]
	onChoose: (index: number, action: ItemAction) => void
	backupWorlds: boolean
	onBackupWorlds: (value: boolean) => void
	downgradeOk: boolean
	onDowngradeOk: (value: boolean) => void
	emptyText: string
	/** Server panels use rounded cards; the launcher side is square */
	rounded?: boolean
}) {
	const { t } = useTranslation()
	const groups = useMemo(
		() =>
			STATUS_ORDER.map((status) => ({
				status,
				entries: plan.items
					.map((item, index) => ({ item, index }))
					.filter(({ item }) => item.status === status),
			})).filter((g) => g.entries.length > 0),
		[plan],
	)
	const worldsBytes = plan.worlds.reduce((sum, w) => sum + w.bytes, 0)
	const box = rounded ? "rounded-xl" : ""

	return (
		<div className="flex flex-col gap-3">
			<div className="flex flex-wrap items-center gap-2">
				<span className="font-semibold text-foreground text-sm">
					{planSide(plan, "from")} → {planSide(plan, "to")}
				</span>
				{groups.map(({ status, entries }) => (
					<span
						key={status}
						className={cn(
							"border px-2 py-0.5 text-2xs",
							rounded && "rounded-full",
							STATUS[status].chip,
						)}
					>
						{entries.length} {statusLabel(status, plan.downgrade).toLocaleLowerCase()}
					</span>
				))}
			</div>

			{plan.downgrade && (
				<div
					className={cn(
						"flex flex-col gap-2 border border-destructive/30 bg-destructive/10 p-3 text-destructive text-xs",
						box,
					)}
				>
					<p className="flex items-center gap-2 font-semibold">
						<AlertTriangle className="size-4 shrink-0" />
						{t("versionChange.downgradeTitle", {
							to: plan.toGameVersion,
							from: plan.fromGameVersion,
						})}
					</p>
					<p className="text-destructive/80 leading-relaxed">
						{t("versionChange.downgradeWarning")}
					</p>
					<Label htmlFor="downgrade-ok" className="flex cursor-pointer items-center gap-2">
						<Checkbox
							id="downgrade-ok"
							checked={downgradeOk}
							onCheckedChange={(c) => onDowngradeOk(Boolean(c))}
						/>
						{t("versionChange.downgradeOk")}
					</Label>
				</div>
			)}

			{plan.worlds.length > 0 && (
				<Label
					htmlFor="backup-worlds"
					className={cn(
						"flex cursor-pointer items-start gap-2.5 border border-border bg-card/40 p-3 text-xs",
						box,
					)}
				>
					<Checkbox
						id="backup-worlds"
						checked={backupWorlds}
						onCheckedChange={(c) => onBackupWorlds(Boolean(c))}
						className="mt-0.5"
					/>
					<span className="flex flex-col gap-0.5">
						<span className="font-medium text-foreground">
							{t("versionChange.backupWorlds", {
								count: plan.worlds.length,
								size: formatBytes(worldsBytes),
							})}
						</span>
						<span className="text-muted-foreground leading-relaxed">
							{t("versionChange.backupWorldsHint")}
						</span>
					</span>
				</Label>
			)}

			{plan.items.length === 0 && (
				<p
					className={cn(
						"border border-border bg-card/40 p-4 text-center text-muted-foreground text-xs",
						box,
					)}
				>
					{emptyText}
				</p>
			)}

			{groups.map(({ status, entries }) => {
				const Icon = STATUS[status].icon
				return (
					<section key={status} className="flex flex-col gap-1.5">
						<h3
							className={cn(
								"flex items-center gap-1.5 font-semibold text-xs",
								STATUS[status].className,
							)}
						>
							<Icon className="size-3.5" />
							{statusLabel(status, plan.downgrade)}
							<span className="font-normal text-muted-foreground">{entries.length}</span>
						</h3>
						{entries.map(({ item, index }) => (
							<PlanRow
								key={`${item.folder}/${item.fileName ?? item.projectId}-${index}`}
								item={item}
								choice={choices[index]}
								onChoose={(action) => onChoose(index, action)}
								rounded={rounded}
							/>
						))}
					</section>
				)
			})}
		</div>
	)
}

function PlanRow({
	item,
	choice,
	onChoose,
	rounded,
}: {
	item: PlanItem
	choice: ItemAction
	onChoose: (action: ItemAction) => void
	rounded: boolean
}) {
	const { t } = useTranslation()
	const kind = LABELED_KINDS.includes(item.kind) ? t(`versionChange.kinds.${item.kind}`) : null
	const off = choice === "disable" || choice === "skip"
	const pageUrl = item.pageUrl
	return (
		<div
			className={cn(
				"flex items-center gap-3 border border-border/80 bg-card/30 px-3 py-2",
				rounded && "rounded-xl",
				off && "opacity-60",
			)}
		>
			{item.iconUrl ? (
				<img
					src={item.iconUrl}
					alt=""
					className={cn("size-8 shrink-0 object-cover", rounded && "rounded-lg")}
					draggable={false}
				/>
			) : (
				<div
					className={cn(
						"flex size-8 shrink-0 items-center justify-center bg-muted text-muted-foreground",
						rounded && "rounded-lg",
					)}
				>
					<Package className="size-4" />
				</div>
			)}
			<div className="min-w-0 flex-1">
				<p className="flex items-center gap-1.5 truncate font-medium text-foreground text-xs">
					<span className="truncate">{item.title}</span>
					{kind && (
						<span className="shrink-0 bg-muted px-1.5 py-px font-normal text-3xs text-muted-foreground">
							{kind}
						</span>
					)}
					{item.source && pageUrl && (
						<Tooltip>
							<TooltipTrigger
								render={
									<Button
										variant="link"
										size="xs"
										onClick={() => openUrl(pageUrl).catch(console.error)}
										className="h-auto gap-0.5 p-0 font-normal text-3xs text-muted-foreground"
										aria-label={t("versionChange.openOn", { site: SOURCE_LABEL[item.source] })}
									/>
								}
							>
								{SOURCE_LABEL[item.source]}
								<ExternalLink className="size-2.5" />
							</TooltipTrigger>
							<TooltipContent>
								{t("versionChange.openOn", { site: SOURCE_LABEL[item.source] })}
							</TooltipContent>
						</Tooltip>
					)}
				</p>
				<p className="truncate text-2xs text-muted-foreground">
					{choice === "update" || choice === "add" ? (
						<>
							{item.currentVersion ?? (item.fileName ? "?" : t("versionChange.new"))}
							{" → "}
							<span className="text-primary/90">{item.target?.versionNumber}</span>
						</>
					) : (
						(item.currentVersion ?? item.fileName)
					)}
					{item.note && (
						<span className="text-muted-foreground"> · {translatePlanNote(item.note)}</span>
					)}
				</p>
			</div>
			{item.actions.length > 1 && (
				<ToggleGroup
					variant="outline"
					size="sm"
					spacing={0}
					value={choice ? [choice] : []}
					onValueChange={(value) => {
						const action = item.actions.find((a) => a === value[0])
						if (action) onChoose(action)
					}}
					className={cn("shrink-0", !rounded && "rounded-none *:rounded-none")}
				>
					{item.actions.map((action) => (
						<ToggleGroupItem key={action} value={action}>
							{t(`versionChange.actions.${action}`)}
						</ToggleGroupItem>
					))}
				</ToggleGroup>
			)}
		</div>
	)
}
