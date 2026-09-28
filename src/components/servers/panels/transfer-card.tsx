import { useQuery, useQueryClient } from "@tanstack/react-query"
import { save as saveDialog } from "@tauri-apps/plugin-dialog"
import {
	ArrowUpDown,
	CheckCircle2,
	Copy,
	FileArchive,
	Loader2,
	Package,
	Share2,
	Undo2,
	Upload,
} from "lucide-react"
import { useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import type {
	ExportMode,
	ImportedServer,
	ItemAction,
	ServerConfig,
	ServerCoreType,
	VersionPlan,
} from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import {
	loaderName,
	PlanReview,
	planSide,
	planSummary,
} from "@/components/version-change/plan-review"
import { formatPercent } from "@/lib/format"
import { formatBytes } from "@/lib/minecraft"
import { canShareFiles, shareFile } from "@/lib/share"
import { useServerStatus } from "@/services/server-data"
import { rpc, serverService } from "@/services/server-service"
import { Card, CardHeader, ErrorNote } from "../shared/primitives"

type Busy = "export-configs" | "export-full" | "copy" | "version" | "undo" | null

/** The last version change, while it can still be undone */
function useVersionBackup(serverId: string) {
	return useQuery({
		queryKey: ["server-version-backup", serverId],
		queryFn: () => rpc.get_server_version_backup(serverId),
	})
}

/** Export, copy and change version */
export function TransferCard({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
	const { status, isRunning } = useServerStatus(server.id)
	const stopped = status === "stopped"
	const [busy, setBusy] = useState<Busy>(null)
	const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null)
	const [versionOpen, setVersionOpen] = useState(false)
	const queryClient = useQueryClient()
	const { data: versionBackup } = useVersionBackup(server.id)
	const refreshBackup = () =>
		queryClient.invalidateQueries({ queryKey: ["server-version-backup", server.id] })

	const run = async (kind: Busy, task: () => Promise<string | null>) => {
		setBusy(kind)
		setNotice(null)
		try {
			const text = await task()
			if (text) setNotice({ ok: true, text })
		} catch (e) {
			setNotice({ ok: false, text: String(e) })
		} finally {
			setBusy(null)
		}
	}

	const exportServer = (mode: ExportMode) =>
		run(mode === "full" ? "export-full" : "export-configs", async () => {
			// Make sure the world on disk is current before copying it
			if (mode === "full" && isRunning) await rpc.save_server_world(server.id, true).catch(() => {})
			if (canShareFiles()) {
				const path = await rpc.export_server(server.id, mode, null)
				shareFile(path, "application/zip", server.name)
				return null
			}
			const suffix = mode === "full" ? "full" : "configs"
			const dest = await saveDialog({
				defaultPath: `${server.name.replace(/[^\w-]+/g, "_")}-${server.gameVersion}-${suffix}.zip`,
				filters: [{ name: t("transfer.zipArchive"), extensions: ["zip"] }],
			})
			if (!dest) return null
			const path = await rpc.export_server(server.id, mode, dest)
			return t("transfer.savedTo", { path })
		})

	const undoVersion = () =>
		run("undo", async () => {
			const restored = await rpc.undo_server_version_change(server.id)
			await serverService.refreshServers()
			await refreshBackup()
			return t("crashDialog.undone", { version: restored.gameVersion })
		})

	const discardBackup = () =>
		run("undo", async () => {
			await rpc.discard_server_version_backup(server.id)
			await refreshBackup()
			return null
		})

	const copy = () =>
		run("copy", async () => {
			const created = await rpc.duplicate_server(
				server.id,
				t("transfer.copyName", { name: server.name }),
				null,
				null,
			)
			await serverService.refreshServers()
			return t("transfer.created", { name: created.name })
		})

	return (
		<Card>
			<CardHeader
				icon={FileArchive}
				title={t("transfer.title")}
				description={t("transfer.description")}
			/>
			<div className="flex flex-col gap-2 px-4 pb-4">
				<div className="grid grid-cols-2 gap-2">
					<ActionButton
						icon={canShareFiles() ? Share2 : Package}
						label={t("transfer.exportConfigs")}
						hint={t("transfer.withoutWorlds")}
						busy={busy === "export-configs"}
						disabled={busy !== null}
						onClick={() => exportServer("configs")}
					/>
					<ActionButton
						icon={canShareFiles() ? Share2 : FileArchive}
						label={t("transfer.exportAll")}
						hint={t("transfer.withWorlds")}
						busy={busy === "export-full"}
						disabled={busy !== null}
						onClick={() => exportServer("full")}
					/>
					<ActionButton
						icon={Copy}
						label={t("transfer.makeCopy")}
						hint={stopped ? t("transfer.secondServer") : t("transfer.stopFirst")}
						busy={busy === "copy"}
						disabled={busy !== null || !stopped}
						onClick={copy}
					/>
					<ActionButton
						icon={ArrowUpDown}
						label={t("versionChange.title")}
						hint={
							stopped
								? t("transfer.nowVersion", { version: server.gameVersion })
								: t("transfer.stopFirst")
						}
						busy={busy === "version"}
						disabled={busy !== null || !stopped}
						onClick={() => setVersionOpen(true)}
					/>
				</div>
				{versionBackup && (
					<div className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-zinc-800 bg-zinc-950/60 p-3">
						<p className="min-w-0 flex-1 text-xs text-zinc-400">
							{t(versionBackup.worldsBackedUp ? "transfer.backupWithWorlds" : "transfer.backup", {
								version:
									versionBackup.fromLoader === versionBackup.toLoader
										? versionBackup.fromGameVersion
										: `${loaderName(versionBackup.fromLoader)} ${versionBackup.fromGameVersion}`,
								size: formatBytes(versionBackup.bytes),
							})}
						</p>
						<div className="flex gap-1.5">
							<Button
								variant="ghost"
								size="sm"
								onClick={discardBackup}
								disabled={busy !== null}
								className="h-8 rounded-lg text-xs text-zinc-500"
							>
								{t("transfer.deleteBackup")}
							</Button>
							<Button
								size="sm"
								onClick={undoVersion}
								disabled={busy !== null || !stopped}
								title={stopped ? undefined : t("transfer.stopFirst")}
								className="h-8 gap-1.5 rounded-lg text-xs"
							>
								{busy === "undo" ? (
									<Loader2 className="size-3.5 animate-spin" />
								) : (
									<Undo2 className="size-3.5" />
								)}
								{t("transfer.undo", { version: versionBackup.fromGameVersion })}
							</Button>
						</div>
					</div>
				)}
				{notice &&
					(notice.ok ? (
						<p className="break-all rounded-xl border border-emerald-500/20 bg-emerald-500/10 px-3 py-2.5 text-emerald-200 text-xs">
							{notice.text}
						</p>
					) : (
						<ErrorNote>{notice.text}</ErrorNote>
					))}
			</div>
			{/* Mounted only while open, so every opening starts from the version picker */}
			{versionOpen && (
				<ChangeVersionDialog
					server={server}
					open={versionOpen}
					onOpenChange={setVersionOpen}
					onDone={(text) => {
						setNotice({ ok: true, text })
						refreshBackup()
					}}
				/>
			)}
		</Card>
	)
}

function ActionButton({
	icon: Icon,
	label,
	hint,
	busy,
	disabled,
	onClick,
}: {
	icon: typeof Copy
	label: string
	hint: string
	busy: boolean
	disabled: boolean
	onClick: () => void
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			disabled={disabled}
			className="flex min-w-0 items-start gap-2.5 rounded-xl border border-zinc-800 bg-zinc-950/60 p-3 text-left transition-colors hover:bg-zinc-900 disabled:opacity-50"
		>
			{busy ? (
				<Loader2 className="mt-0.5 size-4 shrink-0 animate-spin text-zinc-300" />
			) : (
				<Icon className="mt-0.5 size-4 shrink-0 text-zinc-400" />
			)}
			<span className="min-w-0">
				<span className="block truncate font-medium text-sm text-zinc-100">{label}</span>
				<span className="block truncate text-[11px] text-zinc-500">{hint}</span>
			</span>
		</button>
	)
}

/** Cores a server can switch to without starting over (the checks happen in Rust too) */
const CORE_FAMILIES: ServerCoreType[][] = [
	["paper", "purpur", "folia"],
	["fabric", "quilt", "neoforge", "forge"],
]

type Stage = "pick" | "checking" | "review" | "applying" | "done"

function ChangeVersionDialog({
	server,
	open,
	onOpenChange,
	onDone,
}: {
	server: ServerConfig
	open: boolean
	onOpenChange: (open: boolean) => void
	onDone: (text: string) => void
}) {
	const { t } = useTranslation()
	const cores = CORE_FAMILIES.find((family) => family.includes(server.core)) ?? [server.core]
	const [core, setCore] = useState<ServerCoreType>(server.core)
	const { data: versions = [], isLoading } = useQuery({
		queryKey: ["core-versions", core],
		queryFn: () => serverService.getAvailableServerCoreVersions(core),
		enabled: open,
		staleTime: 10 * 60_000,
	})
	const [version, setVersion] = useState<string | null>(null)
	const [stage, setStage] = useState<Stage>("pick")
	const [plan, setPlan] = useState<VersionPlan | null>(null)
	const [choices, setChoices] = useState<ItemAction[]>([])
	const [backupWorlds, setBackupWorlds] = useState(true)
	const [downgradeOk, setDowngradeOk] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const target =
		(version && versions.includes(version) ? version : null) ??
		versions.find((v) => v !== server.gameVersion || core !== server.core) ??
		null
	const unchanged = target === server.gameVersion && core === server.core
	const busy = stage === "checking" || stage === "applying"

	const check = async () => {
		if (!target) return
		setError(null)
		setStage("checking")
		try {
			const result = await rpc.check_server_version_change(server.id, target, core)
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
			await rpc.apply_server_version_change(
				{ ...plan, items: plan.items.map((item, i) => ({ ...item, action: choices[i] })) },
				backupWorlds,
			)
			await serverService.refreshServers()
			setStage("done")
			onDone(t("transfer.willStartOn", { name: server.name, version: planSide(plan, "to") }))
		} catch (e) {
			setError(String(e))
			setStage("review")
		}
	}

	const options = versions.map((v) => ({
		value: v,
		label:
			v === server.gameVersion && core === server.core
				? t("transfer.currentVersion", { version: v })
				: v,
	}))

	return (
		<Dialog open={open} onOpenChange={(o) => !busy && onOpenChange(o)}>
			<DialogContent className="flex max-h-[90vh] flex-col gap-4 p-5 sm:max-w-2xl">
				<DialogTitle className="text-base">{t("transfer.changeTitle")}</DialogTitle>

				{(stage === "pick" || stage === "checking") && (
					<>
						<p className="text-sm text-zinc-400 leading-relaxed">
							{t("transfer.changeDescription", {
								context: cores.includes("paper") ? "plugin" : "mod",
							})}
						</p>
						{cores.length > 1 && (
							<div className="grid gap-1.5">
								<span className="font-medium text-xs text-zinc-400">
									{t("transfer.serverType")}
								</span>
								<div className="flex flex-wrap gap-1.5">
									{cores.map((id) => (
										<button
											key={id}
											type="button"
											disabled={busy}
											onClick={() => {
												setCore(id)
												setVersion(null)
											}}
											className={
												core === id
													? "rounded-lg border border-emerald-500/60 bg-emerald-500/10 px-3 py-1.5 text-sm text-zinc-50"
													: "rounded-lg border border-zinc-800 bg-zinc-950/60 px-3 py-1.5 text-sm text-zinc-400 hover:text-zinc-200"
											}
										>
											{loaderName(id)}
											{id === server.core && (
												<span className="ml-1 text-[10px] text-zinc-500">
													{t("versionChange.now")}
												</span>
											)}
										</button>
									))}
								</div>
							</div>
						)}
						{isLoading ? (
							<div className="flex justify-center py-4">
								<Loader2 className="size-5 animate-spin text-zinc-500" />
							</div>
						) : (
							<Select items={options} value={target} onValueChange={(v) => v && setVersion(v)}>
								<SelectTrigger className="h-11 rounded-xl border-zinc-800 bg-zinc-950/60 text-sm">
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									{options.map((o) => (
										<SelectItem key={o.value} value={o.value}>
											{o.label}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						)}
					</>
				)}

				{stage === "review" && plan && (
					<div className="-mr-2 min-h-0 flex-1 overflow-y-auto pr-2">
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
							emptyText={t("transfer.empty")}
							rounded
						/>
					</div>
				)}

				{stage === "applying" && (
					<div className="flex flex-col items-center gap-3 py-8 text-center">
						<Loader2 className="size-7 animate-spin text-emerald-400" />
						<p className="font-medium text-sm text-zinc-200">{t("versionChange.applying")}</p>
						<p className="max-w-sm text-xs text-zinc-500 leading-relaxed">
							{t("versionChange.applyingHint")}
						</p>
					</div>
				)}

				{stage === "done" && plan && (
					<div className="flex flex-col items-center gap-3 py-6 text-center">
						<CheckCircle2 className="size-8 text-emerald-400" />
						<p className="font-semibold text-sm text-zinc-50">
							{t("transfer.readyFor", { version: planSide(plan, "to") })}
						</p>
						<p className="max-w-sm text-xs text-zinc-400 leading-relaxed">
							{t("transfer.doneNote")}
						</p>
					</div>
				)}

				{error && <ErrorNote>{error}</ErrorNote>}

				<div className="flex items-center justify-between gap-2">
					<p className="text-[11px] text-zinc-500">
						{stage === "review" && plan && planSummary(plan, choices)}
					</p>
					<div className="flex gap-2">
						{stage === "review" && (
							<Button variant="ghost" onClick={() => setStage("pick")} className="rounded-xl">
								{t("versionChange.back")}
							</Button>
						)}
						{(stage === "pick" || stage === "checking") && (
							<Button
								onClick={check}
								disabled={!target || unchanged || busy}
								className="h-11 gap-2 rounded-xl bg-emerald-600 font-semibold text-white hover:bg-emerald-500"
							>
								{busy ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<ArrowUpDown className="size-4" />
								)}
								{busy ? t("versionChange.checking") : t("versionChange.check")}
							</Button>
						)}
						{stage === "review" && plan && (
							<Button
								onClick={apply}
								disabled={plan.downgrade && !downgradeOk}
								className="h-11 rounded-xl bg-emerald-600 font-semibold text-white hover:bg-emerald-500"
							>
								{t("versionChange.moveToVersion", { version: planSide(plan, "to") })}
							</Button>
						)}
						{stage === "done" && (
							<Button onClick={() => onOpenChange(false)} className="h-11 rounded-xl">
								{t("common.done")}
							</Button>
						)}
					</div>
				</div>
			</DialogContent>
		</Dialog>
	)
}

/** Creates a server from an exported .zip. Uploads in chunks so it works on phones too. */
export function ImportServerButton({
	onImported,
	className,
}: {
	onImported?: (server: ServerConfig) => void
	className?: string
}) {
	const { t } = useTranslation()
	const inputRef = useRef<HTMLInputElement>(null)
	const [progress, setProgress] = useState<number | null>(null)
	const [error, setError] = useState<string | null>(null)
	/** Imported, but some plugins/mods couldn't be downloaded: shown before moving on */
	const [partial, setPartial] = useState<ImportedServer | null>(null)

	const upload = async (file: File) => {
		const chunkSize = 2 * 1024 * 1024
		const uploadId = crypto.randomUUID()
		setError(null)
		setPartial(null)
		setProgress(0)
		try {
			for (let offset = 0; offset < file.size || offset === 0; offset += chunkSize) {
				const bytes = new Uint8Array(await file.slice(offset, offset + chunkSize).arrayBuffer())
				let binary = ""
				for (let i = 0; i < bytes.length; i += 0x8000) {
					binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
				}
				await rpc.import_upload_chunk(uploadId, btoa(binary), offset === 0)
				setProgress(Math.min(1, (offset + chunkSize) / Math.max(1, file.size)))
				if (file.size === 0) break
			}
			const imported = await rpc.import_server(uploadId)
			await serverService.refreshServers()
			if (imported.warnings.length > 0) setPartial(imported)
			else onImported?.(imported.server)
		} catch (e) {
			setError(String(e))
		} finally {
			setProgress(null)
			if (inputRef.current) inputRef.current.value = ""
		}
	}

	return (
		<>
			<input
				ref={inputRef}
				type="file"
				accept=".zip,application/zip"
				className="hidden"
				onChange={(e) => {
					const file = e.target.files?.[0]
					if (file) upload(file)
				}}
			/>
			<Button
				variant="outline"
				onClick={() => inputRef.current?.click()}
				disabled={progress !== null}
				className={className}
			>
				{progress !== null ? (
					<Loader2 className="size-4 animate-spin" />
				) : (
					<Upload className="size-4" />
				)}
				{progress === null
					? t("transfer.import")
					: progress < 1
						? t("transfer.importing", { percent: formatPercent(progress * 100) })
						: t("transfer.settingUp")}
			</Button>
			{error && <ErrorNote>{error}</ErrorNote>}
			{partial && (
				<div className="flex flex-col gap-2 rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2.5 text-amber-100 text-xs">
					<p className="font-semibold">{t("transfer.partial", { name: partial.server.name })}</p>
					<ul className="list-disc pl-4 text-amber-200/80">
						{partial.warnings.map((w) => (
							<li key={w}>{w}</li>
						))}
					</ul>
					<Button
						size="sm"
						onClick={() => {
							setPartial(null)
							onImported?.(partial.server)
						}}
						className="self-start rounded-lg bg-amber-500/20 text-amber-50 hover:bg-amber-500/30"
					>
						{t("transfer.openServer")}
					</Button>
				</div>
			)}
		</>
	)
}
