import { useQueryClient } from "@tanstack/react-query"
import {
	AlertTriangle,
	Check,
	ChevronRight,
	Cpu,
	FileCode2,
	FolderOpen,
	ImagePlus,
	Loader2,
	Save,
	Search,
	SlidersHorizontal,
	Trash2,
} from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import type { PropertyEntry, ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import Slider from "@/components/ui/slider"
import { Switch } from "@/components/ui/switch"
import { isMobileEnvironment } from "@/lib/platform"
import { toServerIcon } from "@/lib/server-icon"
import { cn } from "@/lib/utils"
import {
	serverKeys,
	useConfigFile,
	useConfigFiles,
	useServerIcon,
	useServerPropertiesAll,
	useServerStatus,
} from "@/services/server-data"
import { rpc, serverService } from "@/services/server-service"
import DeleteServerDialog from "../delete-server-dialog"
import { MC_COLORS, ServerListPreview } from "../shared/motd"
import { Card, CardHeader, ErrorNote, useSticky } from "../shared/primitives"
import { HANDLED_ELSEWHERE, KNOWN_KEYS, PROPERTY_GROUPS, type PropertyDef } from "./property-schema"
import { TransferCard } from "./transfer-card"

export function SettingsPanel({
	server,
	onDeleted,
}: {
	server: ServerConfig
	onDeleted?: () => void
}) {
	const { isRunning } = useServerStatus(server.id)
	return (
		<div className="flex flex-col gap-4">
			{isRunning && (
				<div className="flex items-start gap-2.5 rounded-2xl border border-amber-500/20 bg-amber-500/10 px-4 py-3 text-amber-200 text-xs">
					<AlertTriangle className="mt-0.5 size-4 shrink-0 text-amber-400" />
					Most changes apply after the server restarts.
				</div>
			)}
			<IdentityCard server={server} isRunning={isRunning} />
			<MemoryCard server={server} />
			<PropertiesCard server={server} />
			<ConfigFilesCard serverId={server.id} />
			<TransferCard server={server} />
			<DangerCard server={server} onDeleted={onDeleted} />
		</div>
	)
}

// ─── Name, icon, MOTD ─────────────────────────────────────────────────────────

const MOTD_CODES: { code: string; label: string; style?: React.CSSProperties }[] = [
	...Object.entries(MC_COLORS).map(([code, color]) => ({
		code,
		label: "",
		style: { background: color },
	})),
	{ code: "l", label: "B", style: { fontWeight: 700 } },
	{ code: "o", label: "I", style: { fontStyle: "italic" } },
	{ code: "n", label: "U", style: { textDecoration: "underline" } },
	{ code: "r", label: "Reset" },
]

function IdentityCard({ server, isRunning }: { server: ServerConfig; isRunning: boolean }) {
	const queryClient = useQueryClient()
	const { data: icon } = useServerIcon(server.id)
	const { values, save } = useServerPropertiesAll(server.id)
	const [name, setName] = useState(server.name)
	const [motd, setMotd] = useState<string | null>(null)
	const [status, setStatus] = useState<"idle" | "saving" | "saved">("idle")
	const [error, setError] = useState<string | null>(null)
	const [iconBusy, setIconBusy] = useState(false)
	const fileRef = useRef<HTMLInputElement>(null)
	const motdRef = useRef<HTMLTextAreaElement>(null)

	useEffect(() => setName(server.name), [server.name])
	const savedMotd = values.get("motd") ?? ""
	const currentMotd = motd ?? savedMotd
	const dirty = name.trim() !== server.name || (motd !== null && motd !== savedMotd)

	const insertCode = (code: string) => {
		const el = motdRef.current
		const text = `§${code}`
		const start = el?.selectionStart ?? currentMotd.length
		const end = el?.selectionEnd ?? currentMotd.length
		setMotd(currentMotd.slice(0, start) + text + currentMotd.slice(end))
		requestAnimationFrame(() => {
			el?.focus()
			el?.setSelectionRange(start + text.length, start + text.length)
		})
	}

	const onIcon = async (file: File | undefined) => {
		if (!file) return
		setIconBusy(true)
		setError(null)
		try {
			await serverService.setServerIcon(server.id, await toServerIcon(file))
			await queryClient.invalidateQueries({ queryKey: serverKeys.icon(server.id) })
		} catch (e) {
			setError(`Couldn't use that image: ${e}`)
		} finally {
			setIconBusy(false)
			if (fileRef.current) fileRef.current.value = ""
		}
	}

	const onSave = async () => {
		if (!name.trim()) return setError("The name can't be empty.")
		setStatus("saving")
		setError(null)
		try {
			if (name.trim() !== server.name)
				await serverService.updateServer({ ...server, name: name.trim() })
			if (motd !== null && motd !== savedMotd)
				await save.mutateAsync([{ key: "motd", value: motd }])
			setMotd(null)
			setStatus("saved")
			setTimeout(() => setStatus("idle"), 1800)
		} catch (e) {
			setError(String(e))
			setStatus("idle")
		}
	}

	return (
		<Card>
			<CardHeader
				title="Server identity"
				description="How your server looks in the multiplayer list."
			/>
			<div className="flex flex-col gap-4 px-4 pb-4">
				<ServerListPreview
					motd={currentMotd}
					icon={icon}
					name={name || server.name}
					max={Number(values.get("max-players") ?? 20)}
					isRunning={isRunning}
				/>

				<div className="flex items-center gap-3">
					<button
						type="button"
						onClick={() => fileRef.current?.click()}
						disabled={iconBusy}
						className="group relative flex size-16 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-zinc-700 border-dashed bg-zinc-900 transition-colors hover:border-emerald-500/60"
					>
						{icon ? (
							<img
								src={icon}
								alt=""
								className="size-full object-cover [image-rendering:pixelated]"
							/>
						) : (
							<ImagePlus className="size-5 text-zinc-500" />
						)}
						<span className="absolute inset-0 flex items-center justify-center bg-black/60 opacity-0 transition-opacity group-hover:opacity-100">
							{iconBusy ? (
								<Loader2 className="size-4 animate-spin text-white" />
							) : (
								<ImagePlus className="size-4 text-white" />
							)}
						</span>
					</button>
					<input
						ref={fileRef}
						type="file"
						accept="image/*"
						className="hidden"
						onChange={(e) => onIcon(e.target.files?.[0])}
					/>
					<div className="flex min-w-0 flex-1 flex-col gap-1.5">
						<label htmlFor={`name-${server.id}`} className="text-xs text-zinc-400">
							Name
						</label>
						<Input
							id={`name-${server.id}`}
							value={name}
							onChange={(e) => setName(e.target.value)}
							className="h-10 rounded-xl border-zinc-800 bg-zinc-950/60 text-sm"
						/>
					</div>
				</div>

				<div className="flex flex-col gap-1.5">
					<label htmlFor={`motd-${server.id}`} className="text-xs text-zinc-400">
						Description (MOTD) · two lines, use Enter for the second
					</label>
					<textarea
						id={`motd-${server.id}`}
						ref={motdRef}
						rows={2}
						value={currentMotd.split("\\n").join("\n")}
						onChange={(e) => setMotd(e.target.value.split("\n").slice(0, 2).join("\n"))}
						spellCheck={false}
						className="resize-none rounded-xl border border-zinc-800 bg-zinc-950/60 px-3 py-2 font-mono text-sm text-zinc-100 outline-none focus:border-zinc-600"
					/>
					<div className="flex flex-wrap gap-1">
						{MOTD_CODES.map(({ code, label, style }) => (
							<button
								key={code}
								type="button"
								title={`§${code}`}
								onClick={() => insertCode(code)}
								className={cn(
									"flex h-7 items-center justify-center rounded-md border border-zinc-800 text-[11px] text-zinc-300 transition-transform hover:scale-110",
									label ? "min-w-7 bg-zinc-900 px-1.5" : "w-7",
								)}
								style={label ? { ...style, background: undefined } : style}
							>
								{label}
							</button>
						))}
					</div>
				</div>

				{error && <ErrorNote>{error}</ErrorNote>}
				<SaveButton dirty={dirty} status={status} onClick={onSave} />
			</div>
		</Card>
	)
}

function SaveButton({
	dirty,
	status,
	onClick,
	label = "Save changes",
}: {
	dirty: boolean
	status: "idle" | "saving" | "saved"
	onClick: () => void
	label?: string
}) {
	// Nothing to save: keep the card clean
	if (!dirty && status === "idle") return null
	return (
		<Button
			onClick={onClick}
			disabled={!dirty || status === "saving"}
			className={cn(
				"h-10 gap-1.5 self-end rounded-xl px-4",
				status === "saved"
					? "bg-emerald-500/15 text-emerald-300"
					: "bg-emerald-600 text-white hover:bg-emerald-500",
			)}
		>
			{status === "saving" ? (
				<Loader2 className="size-4 animate-spin" />
			) : status === "saved" ? (
				<Check className="size-4" />
			) : (
				<Save className="size-4" />
			)}
			{status === "saved" ? "Saved" : label}
		</Button>
	)
}

// ─── Memory ───────────────────────────────────────────────────────────────────

function MemoryCard({ server }: { server: ServerConfig }) {
	const [ram, setRam] = useState(server.memoryMaxMb)
	const [status, setStatus] = useState<"idle" | "saving" | "saved">("idle")
	const mobile = isMobileEnvironment()
	const max = mobile ? 8192 : 16384
	useEffect(() => setRam(server.memoryMaxMb), [server.memoryMaxMb])

	if (server.core === "pumpkin") return null

	const onSave = async () => {
		setStatus("saving")
		await serverService.updateServer({
			...server,
			memoryMaxMb: ram,
			memoryMinMb: Math.min(server.memoryMinMb, Math.floor(ram / 2)),
		})
		setStatus("saved")
		setTimeout(() => setStatus("idle"), 1800)
	}

	return (
		<Card>
			<CardHeader
				icon={Cpu}
				title="Memory"
				description={
					mobile ? "Keep it well below your phone's RAM." : "Maximum RAM for the Java server."
				}
				action={
					<span className="font-mono font-semibold text-emerald-300 text-sm">
						{(ram / 1024).toFixed(1)} GB
					</span>
				}
			/>
			<div className="flex flex-col gap-3 px-4 pb-4">
				<Slider
					min={512}
					max={max}
					step={512}
					value={[Math.min(ram, max)]}
					onValueChange={(v) => setRam(Array.isArray(v) ? v[0] : (v as number))}
				/>
				<SaveButton dirty={ram !== server.memoryMaxMb} status={status} onClick={onSave} />
			</div>
		</Card>
	)
}

// ─── server.properties ────────────────────────────────────────────────────────

function PropertiesCard({ server }: { server: ServerConfig }) {
	const { data, values, save, isLoading } = useServerPropertiesAll(server.id)
	const [draft, setDraft] = useState<Record<string, string>>({})
	const [status, setStatus] = useState<"idle" | "saving" | "saved">("idle")
	const [error, setError] = useState<string | null>(null)
	const [showAll, setShowAll] = useState(false)
	const [query, setQuery] = useState("")

	const hasFile = (data?.length ?? 0) > 0
	const value = (def: PropertyDef) => draft[def.key] ?? values.get(def.key) ?? def.defaultValue
	const set = (key: string, v: string) => setDraft((d) => ({ ...d, [key]: v }))
	const changed = Object.entries(draft).filter(([k, v]) => v !== (values.get(k) ?? undefined))

	// Only show schema entries this server version actually has (or all, before first start)
	const groups = PROPERTY_GROUPS.map((g) => ({
		...g,
		properties: g.properties.filter((p) => !hasFile || values.has(p.key)),
	})).filter((g) => g.properties.length > 0)

	const others = useMemo(
		() =>
			(data ?? []).filter(
				(e) =>
					!KNOWN_KEYS.has(e.key) &&
					!HANDLED_ELSEWHERE.has(e.key) &&
					e.key.toLowerCase().includes(query.trim().toLowerCase()),
			),
		[data, query],
	)

	const portValue = draft["server-port"]
	const portConflict = portValue ? serverService.isPortInUse(Number(portValue), server.id) : false

	const onSave = async () => {
		if (portConflict) return setError(`Port ${portValue} is used by another server.`)
		setStatus("saving")
		setError(null)
		try {
			const entries: PropertyEntry[] = changed.map(([key, v]) => ({ key, value: v }))
			await save.mutateAsync(entries)
			if (draft["server-port"]) await serverService.refreshServers()
			setDraft({})
			setStatus("saved")
			setTimeout(() => setStatus("idle"), 1800)
		} catch (e) {
			setError(String(e))
			setStatus("idle")
		}
	}

	return (
		<Card>
			<CardHeader
				icon={SlidersHorizontal}
				title="Game settings"
				description="server.properties, with explanations."
			/>
			{isLoading ? (
				<div className="flex justify-center py-8">
					<Loader2 className="size-4 animate-spin text-zinc-500" />
				</div>
			) : (
				<div className="flex flex-col gap-5 px-4 pb-4">
					{groups.map((group) => (
						<div key={group.id} className="flex flex-col">
							<h4 className="mb-1 font-semibold text-[11px] text-zinc-500 uppercase tracking-wider">
								{group.title}
							</h4>
							<div className="divide-y divide-zinc-800/60">
								{group.properties.map((def) => (
									<PropertyRow
										key={def.key}
										def={def}
										value={value(def)}
										changed={def.key in draft}
										onChange={(v) => set(def.key, v)}
									/>
								))}
							</div>
						</div>
					))}

					{(data?.length ?? 0) > 0 && (
						<div className="flex flex-col gap-2">
							<button
								type="button"
								onClick={() => setShowAll((s) => !s)}
								className="flex items-center gap-1.5 self-start font-medium text-xs text-zinc-400 hover:text-zinc-200"
							>
								<ChevronRight
									className={cn("size-3.5 transition-transform", showAll && "rotate-90")}
								/>
								All other properties
							</button>
							{showAll && (
								<div className="flex flex-col gap-2">
									<div className="relative">
										<Search className="absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-zinc-500" />
										<Input
											value={query}
											onChange={(e) => setQuery(e.target.value)}
											placeholder="Filter properties"
											className="h-9 rounded-xl border-zinc-800 bg-zinc-950/60 pl-9 text-xs"
										/>
									</div>
									{others.map((e) => (
										<div
											key={e.key}
											className="flex flex-col gap-1 sm:flex-row sm:items-center sm:gap-3"
										>
											<span className="truncate font-mono text-[11px] text-zinc-400 sm:w-64 sm:shrink-0">
												{e.key}
											</span>
											<Input
												value={draft[e.key] ?? e.value}
												onChange={(ev) => set(e.key, ev.target.value)}
												className={cn(
													"h-8 rounded-lg border-zinc-800 bg-zinc-950/60 font-mono text-xs",
													e.key in draft && "border-emerald-500/40",
												)}
											/>
										</div>
									))}
								</div>
							)}
						</div>
					)}

					{error && <ErrorNote>{error}</ErrorNote>}
					{(changed.length > 0 || status !== "idle") && (
						<div className="sticky bottom-0 flex items-center justify-end gap-3 bg-gradient-to-t from-zinc-950 via-zinc-950/90 to-transparent pt-3">
							{changed.length > 0 && (
								<>
									<span className="text-xs text-zinc-500">{changed.length} changed</span>
									<Button variant="ghost" onClick={() => setDraft({})} className="h-10 rounded-xl">
										Discard
									</Button>
								</>
							)}
							<SaveButton dirty={changed.length > 0} status={status} onClick={onSave} />
						</div>
					)}
				</div>
			)}
		</Card>
	)
}

function PropertyRow({
	def,
	value,
	changed,
	onChange,
}: {
	def: PropertyDef
	value: string
	changed: boolean
	onChange: (value: string) => void
}) {
	const control = def.control
	return (
		<div className="flex items-center justify-between gap-4 py-3">
			<div className="min-w-0">
				<p className="flex items-center gap-1.5 font-medium text-sm text-zinc-200">
					{def.label}
					{changed && <span className="size-1.5 rounded-full bg-emerald-400" />}
				</p>
				<p className="text-xs text-zinc-500 leading-relaxed">{def.description}</p>
			</div>
			<div className="shrink-0">
				{control.type === "boolean" && (
					<Switch checked={value === "true"} onCheckedChange={(c) => onChange(String(c))} />
				)}
				{control.type === "select" && (
					<Select items={control.options} value={value} onValueChange={(v) => v && onChange(v)}>
						<SelectTrigger className="h-9 w-36 rounded-xl border-zinc-800 bg-zinc-950/60 text-xs">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{control.options.map((o) => (
								<SelectItem key={o.value} value={o.value}>
									{o.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				)}
				{control.type === "number" && (
					<div className="flex items-center gap-1.5">
						<Input
							type="number"
							inputMode="numeric"
							min={control.min}
							max={control.max}
							value={value}
							onChange={(e) => onChange(e.target.value)}
							className="h-9 w-24 rounded-xl border-zinc-800 bg-zinc-950/60 text-right font-mono text-xs"
						/>
						{control.unit && <span className="w-10 text-[11px] text-zinc-500">{control.unit}</span>}
					</div>
				)}
				{control.type === "text" && (
					<Input
						value={value}
						placeholder={control.placeholder}
						onChange={(e) => onChange(e.target.value)}
						className="h-9 w-40 rounded-xl border-zinc-800 bg-zinc-950/60 font-mono text-xs"
					/>
				)}
			</div>
		</div>
	)
}

// ─── Other config files ───────────────────────────────────────────────────────

function ConfigFilesCard({ serverId }: { serverId: string }) {
	const { data: files = [] } = useConfigFiles(serverId)
	const [open, setOpen] = useState<string | null>(null)
	if (files.length === 0) return null
	return (
		<Card>
			<CardHeader
				icon={FileCode2}
				title="Config files"
				description="Server and plugin configs (YAML, JSON, TOML)."
			/>
			<ul className="divide-y divide-zinc-800/60 border-zinc-800/60 border-t">
				{files.map((f) => (
					<li key={f.path}>
						<button
							type="button"
							onClick={() => setOpen(f.path)}
							className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left transition-colors hover:bg-zinc-900/60"
						>
							<span className="min-w-0 truncate font-mono text-xs text-zinc-300">{f.path}</span>
							<span className="flex shrink-0 items-center gap-2 text-[11px] text-zinc-600">
								{(f.size / 1024).toFixed(1)} KB
								<ChevronRight className="size-3.5" />
							</span>
						</button>
					</li>
				))}
			</ul>
			<ConfigFileEditor serverId={serverId} path={open} onClose={() => setOpen(null)} />
		</Card>
	)
}

function ConfigFileEditor({
	serverId,
	path,
	onClose,
}: {
	serverId: string
	path: string | null
	onClose: () => void
}) {
	const queryClient = useQueryClient()
	const shownPath = useSticky(path)
	const { data, isLoading } = useConfigFile(serverId, shownPath)
	const [text, setText] = useState<string | null>(null)
	const [status, setStatus] = useState<"idle" | "saving" | "saved">("idle")
	const [error, setError] = useState<string | null>(null)

	// Reset the draft whenever another file is opened
	// biome-ignore lint/correctness/useExhaustiveDependencies: path is the trigger, not an input
	useEffect(() => {
		setText(null)
		setError(null)
	}, [path])

	const current = text ?? data ?? ""
	const onSave = async () => {
		if (!path || text === null) return
		setStatus("saving")
		setError(null)
		try {
			await rpc.write_server_config_file(serverId, path, text)
			queryClient.setQueryData(serverKeys.configFile(serverId, path), text)
			setText(null)
			setStatus("saved")
			setTimeout(() => setStatus("idle"), 1800)
		} catch (e) {
			setError(String(e))
			setStatus("idle")
		}
	}

	return (
		<Dialog open={Boolean(path)} onOpenChange={(o) => !o && onClose()}>
			<DialogContent className="flex h-[85vh] flex-col gap-3 p-4 sm:max-w-3xl">
				<DialogTitle className="truncate pr-8 font-mono text-sm">{shownPath}</DialogTitle>
				{isLoading ? (
					<div className="flex flex-1 items-center justify-center">
						<Loader2 className="size-5 animate-spin text-zinc-500" />
					</div>
				) : (
					<textarea
						value={current}
						onChange={(e) => setText(e.target.value)}
						spellCheck={false}
						autoCapitalize="off"
						autoCorrect="off"
						className="min-h-0 flex-1 resize-none rounded-xl border border-zinc-800 bg-zinc-950 p-3 font-mono text-xs text-zinc-200 leading-relaxed outline-none focus:border-zinc-600"
					/>
				)}
				{error && <ErrorNote>{error}</ErrorNote>}
				<div className="flex items-center justify-end gap-2">
					{text !== null && (
						<Button variant="ghost" onClick={() => setText(null)} className="h-10 rounded-xl">
							Discard
						</Button>
					)}
					<SaveButton
						dirty={text !== null && text !== data}
						status={status}
						onClick={onSave}
						label="Save file"
					/>
				</div>
			</DialogContent>
		</Dialog>
	)
}

// ─── Danger zone ──────────────────────────────────────────────────────────────

function DangerCard({ server, onDeleted }: { server: ServerConfig; onDeleted?: () => void }) {
	const [deleting, setDeleting] = useState(false)
	const mobile = isMobileEnvironment()
	return (
		<Card>
			<CardHeader title="Manage" />
			<div className="flex flex-wrap gap-2 px-4 pb-4">
				{!mobile && (
					<Button
						variant="outline"
						onClick={() => serverService.openServerFolder(server.id)}
						className="h-10 gap-1.5 rounded-xl"
					>
						<FolderOpen className="size-4" />
						Open folder
					</Button>
				)}
				<Button
					variant="destructive"
					onClick={() => setDeleting(true)}
					className="h-10 gap-1.5 rounded-xl"
				>
					<Trash2 className="size-4" />
					Delete server
				</Button>
			</div>
			<DeleteServerDialog
				server={deleting ? server : null}
				open={deleting}
				onOpenChange={setDeleting}
				onConfirm={async (id, deleteFiles) => {
					await serverService.deleteServer(id, deleteFiles)
					setDeleting(false)
					onDeleted?.()
				}}
			/>
		</Card>
	)
}
