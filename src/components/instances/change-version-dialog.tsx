import { cn } from "cn"
import { ArrowRight, CheckCircle2 } from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type {
	InstanceConfig,
	ItemAction,
	ModLoaderType,
	VersionManifestEntry,
	VersionPlan,
} from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import { ScrollArea } from "@/components/common/scroll-area"
import SearchableSelect from "@/components/common/searchable-select"
import LoaderIcon from "@/components/instances/loader-icon"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Spinner } from "@/components/ui/spinner"
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
	const { t } = useTranslation()
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
			<DialogContent className="flex max-h-[90vh] w-full flex-col gap-3 p-4 sm:max-w-2xl sm:p-5">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2.5">
						<LoaderIcon loader={instance.loader} size={22} />
						<DialogTitle className="font-semibold text-foreground text-lg">
							{t("versionChange.title")}
						</DialogTitle>
					</div>
					<DialogDescription className="text-muted-foreground text-xs">
						{t("versionChange.instanceDescription")}
					</DialogDescription>
				</DialogHeader>

				{(stage === "pick" || stage === "checking") && (
					<div className="flex flex-col gap-4">
						<div className="grid gap-1.5">
							<span className="font-medium text-muted-foreground text-xs">
								{t("newInstance.modLoader")}
							</span>
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
												? "border-primary/60 bg-primary/10 text-foreground"
												: "border-border bg-card/40 text-muted-foreground hover:text-foreground",
										)}
									>
										<LoaderIcon loader={id} size={18} />
										{loaderName(id)}
										{id === instance.loader && (
											<span className="text-[9px] text-muted-foreground">
												{t("versionChange.now")}
											</span>
										)}
									</button>
								))}
							</div>
						</div>
						<div className="grid gap-3 sm:grid-cols-[1fr_auto_1fr] sm:items-end">
							<div className="grid gap-1.5">
								<span className="font-medium text-muted-foreground text-xs">
									{t("versionChange.nowLabel")}
								</span>
								<div className="flex h-9 items-center gap-2 border border-border bg-card/50 px-3 text-foreground/80 text-sm">
									{instance.gameVersion}
									{instance.loaderVersion && (
										<span className="truncate text-muted-foreground text-xs">
											· {loaderName(instance.loader)} {instance.loaderVersion}
										</span>
									)}
								</div>
							</div>
							<ArrowRight className="mx-auto hidden size-4 text-muted-foreground/60 sm:mb-2.5 sm:block" />
							<div className="grid gap-1.5">
								<Label
									htmlFor="target-version"
									className="font-medium text-muted-foreground text-xs"
								>
									{t("versionChange.moveTo")}
								</Label>
								<SearchableSelect
									id="target-version"
									value={target}
									onValueChange={setTarget}
									options={versions.map((v) => ({
										value: v.id,
										label: v.id,
										badge: v.id === instance.gameVersion ? t("versionChange.now") : undefined,
									}))}
									placeholder={
										versions.length
											? t("versionChange.pickVersion")
											: t("newInstance.loadingVersions")
									}
									searchPlaceholder={t("versionChange.searchVersion")}
									disabled={busy || versions.length === 0}
								/>
							</div>
						</div>
						{modded && (
							<div className="grid gap-1.5">
								<Label
									htmlFor="target-loader"
									className="font-medium text-muted-foreground text-xs"
								>
									{t("versionChange.loaderVersion", { loader: loaderName(loader) })}
								</Label>
								<SearchableSelect
									id="target-loader"
									value={loaderVersion}
									onValueChange={setLoaderVersion}
									options={loaderVersions.map((v, i) => ({
										value: v,
										label: v,
										badge: i === 0 ? t("newInstance.latestBadge") : undefined,
									}))}
									placeholder={
										loadingLoaders
											? t("newInstance.loadingVersions")
											: loaderVersions.length
												? t("versionChange.pickLoaderVersion")
												: target
													? t("versionChange.noLoaderFor", {
															loader: loaderName(loader),
															version: target,
														})
													: t("versionChange.noLoader", { loader: loaderName(loader) })
									}
									searchPlaceholder={t("newInstance.searchLoaderVersion")}
									disabled={busy || loadingLoaders || loaderVersions.length === 0}
								/>
							</div>
						)}
						{loaderChanges && (
							<Alert className={alertTone.info}>
								<AlertDescription>
									{t("versionChange.loaderSwitchNote", { loader: loaderName(loader) })}
								</AlertDescription>
							</Alert>
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
								emptyText={t("versionChange.instanceEmpty")}
							/>
						</div>
					</ScrollArea>
				)}

				{stage === "applying" && (
					<div className="flex flex-col items-center gap-3 py-10 text-center">
						<Spinner className="size-7 text-primary" />
						<p className="font-medium text-foreground text-sm">{t("versionChange.applying")}</p>
						<p className="max-w-sm text-muted-foreground text-xs leading-relaxed">
							{backupWorlds && (plan?.worlds.length ?? 0) > 0
								? t("versionChange.applyingWithWorlds")
								: t("versionChange.applyingHint")}
						</p>
					</div>
				)}

				{stage === "done" && plan && (
					<div className="flex flex-col items-center gap-3 py-8 text-center">
						<CheckCircle2 className="size-8 text-primary" />
						<p className="font-semibold text-foreground text-sm">
							{t("versionChange.nowOn", { version: planSide(plan, "to") })}
						</p>
						<p className="max-w-sm text-muted-foreground text-xs leading-relaxed">
							{t("versionChange.instanceDone", { version: planSide(plan, "from") })}
						</p>
					</div>
				)}

				{error && (
					<Alert variant="destructive" className="whitespace-pre-wrap">
						<AlertDescription>{error}</AlertDescription>
					</Alert>
				)}

				<DialogFooter className="flex flex-row items-center justify-between gap-2 pt-3 sm:justify-between">
					<p className="text-[11px] text-muted-foreground">
						{stage === "review" && plan && planSummary(plan, choices)}
					</p>
					<div className="flex gap-2">
						{stage === "review" && (
							<Button variant="ghost" size="sm" onClick={() => setStage("pick")}>
								{t("versionChange.back")}
							</Button>
						)}
						{(stage === "pick" || stage === "checking") && (
							<Button
								size="sm"
								onClick={check}
								disabled={busy || !target || unchanged || (modded && !loaderVersion)}
								className="gap-1.5"
							>
								{stage === "checking" && <Spinner className="size-3.5" />}
								{stage === "checking" ? t("versionChange.checking") : t("versionChange.check")}
							</Button>
						)}
						{stage === "review" && plan && (
							<Button size="sm" onClick={apply} disabled={plan.downgrade && !downgradeOk}>
								{t("versionChange.moveToVersion", { version: planSide(plan, "to") })}
							</Button>
						)}
						{stage === "done" && (
							<Button size="sm" onClick={() => onOpenChange(false)}>
								{t("common.done")}
							</Button>
						)}
					</div>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
