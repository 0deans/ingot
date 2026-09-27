import { cn } from "cn"
import { ArrowRight, CheckCircle2, Loader2 } from "lucide-react"
import { useEffect, useState } from "react"
import type {
	InstanceConfig,
	ItemAction,
	ModLoaderType,
	VersionManifestEntry,
	VersionPlan,
} from "@/bindings"
import LoaderIcon from "@/components/instances/loader-icon"
import { Button } from "@/components/ui/button"
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
import {
	loaderName,
	PlanReview,
	planSide,
	planSummary,
} from "@/components/version-change/plan-review"
import { instanceService, rpc } from "@/services/instance-service"

interface ChangeVersionDialogProps {
	instance: InstanceConfig
	open: boolean
	onOpenChange: (open: boolean) => void
	/** After the change is applied (the instance now has the new version) */
	onChanged: (updated: InstanceConfig) => void
}

type Stage = "pick" | "checking" | "review" | "applying" | "done"

const LOADERS: ModLoaderType[] = ["vanilla", "fabric", "quilt", "neoforge", "forge"]

export default function ChangeVersionDialog({
	instance,
	open,
	onOpenChange,
	onChanged,
}: ChangeVersionDialogProps) {
	const [stage, setStage] = useState<Stage>("pick")
	const [versions, setVersions] = useState<VersionManifestEntry[]>([])
	const [target, setTarget] = useState("")
	const [loader, setLoader] = useState<ModLoaderType>(instance.loader)
	const [loaderVersions, setLoaderVersions] = useState<string[]>([])
	const [loaderVersion, setLoaderVersion] = useState("")
	const [loadingLoaders, setLoadingLoaders] = useState(false)
	const [plan, setPlan] = useState<VersionPlan | null>(null)
	/** The user's choice per item (index into plan.items) */
	const [choices, setChoices] = useState<ItemAction[]>([])
	const [backupWorlds, setBackupWorlds] = useState(true)
	const [downgradeOk, setDowngradeOk] = useState(false)
	const [error, setError] = useState<string | null>(null)

	const modded = loader !== "vanilla"
	const loaderChanges = loader !== instance.loader

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
			.getAvailableLoaderVersions(target, loader)
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
	}, [open, modded, target, loader])

	const check = async () => {
		setError(null)
		setStage("checking")
		try {
			const result = await rpc.check_instance_version_change(
				instance.id,
				target,
				loader,
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

	const busy = stage === "checking" || stage === "applying"
	// The same version and loader: nothing would change
	const unchanged = target === instance.gameVersion && !loaderChanges

	return (
		<Dialog open={open} onOpenChange={(o) => !busy && onOpenChange(o)}>
			<DialogContent className="flex max-h-[90vh] w-full flex-col gap-3 border-border/60 bg-zinc-950 p-4 sm:max-w-2xl sm:p-5">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2.5">
						<LoaderIcon loader={instance.loader} size={22} />
						<DialogTitle className="font-semibold text-lg text-zinc-50">Change version</DialogTitle>
					</div>
					<DialogDescription className="text-muted-foreground text-xs">
						Ingot checks every mod, resource pack and shader first and changes nothing until you
						apply. You can undo it afterwards.
					</DialogDescription>
				</DialogHeader>

				{(stage === "pick" || stage === "checking") && (
					<div className="flex flex-col gap-4">
						<div className="grid gap-1.5">
							<span className="font-medium text-muted-foreground text-xs">Mod loader</span>
							<div className="grid grid-cols-5 gap-1.5">
								{LOADERS.map((id) => (
									<button
										key={id}
										type="button"
										disabled={busy}
										onClick={() => setLoader(id)}
										className={cn(
											"flex flex-col items-center gap-1 border p-2 text-[11px] transition-colors",
											loader === id
												? "border-emerald-500/60 bg-emerald-500/10 text-zinc-50"
												: "border-zinc-800 bg-zinc-900/40 text-zinc-400 hover:text-zinc-200",
										)}
									>
										<LoaderIcon loader={id} size={18} />
										{loaderName(id)}
										{id === instance.loader && (
											<span className="text-[9px] text-zinc-500">now</span>
										)}
									</button>
								))}
							</div>
						</div>
						<div className="grid gap-3 sm:grid-cols-[1fr_auto_1fr] sm:items-end">
							<div className="grid gap-1.5">
								<span className="font-medium text-muted-foreground text-xs">Now</span>
								<div className="flex h-9 items-center gap-2 border border-zinc-800 bg-zinc-900/50 px-3 text-sm text-zinc-300">
									{instance.gameVersion}
									{instance.loaderVersion && (
										<span className="truncate text-xs text-zinc-500">
											· {loaderName(instance.loader)} {instance.loaderVersion}
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
									options={versions.map((v) => ({
										value: v.id,
										label: v.id,
										badge: v.id === instance.gameVersion ? "now" : undefined,
									}))}
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
									{loaderName(loader)} version
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
												: `No ${loaderName(loader)} for ${target || "this version"}`
									}
									searchPlaceholder="Search loader version..."
									disabled={busy || loadingLoaders || loaderVersions.length === 0}
								/>
							</div>
						)}
						{loaderChanges && (
							<p className="border border-sky-500/20 bg-sky-500/5 p-3 text-sky-100/80 text-xs leading-relaxed">
								Mods are made for one loader. Ingot looks for the {loaderName(loader)} version of
								each mod and turns off the ones that don&apos;t have one.
							</p>
						)}
					</div>
				)}

				{stage === "review" && plan && (
					<ScrollArea scrollFade className="-mr-2 max-h-[60vh] pr-2">
						<div className="pr-1 pb-1">
							<PlanReview
								plan={plan}
								choices={choices}
								onChoose={(index, action) =>
									setChoices((prev) => prev.map((c, i) => (i === index ? action : c)))
								}
								backupWorlds={backupWorlds}
								onBackupWorlds={setBackupWorlds}
								downgradeOk={downgradeOk}
								onDowngradeOk={setDowngradeOk}
								emptyText="No mods, resource packs or shaders to check. Only the version changes."
							/>
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
						<p className="font-semibold text-sm text-zinc-50">Now on {planSide(plan, "to")}</p>
						<p className="max-w-sm text-xs text-zinc-400 leading-relaxed">
							Changed from {planSide(plan, "from")}. If something doesn&apos;t work, undo it from
							this instance&apos;s settings: everything goes back exactly as it was.
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
						{stage === "review" && plan && planSummary(plan, choices)}
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
								disabled={busy || !target || unchanged || (modded && !loaderVersion)}
								className="gap-1.5 bg-emerald-600 text-white hover:bg-emerald-500"
							>
								{stage === "checking" && <Loader2 className="size-3.5 animate-spin" />}
								{stage === "checking" ? "Checking..." : "Check compatibility"}
							</Button>
						)}
						{stage === "review" && plan && (
							<Button
								size="sm"
								onClick={apply}
								disabled={plan.downgrade && !downgradeOk}
								className="bg-emerald-600 text-white hover:bg-emerald-500"
							>
								Move to {planSide(plan, "to")}
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
