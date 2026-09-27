import { cn } from "cn"
import {
	AlertTriangle,
	ArrowRight,
	CheckCircle2,
	CircleHelp,
	CircleSlash,
	Download,
	Loader2,
	Package,
	PlusCircle,
	ShieldAlert,
} from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import type {
	InstanceConfig,
	ItemAction,
	ItemStatus,
	PlanItem,
	VersionManifestEntry,
	VersionPlan,
} from "@/bindings"
import LoaderIcon from "@/components/instances/loader-icon"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import SearchableSelect from "@/components/ui/searchable-select"
import { formatBytes } from "@/lib/minecraft"
import { instanceService, rpc } from "@/services/instance-service"

interface ChangeVersionDialogProps {
	instance: InstanceConfig
	open: boolean
	onOpenChange: (open: boolean) => void
	/** After the change is applied (the instance now has the new version) */
	onChanged: (updated: InstanceConfig) => void
}

type Stage = "pick" | "checking" | "review" | "applying" | "done"

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

/** Going to an older version, "updates" are older versions */
function statusLabel(status: ItemStatus, downgrade: boolean): string {
	return status === "update" && downgrade ? "Older versions" : STATUS[status].label
}

const ACTION_LABEL: Record<ItemAction, string> = {
	keep: "Keep",
	update: "Update",
	disable: "Turn off",
	add: "Install",
	skip: "Don't install",
}

const KIND_LABEL = { mod: null, resourcePack: "Resource pack", shader: "Shader" } as const

export default function ChangeVersionDialog({
	instance,
	open,
	onOpenChange,
	onChanged,
}: ChangeVersionDialogProps) {
	const [stage, setStage] = useState<Stage>("pick")
	const [versions, setVersions] = useState<VersionManifestEntry[]>([])
	const [target, setTarget] = useState("")
	const [loaderVersions, setLoaderVersions] = useState<string[]>([])
	const [loaderVersion, setLoaderVersion] = useState("")
	const [loadingLoaders, setLoadingLoaders] = useState(false)
	const [plan, setPlan] = useState<VersionPlan | null>(null)
	/** The user's choice per item (index into plan.items) */
	const [choices, setChoices] = useState<ItemAction[]>([])
	const [backupWorlds, setBackupWorlds] = useState(true)
	const [downgradeOk, setDowngradeOk] = useState(false)
	const [error, setError] = useState<string | null>(null)

	const modded = instance.loader !== "vanilla"

	// Mounted fresh for each opening (see the settings dialog), so state starts clean
	useEffect(() => {
		if (!open) return
		instanceService
			.getAvailableGameVersions()
			.then((list) => {
				const releases = list.filter((v) => v.type === "release")
				setVersions(releases)
				setTarget(
					(current) => current || releases.find((v) => v.id !== instance.gameVersion)?.id || "",
				)
			})
			.catch((e) => setError(String(e)))
	}, [open, instance.gameVersion])

	useEffect(() => {
		if (!open || !modded || !target) {
			setLoaderVersions([])
			setLoaderVersion("")
			return
		}
		let cancelled = false
		setLoadingLoaders(true)
		instanceService
			.getAvailableLoaderVersions(target, instance.loader)
			.then((list) => {
				if (cancelled) return
				setLoaderVersions(list)
				setLoaderVersion(list[0] ?? "")
			})
			.catch(() => !cancelled && setLoaderVersions([]))
			.finally(() => !cancelled && setLoadingLoaders(false))
		return () => {
			cancelled = true
		}
	}, [open, modded, target, instance.loader])

	const check = async () => {
		setError(null)
		setStage("checking")
		try {
			const result = await rpc.check_instance_version_change(
				instance.id,
				target,
				modded ? loaderVersion : null,
			)
			setPlan(result)
			setChoices(result.items.map((i) => i.action))
			setBackupWorlds(true)
			setDowngradeOk(false)
			setStage("review")
		} catch (e) {
			setError(String(e))
			setStage("pick")
		}
	}

	const apply = async () => {
		if (!plan) return
		setError(null)
		setStage("applying")
		try {
			const updated = await rpc.apply_instance_version_change(
				{ ...plan, items: plan.items.map((item, i) => ({ ...item, action: choices[i] })) },
				backupWorlds,
			)
			setStage("done")
			onChanged(updated)
		} catch (e) {
			setError(String(e))
			setStage("review")
		}
	}

	const groups = useMemo(() => {
		if (!plan) return []
		return STATUS_ORDER.map((status) => ({
			status,
			entries: plan.items
				.map((item, index) => ({ item, index }))
				.filter(({ item }) => item.status === status),
		})).filter((g) => g.entries.length > 0)
	}, [plan])

	const downloads = plan
		? plan.items.filter((_, i) => choices[i] === "update" || choices[i] === "add")
		: []
	const downloadBytes = downloads.reduce((sum, item) => sum + (item.target?.bytes ?? 0), 0)
	const turnedOff = plan ? choices.filter((c) => c === "disable").length : 0
	const worldsBytes = plan?.worlds.reduce((sum, w) => sum + w.bytes, 0) ?? 0
	/** Keeping a conflicting mod is allowed, but the user should know it may not start */
	const keptConflicts = plan
		? plan.items.filter((item, i) => item.status === "conflict" && choices[i] === "keep").length
		: 0
	const blocked = Boolean(plan?.downgrade && !downgradeOk)
	const busy = stage === "checking" || stage === "applying"

	return (
		<Dialog open={open} onOpenChange={(o) => !busy && onOpenChange(o)}>
			<DialogContent className="flex max-h-[90vh] w-full flex-col gap-3 border-border/60 bg-zinc-950 p-4 sm:max-w-2xl sm:p-5">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2.5">
						<LoaderIcon loader={instance.loader} size={22} />
						<DialogTitle className="font-semibold text-lg text-white">Change version</DialogTitle>
					</div>
					<DialogDescription className="text-muted-foreground text-xs">
						Ingot checks every mod, resource pack and shader first and changes nothing until you
						apply. You can undo it afterwards.
					</DialogDescription>
				</DialogHeader>

				{(stage === "pick" || stage === "checking") && (
					<div className="flex flex-col gap-4">
						<div className="grid gap-3 sm:grid-cols-[1fr_auto_1fr] sm:items-end">
							<div className="grid gap-1.5">
								<span className="font-medium text-muted-foreground text-xs">Now</span>
								<div className="flex h-9 items-center gap-2 border border-zinc-800 bg-zinc-900/50 px-3 text-sm text-zinc-300">
									{instance.gameVersion}
									{instance.loaderVersion && (
										<span className="truncate text-xs text-zinc-500">
											· loader {instance.loaderVersion}
										</span>
									)}
								</div>
							</div>
							<ArrowRight className="mx-auto hidden size-4 text-zinc-600 sm:mb-2.5 sm:block" />
							<div className="grid gap-1.5">
								<label
									htmlFor="target-version"
									className="font-medium text-muted-foreground text-xs"
								>
									Move to
								</label>
								<SearchableSelect
									id="target-version"
									value={target}
									onValueChange={setTarget}
									options={versions
										.filter((v) => v.id !== instance.gameVersion)
										.map((v) => ({ value: v.id, label: v.id }))}
									placeholder={versions.length ? "Pick a version" : "Loading versions..."}
									searchPlaceholder="Search version..."
									disabled={busy || versions.length === 0}
								/>
							</div>
						</div>
						{modded && (
							<div className="grid gap-1.5">
								<label
									htmlFor="target-loader"
									className="font-medium text-muted-foreground text-xs"
								>
									Loader version
								</label>
								<SearchableSelect
									id="target-loader"
									value={loaderVersion}
									onValueChange={setLoaderVersion}
									options={loaderVersions.map((v, i) => ({
										value: v,
										label: v,
										badge: i === 0 ? "latest" : undefined,
									}))}
									placeholder={
										loadingLoaders
											? "Loading versions..."
											: loaderVersions.length
												? "Pick a loader version"
												: `No loader for ${target || "this version"}`
									}
									searchPlaceholder="Search loader version..."
									disabled={busy || loadingLoaders || loaderVersions.length === 0}
								/>
							</div>
						)}
					</div>
				)}

				{stage === "review" && plan && (
					<ScrollArea scrollFade className="-mr-2 max-h-[60vh] pr-2">
						<div className="flex flex-col gap-3 pr-1 pb-1">
							<div className="flex flex-wrap items-center gap-2">
								<span className="font-semibold text-sm text-white">
									{plan.fromGameVersion} → {plan.toGameVersion}
								</span>
								{groups.map(({ status, entries }) => (
									<span
										key={status}
										className={cn("border px-2 py-0.5 text-[11px]", STATUS[status].chip)}
									>
										{entries.length} {statusLabel(status, plan.downgrade).toLowerCase()}
									</span>
								))}
							</div>

							{plan.downgrade && (
								<div className="flex flex-col gap-2 border border-red-500/30 bg-red-500/10 p-3 text-red-100 text-xs">
									<p className="flex items-center gap-2 font-semibold">
										<AlertTriangle className="size-4 shrink-0" />
										{plan.toGameVersion} is older than {plan.fromGameVersion}
									</p>
									<p className="text-red-200/80 leading-relaxed">
										Worlds played in a newer version usually can&apos;t be opened in an older one
										and may break. Keep the world backup on so undo can bring them back.
									</p>
									<label htmlFor="downgrade-ok" className="flex cursor-pointer items-center gap-2">
										<Checkbox
											id="downgrade-ok"
											checked={downgradeOk}
											onCheckedChange={(c) => setDowngradeOk(Boolean(c))}
										/>
										I understand, go to the older version anyway
									</label>
								</div>
							)}

							{plan.worlds.length > 0 && (
								<label
									htmlFor="backup-worlds"
									className="flex cursor-pointer items-start gap-2.5 border border-zinc-800 bg-zinc-900/40 p-3 text-xs"
								>
									<Checkbox
										id="backup-worlds"
										checked={backupWorlds}
										onCheckedChange={(c) => setBackupWorlds(Boolean(c))}
										className="mt-0.5"
									/>
									<span className="flex flex-col gap-0.5">
										<span className="font-medium text-zinc-200">
											Back up worlds ({plan.worlds.length}{" "}
											{plan.worlds.length === 1 ? "world" : "worlds"}, {formatBytes(worldsBytes)})
										</span>
										<span className="text-zinc-500 leading-relaxed">
											Minecraft upgrades worlds when they're opened, and that can't be reversed.
											With a backup, undo restores them too.
										</span>
									</span>
								</label>
							)}

							{plan.items.length === 0 && (
								<p className="border border-zinc-800 bg-zinc-900/40 p-4 text-center text-xs text-zinc-400">
									No mods, resource packs or shaders to check. Only the version changes.
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
												key={`${item.fileName ?? item.projectId}-${index}`}
												item={item}
												choice={choices[index]}
												onChoose={(action) =>
													setChoices((prev) => prev.map((c, i) => (i === index ? action : c)))
												}
											/>
										))}
									</section>
								)
							})}
						</div>
					</ScrollArea>
				)}

				{stage === "applying" && (
					<div className="flex flex-col items-center gap-3 py-10 text-center">
						<Loader2 className="size-7 animate-spin text-emerald-400" />
						<p className="font-medium text-sm text-zinc-200">Applying...</p>
						<p className="max-w-sm text-xs text-zinc-500 leading-relaxed">
							Downloading and checking the new files, then backing up
							{backupWorlds && (plan?.worlds.length ?? 0) > 0 ? " your worlds and" : ""} the current
							ones. Nothing changes until everything is ready.
						</p>
					</div>
				)}

				{stage === "done" && plan && (
					<div className="flex flex-col items-center gap-3 py-8 text-center">
						<CheckCircle2 className="size-8 text-emerald-400" />
						<p className="font-semibold text-sm text-white">Now on {plan.toGameVersion}</p>
						<p className="max-w-sm text-xs text-zinc-400 leading-relaxed">
							Changed from {plan.fromGameVersion}. If something doesn&apos;t work, undo it from this
							instance&apos;s settings: everything goes back exactly as it was.
						</p>
					</div>
				)}

				{error && (
					<p className="whitespace-pre-wrap border border-red-500/30 bg-red-500/10 p-3 text-red-200 text-xs">
						{error}
					</p>
				)}

				<DialogFooter className="flex flex-row items-center justify-between gap-2 border-border/40 border-t pt-3 sm:justify-between">
					<p className="text-[11px] text-zinc-500">
						{stage === "review" &&
							[
								downloads.length > 0 &&
									`${downloads.length} to download (${formatBytes(downloadBytes)})`,
								turnedOff > 0 && `${turnedOff} turned off`,
								keptConflicts > 0 &&
									`${keptConflicts} conflict${keptConflicts > 1 ? "s" : ""} kept`,
							]
								.filter(Boolean)
								.join(" · ")}
					</p>
					<div className="flex gap-2">
						{stage === "review" && (
							<Button variant="ghost" size="sm" onClick={() => setStage("pick")}>
								Back
							</Button>
						)}
						{(stage === "pick" || stage === "checking") && (
							<Button
								size="sm"
								onClick={check}
								disabled={busy || !target || (modded && !loaderVersion)}
								className="gap-1.5 bg-emerald-600 text-white hover:bg-emerald-500"
							>
								{stage === "checking" && <Loader2 className="size-3.5 animate-spin" />}
								{stage === "checking" ? "Checking..." : "Check compatibility"}
							</Button>
						)}
						{stage === "review" && (
							<Button
								size="sm"
								onClick={apply}
								disabled={blocked}
								className="bg-emerald-600 text-white hover:bg-emerald-500"
							>
								Move to {plan?.toGameVersion}
							</Button>
						)}
						{stage === "done" && (
							<Button size="sm" onClick={() => onOpenChange(false)}>
								Done
							</Button>
						)}
					</div>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

function PlanRow({
	item,
	choice,
	onChoose,
}: {
	item: PlanItem
	choice: ItemAction
	onChoose: (action: ItemAction) => void
}) {
	const kind = KIND_LABEL[item.kind]
	const off = choice === "disable" || choice === "skip"
	return (
		<div
			className={cn(
				"flex items-center gap-3 border border-zinc-800/80 bg-zinc-900/30 px-3 py-2",
				off && "opacity-60",
			)}
		>
			{item.iconUrl ? (
				<img src={item.iconUrl} alt="" className="size-8 shrink-0 object-cover" draggable={false} />
			) : (
				<div className="flex size-8 shrink-0 items-center justify-center bg-zinc-800 text-zinc-500">
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
				<div className="flex shrink-0 border border-zinc-800 bg-zinc-950 p-0.5">
					{item.actions.map((action) => (
						<button
							key={action}
							type="button"
							onClick={() => onChoose(action)}
							className={cn(
								"px-2 py-1 text-[11px] transition-colors",
								choice === action
									? "bg-zinc-800 font-medium text-white"
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
