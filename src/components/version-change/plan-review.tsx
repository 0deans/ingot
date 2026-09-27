import { openUrl } from "@tauri-apps/plugin-opener"
import { cn } from "cn"
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
import type { ItemAction, ItemStatus, PlanItem, Source, VersionPlan } from "@/bindings"
import { Checkbox } from "@/components/ui/checkbox"
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

const STATUS: Record<
	ItemStatus,
	{ label: string; icon: typeof CheckCircle2; className: string; chip: string }
> = {
	conflict: {
		label: "Conflicts",
		icon: ShieldAlert,
		className: "text-red-400",
		chip: "border-red-500/30 bg-red-500/10 text-red-300",
	},
	missing: {
		label: "Not available yet",
		icon: CircleSlash,
		className: "text-amber-400",
		chip: "border-amber-500/30 bg-amber-500/10 text-amber-300",
	},
	unknown: {
		label: "Can't be checked",
		icon: CircleHelp,
		className: "text-zinc-400",
		chip: "border-zinc-600/40 bg-zinc-800/40 text-zinc-300",
	},
	newDependency: {
		label: "New dependencies",
		icon: PlusCircle,
		className: "text-sky-400",
		chip: "border-sky-500/30 bg-sky-500/10 text-sky-300",
	},
	update: {
		label: "Updates",
		icon: Download,
		className: "text-emerald-400",
		chip: "border-emerald-500/30 bg-emerald-500/10 text-emerald-300",
	},
	works: {
		label: "Already compatible",
		icon: CheckCircle2,
		className: "text-emerald-400/70",
		chip: "border-zinc-700/60 bg-zinc-800/40 text-zinc-300",
	},
}

const ACTION_LABEL: Record<ItemAction, string> = {
	keep: "Keep",
	update: "Update",
	disable: "Turn off",
	add: "Install",
	skip: "Don't install",
}

const KIND_LABEL: Record<PlanItem["kind"], string | null> = {
	mod: null,
	plugin: null,
	resourcePack: "Resource pack",
	shader: "Shader",
}

const SOURCE_LABEL: Record<Source, string> = {
	modrinth: "Modrinth",
	curseforge: "CurseForge",
	hangar: "Hangar",
}

const LOADER_LABEL: Record<string, string> = {
	neoforge: "NeoForge",
	vanilla: "Vanilla",
}

/** "fabric" -> "Fabric", "neoforge" -> "NeoForge" */
export function loaderName(id: string): string {
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
	return status === "update" && downgrade ? "Older versions" : STATUS[status].label
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
		downloads.length > 0 && `${downloads.length} to download (${formatBytes(bytes)})`,
		turnedOff > 0 && `${turnedOff} turned off`,
		keptConflicts > 0 && `${keptConflicts} conflict${keptConflicts > 1 ? "s" : ""} kept`,
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
				<span className="font-semibold text-sm text-zinc-50">
					{planSide(plan, "from")} → {planSide(plan, "to")}
				</span>
				{groups.map(({ status, entries }) => (
					<span
						key={status}
						className={cn(
							"border px-2 py-0.5 text-[11px]",
							rounded && "rounded-full",
							STATUS[status].chip,
						)}
					>
						{entries.length} {statusLabel(status, plan.downgrade).toLowerCase()}
					</span>
				))}
			</div>

			{plan.downgrade && (
				<div
					className={cn(
						"flex flex-col gap-2 border border-red-500/30 bg-red-500/10 p-3 text-red-100 text-xs",
						box,
					)}
				>
					<p className="flex items-center gap-2 font-semibold">
						<AlertTriangle className="size-4 shrink-0" />
						{plan.toGameVersion} is older than {plan.fromGameVersion}
					</p>
					<p className="text-red-200/80 leading-relaxed">
						Worlds played in a newer version usually can&apos;t be opened in an older one and may
						break. Keep the world backup on so undo can bring them back.
					</p>
					<label htmlFor="downgrade-ok" className="flex cursor-pointer items-center gap-2">
						<Checkbox
							id="downgrade-ok"
							checked={downgradeOk}
							onCheckedChange={(c) => onDowngradeOk(Boolean(c))}
						/>
						I understand, go to the older version anyway
					</label>
				</div>
			)}

			{plan.worlds.length > 0 && (
				<label
					htmlFor="backup-worlds"
					className={cn(
						"flex cursor-pointer items-start gap-2.5 border border-zinc-800 bg-zinc-900/40 p-3 text-xs",
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
						<span className="font-medium text-zinc-200">
							Back up worlds ({plan.worlds.length} {plan.worlds.length === 1 ? "world" : "worlds"},{" "}
							{formatBytes(worldsBytes)})
						</span>
						<span className="text-zinc-500 leading-relaxed">
							Minecraft upgrades worlds when they&apos;re opened, and that can&apos;t be reversed.
							With a backup, undo restores them too.
						</span>
					</span>
				</label>
			)}

			{plan.items.length === 0 && (
				<p
					className={cn(
						"border border-zinc-800 bg-zinc-900/40 p-4 text-center text-xs text-zinc-400",
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
							<span className="font-normal text-zinc-500">{entries.length}</span>
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
	const kind = KIND_LABEL[item.kind]
	const off = choice === "disable" || choice === "skip"
	const pageUrl = item.pageUrl
	return (
		<div
			className={cn(
				"flex items-center gap-3 border border-zinc-800/80 bg-zinc-900/30 px-3 py-2",
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
						"flex size-8 shrink-0 items-center justify-center bg-zinc-800 text-zinc-500",
						rounded && "rounded-lg",
					)}
				>
					<Package className="size-4" />
				</div>
			)}
			<div className="min-w-0 flex-1">
				<p className="flex items-center gap-1.5 truncate font-medium text-xs text-zinc-100">
					<span className="truncate">{item.title}</span>
					{kind && (
						<span className="shrink-0 bg-zinc-800 px-1.5 py-px font-normal text-[10px] text-zinc-400">
							{kind}
						</span>
					)}
					{item.source && pageUrl && (
						<button
							type="button"
							onClick={() => openUrl(pageUrl).catch(console.error)}
							className="inline-flex shrink-0 items-center gap-0.5 font-normal text-[10px] text-zinc-500 hover:text-zinc-200"
							title={`Open on ${SOURCE_LABEL[item.source]}`}
						>
							{SOURCE_LABEL[item.source]}
							<ExternalLink className="size-2.5" />
						</button>
					)}
				</p>
				<p className="truncate text-[11px] text-zinc-500">
					{choice === "update" || choice === "add" ? (
						<>
							{item.currentVersion ?? (item.fileName ? "?" : "new")}
							{" → "}
							<span className="text-emerald-300/90">{item.target?.versionNumber}</span>
						</>
					) : (
						(item.currentVersion ?? item.fileName)
					)}
					{item.note && <span className="text-zinc-400"> · {item.note}</span>}
				</p>
			</div>
			{item.actions.length > 1 && (
				<div
					className={cn(
						"flex shrink-0 border border-zinc-800 bg-zinc-950 p-0.5",
						rounded && "rounded-lg",
					)}
				>
					{item.actions.map((action) => (
						<button
							key={action}
							type="button"
							onClick={() => onChoose(action)}
							className={cn(
								"px-2 py-1 text-[11px] transition-colors",
								rounded && "rounded-md",
								choice === action
									? "bg-zinc-800 font-medium text-zinc-50"
									: "text-zinc-500 hover:text-zinc-200",
							)}
						>
							{ACTION_LABEL[action]}
						</button>
					))}
				</div>
			)}
		</div>
	)
}
