import { useQueryClient } from "@tanstack/react-query"
import {
	AlertTriangle,
	Check,
	ChevronRight,
	Cpu,
	FileCode2,
	FolderOpen,
	ImagePlus,
	Save,
	Search,
	SlidersHorizontal,
	Trash2,
} from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import type { PropertyEntry, ServerConfig } from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"
import { Item, ItemActions, ItemContent, ItemTitle } from "@/components/ui/item"
import { Label } from "@/components/ui/label"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { Textarea } from "@/components/ui/textarea"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { formatBytes, formatMegabytes } from "@/lib/format"
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
import { useMemorySettings } from "@/services/settings-service"
import DeleteServerDialog from "../delete-server-dialog"
import { MC_COLORS, ServerListPreview } from "../shared/motd"
import { Card, CardContent, CardHeader, ErrorNote, useSticky } from "../shared/primitives"
import {
	HANDLED_ELSEWHERE,
	KNOWN_KEYS,
	localizedPropertyGroups,
	type PropertyDef,
} from "./property-schema"
import { TransferCard } from "./transfer-card"

export function SettingsPanel({
	server,
	onDeleted,
}: {
	server: ServerConfig
	onDeleted?: () => void
}) {
	const { t } = useTranslation()
	const { isRunning } = useServerStatus(server.id)
	return (
		<div className="flex flex-col gap-4">
			{isRunning && (
				<Alert className={alertTone.warning}>
					<AlertTriangle />
					<AlertDescription>{t("serverSettings.restartNote")}</AlertDescription>
				</Alert>
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
	const { t } = useTranslation()
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
			setError(t("serverSettings.badImage", { error: String(e) }))
		} finally {
			setIconBusy(false)
			if (fileRef.current) fileRef.current.value = ""
		}
	}

	const onSave = async () => {
		if (!name.trim()) return setError(t("serverSettings.emptyName"))
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
				title={t("serverSettings.identity")}
				description={t("serverSettings.identityDesc")}
			/>
			<CardContent className="flex flex-col gap-4">
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
						className="group relative flex size-16 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-input border-dashed bg-card transition-colors hover:border-primary/60"
					>
						{icon ? (
							<img
								src={icon}
								alt=""
								className="size-full object-cover [image-rendering:pixelated]"
							/>
						) : (
							<ImagePlus className="size-5 text-muted-foreground" />
						)}
						<span
							className={cn(
								"absolute inset-0 flex items-center justify-center bg-black/60 opacity-0 transition-opacity group-hover:opacity-100",
								iconBusy && "opacity-100",
							)}
						>
							{iconBusy ? (
								<Spinner className="size-4 text-white" />
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
						<Label htmlFor={`name-${server.id}`} className="text-muted-foreground text-xs">
							{t("serverSettings.name")}
						</Label>
						<Input
							id={`name-${server.id}`}
							value={name}
							onChange={(e) => setName(e.target.value)}
							className="h-10 rounded-xl text-sm"
						/>
					</div>
				</div>

				<div className="flex flex-col gap-1.5">
					<Label htmlFor={`motd-${server.id}`} className="text-muted-foreground text-xs">
						{t("serverSettings.motd")}
					</Label>
					<Textarea
						id={`motd-${server.id}`}
						ref={motdRef}
						rows={2}
						value={currentMotd.split("\\n").join("\n")}
						onChange={(e) => setMotd(e.target.value.split("\n").slice(0, 2).join("\n"))}
						spellCheck={false}
						className="resize-none font-mono"
					/>
					<div className="flex flex-wrap gap-1">
						{MOTD_CODES.map(({ code, label, style }) => (
							<Tooltip key={code}>
								<TooltipTrigger
									render={
										<button
											type="button"
											onClick={() => insertCode(code)}
											className={cn(
												"flex h-7 items-center justify-center rounded-md border border-border text-2xs text-foreground/80 transition-transform hover:scale-110",
												label ? "min-w-7 bg-card px-1.5" : "w-7",
											)}
											style={label ? { ...style, background: undefined } : style}
											aria-label={`§${code}`}
										/>
									}
								>
									{code === "r" ? t("serverSettings.motdReset") : label}
								</TooltipTrigger>
								<TooltipContent>{`§${code}`}</TooltipContent>
							</Tooltip>
						))}
					</div>
				</div>

				{error && <ErrorNote>{error}</ErrorNote>}
				<SaveButton dirty={dirty} status={status} onClick={onSave} />
			</CardContent>
		</Card>
	)
}

function SaveButton({
	dirty,
	status,
	onClick,
	label,
}: {
	dirty: boolean
	status: "idle" | "saving" | "saved"
	onClick: () => void
	label?: string
}) {
	const { t } = useTranslation()
	// Nothing to save: keep the card clean
	if (!dirty && status === "idle") return null
	return (
		<Button
			onClick={onClick}
			disabled={!dirty || status === "saving"}
			className={cn(
				"h-10 gap-1.5 self-end rounded-xl px-4",
				status === "saved"
					? "bg-primary/15 text-primary"
					: "bg-primary text-primary-foreground hover:bg-primary",
			)}
		>
			{status === "saving" ? (
				<Spinner className="size-4" />
			) : status === "saved" ? (
				<Check className="size-4" />
			) : (
				<Save className="size-4" />
			)}
			{status === "saved" ? t("serverSettings.saved") : (label ?? t("serverSettings.saveChanges"))}
		</Button>
	)
}

// ─── Memory ───────────────────────────────────────────────────────────────────

function MemoryCard({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
	const [ram, setRam] = useState(server.memoryMaxMb)
	const [status, setStatus] = useState<"idle" | "saving" | "saved">("idle")
	const { systemMemory } = useMemorySettings()
	const mobile = isMobileEnvironment()
	const max = systemMemory?.totalMb
		? Math.max(512, Math.floor(systemMemory.totalMb / 512) * 512)
		: mobile
			? 8192
			: 16384
	useEffect(() => setRam(server.memoryMaxMb), [server.memoryMaxMb])

	if (server.core === "pumpkin" || server.core === "bedrock") return null

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
				title={t("serverSettings.memory")}
				description={mobile ? t("serverSettings.memoryMobile") : t("serverSettings.memoryDesktop")}
				action={
					<span className="font-mono font-semibold text-primary text-sm">
						{formatMegabytes(ram)}
					</span>
				}
			/>
			<CardContent className="flex flex-col gap-3">
				<Slider
					min={512}
					max={max}
					step={512}
					value={[Math.min(ram, max)]}
					onValueChange={(v) => setRam(Array.isArray(v) ? v[0] : (v as number))}
				/>
				<SaveButton dirty={ram !== server.memoryMaxMb} status={status} onClick={onSave} />
			</CardContent>
		</Card>
	)
}

// ─── server.properties ────────────────────────────────────────────────────────

function PropertiesCard({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
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
	const groups = localizedPropertyGroups(t)
		.map((g) => ({
			...g,
			properties: g.properties.filter((p) => !hasFile || values.has(p.key)),
		}))
		.filter((g) => g.properties.length > 0)

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
		if (portConflict)
			return setError(
				t("serverSettings.portInUse", {
					port: portValue,
					next: serverService.getNextAvailablePort(),
				}),
			)
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
				title={t("serverSettings.gameSettings")}
				description={t("serverSettings.gameSettingsDesc")}
			/>
			{isLoading ? (
				<div className="flex justify-center py-8">
					<Spinner className="size-4 text-muted-foreground" />
				</div>
			) : (
				<CardContent className="flex flex-col gap-5">
					{groups.map((group) => (
						<div key={group.id} className="flex flex-col">
							<h4 className="mb-1 font-semibold text-2xs text-muted-foreground uppercase tracking-wider">
								{group.title}
							</h4>
							<div className="divide-y divide-border/60">
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
						<Collapsible open={showAll} onOpenChange={setShowAll} className="flex flex-col gap-2">
							<CollapsibleTrigger
								render={
									<Button variant="ghost" size="sm" className="self-start text-muted-foreground" />
								}
							>
								<ChevronRight className={cn("transition-transform", showAll && "rotate-90")} />
								{t("serverSettings.allOther")}
							</CollapsibleTrigger>
							<CollapsibleContent className="flex flex-col gap-2">
								<InputGroup>
									<InputGroupAddon>
										<Search />
									</InputGroupAddon>
									<InputGroupInput
										value={query}
										onChange={(e) => setQuery(e.target.value)}
										placeholder={t("serverSettings.filter")}
										className="text-xs"
									/>
								</InputGroup>
								{others.map((e) => (
									<div
										key={e.key}
										className="flex flex-col gap-1 sm:flex-row sm:items-center sm:gap-3"
									>
										<span className="truncate font-mono text-2xs text-muted-foreground sm:w-64 sm:shrink-0">
											{e.key}
										</span>
										<Input
											value={draft[e.key] ?? e.value}
											onChange={(ev) => set(e.key, ev.target.value)}
											className={cn("h-8 font-mono text-xs", e.key in draft && "border-primary/40")}
										/>
									</div>
								))}
							</CollapsibleContent>
						</Collapsible>
					)}

					{error && <ErrorNote>{error}</ErrorNote>}
					{(changed.length > 0 || status !== "idle") && (
						// Solid bar across the card, in the card's own color, so settings scrolling
						// underneath never show through
						<div className="sticky bottom-0 -mx-4 -mb-4 flex items-center justify-end gap-3 border-border/80 border-t bg-card px-4 py-3">
							{changed.length > 0 && (
								<>
									<span className="text-muted-foreground text-xs">
										{t("serverSettings.changed", { count: changed.length })}
									</span>
									<Button variant="ghost" onClick={() => setDraft({})} className="h-10 rounded-xl">
										{t("serverSettings.discard")}
									</Button>
								</>
							)}
							<SaveButton dirty={changed.length > 0} status={status} onClick={onSave} />
						</div>
					)}
				</CardContent>
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
				<p className="flex items-center gap-1.5 font-medium text-foreground text-sm">
					{def.label}
					{changed && <span className="size-1.5 rounded-full bg-primary" />}
				</p>
				<p className="text-muted-foreground text-xs leading-relaxed">{def.description}</p>
			</div>
			<div className="shrink-0">
				{control.type === "boolean" && (
					<Switch checked={value === "true"} onCheckedChange={(c) => onChange(String(c))} />
				)}
				{control.type === "select" && (
					<Select items={control.options} value={value} onValueChange={(v) => v && onChange(v)}>
						<SelectTrigger className="h-9 w-36 rounded-xl text-xs">
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
							className="h-9 w-24 rounded-xl text-right font-mono text-xs"
						/>
						{control.unit && (
							<span className="w-10 text-2xs text-muted-foreground">{control.unit}</span>
						)}
					</div>
				)}
				{control.type === "text" && (
					<Input
						value={value}
						placeholder={control.placeholder}
						onChange={(e) => onChange(e.target.value)}
						className="h-9 w-40 rounded-xl font-mono text-xs"
					/>
				)}
			</div>
		</div>
	)
}

// ─── Other config files ───────────────────────────────────────────────────────

function ConfigFilesCard({ serverId }: { serverId: string }) {
	const { t } = useTranslation()
	const { data: files = [] } = useConfigFiles(serverId)
	const [open, setOpen] = useState<string | null>(null)
	if (files.length === 0) return null
	return (
		<Card className="pb-0">
			<CardHeader
				icon={FileCode2}
				title={t("serverSettings.configFiles")}
				description={t("serverSettings.configFilesDesc")}
			/>
			<ul className="divide-y divide-border/60 border-border/60 border-t">
				{files.map((f) => (
					<li key={f.path}>
						<Item
							render={<button type="button" onClick={() => setOpen(f.path)} />}
							className="rounded-none px-4 text-left hover:bg-muted/50"
						>
							<ItemContent>
								<ItemTitle className="font-mono text-xs">{f.path}</ItemTitle>
							</ItemContent>
							<ItemActions className="text-2xs text-muted-foreground">
								{formatBytes(f.size)}
								<ChevronRight className="size-3.5" />
							</ItemActions>
						</Item>
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
	const { t } = useTranslation()
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
						<Spinner className="size-5 text-muted-foreground" />
					</div>
				) : (
					<Textarea
						value={current}
						onChange={(e) => setText(e.target.value)}
						spellCheck={false}
						autoCapitalize="off"
						autoCorrect="off"
						className="field-sizing-fixed min-h-0 flex-1 resize-none font-mono text-xs leading-relaxed"
					/>
				)}
				{error && <ErrorNote>{error}</ErrorNote>}
				<div className="flex items-center justify-end gap-2">
					{text !== null && (
						<Button variant="ghost" onClick={() => setText(null)} className="h-10 rounded-xl">
							{t("serverSettings.discard")}
						</Button>
					)}
					<SaveButton
						dirty={text !== null && text !== data}
						status={status}
						onClick={onSave}
						label={t("serverSettings.saveFile")}
					/>
				</div>
			</DialogContent>
		</Dialog>
	)
}

// ─── Danger zone ──────────────────────────────────────────────────────────────

function DangerCard({ server, onDeleted }: { server: ServerConfig; onDeleted?: () => void }) {
	const { t } = useTranslation()
	const [deleting, setDeleting] = useState(false)
	const mobile = isMobileEnvironment()
	return (
		<Card>
			<CardHeader title={t("serverSettings.manage")} />
			<CardContent className="flex flex-wrap gap-2">
				{!mobile && (
					<Button
						variant="outline"
						onClick={() => serverService.openServerFolder(server.id)}
						className="h-10 gap-1.5 rounded-xl"
					>
						<FolderOpen className="size-4" />
						{t("common.openFolder")}
					</Button>
				)}
				<Button
					variant="destructive"
					onClick={() => setDeleting(true)}
					className="h-10 gap-1.5 rounded-xl"
				>
					<Trash2 className="size-4" />
					{t("deleteServer.title")}
				</Button>
			</CardContent>
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
