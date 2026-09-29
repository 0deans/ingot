import i18n from "i18next"
import { AlertCircle, ArrowLeft, Check, Cpu, Plus, Server } from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { ServerConfig, ServerCoreType } from "@/bindings"
import LoaderIcon from "@/components/instances/loader-icon"
import { FadeScroll } from "@/components/servers/shared/primitives"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
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
import { formatMegabytes } from "@/lib/format"
import { isMobileEnvironment } from "@/lib/platform"
import { cn } from "@/lib/utils"
import { serverService } from "@/services/server-service"
import { useMemorySettings } from "@/services/settings-service"

// ─── Data ──────────────────────────────────────────────────────────────────────

/** Taglines and badges are serverCores.<id>.tagline/badge in the locale files */
const SERVER_CORES: {
	id: ServerCoreType
	name: string
}[] = [
	{
		id: "paper",
		name: "Paper",
	},
	{
		id: "purpur",
		name: "Purpur",
	},
	{
		id: "fabric",
		name: "Fabric",
	},
	{
		id: "neoforge",
		name: "NeoForge",
	},
	{
		id: "forge",
		name: "Forge",
	},
	{
		id: "quilt",
		name: "Quilt",
	},
	{
		id: "folia",
		name: "Folia",
	},
	{
		id: "pumpkin",
		name: "PumpkinMC",
	},
	{
		id: "vanilla",
		name: "Vanilla",
	},
]

// ─── Props ─────────────────────────────────────────────────────────────────────

export interface NewServerWizardProps {
	/** Called when the user cancels / presses back past step 1 */
	onCancel: () => void
	/** Called after successful server creation */
	onServerCreated?: (server: ServerConfig) => void
	/** Pass an existing import handler — rendered on the first step as an alternative path */
	importSlot?: React.ReactNode
}

// ─── Step indicator ────────────────────────────────────────────────────────────

function Steps({ current, total }: { current: number; total: number }) {
	return (
		<div className="flex items-center gap-1.5">
			{Array.from({ length: total }, (_, i) => i).map((n) => (
				<span
					key={n}
					className={cn(
						"h-1.5 rounded-full transition-all duration-300",
						n === current
							? "w-5 bg-emerald-400"
							: n < current
								? "w-1.5 bg-emerald-600"
								: "w-1.5 bg-zinc-700",
					)}
				/>
			))}
		</div>
	)
}

// ─── Main wizard ───────────────────────────────────────────────────────────────

export function NewServerWizard({ onCancel, onServerCreated, importSlot }: NewServerWizardProps) {
	const { t } = useTranslation()
	const isMobile = isMobileEnvironment()
	const TOTAL_STEPS = 3

	const [step, setStep] = useState(0)

	// Step 0 state — choose core
	const [core, setCore] = useState<ServerCoreType>("paper")

	// Step 1 state — name + version + port
	const [name, setName] = useState("")
	const [versions, setVersions] = useState<string[]>([])
	const [selectedVersion, setSelectedVersion] = useState("")
	const [port, setPort] = useState(25565)
	const [isLoadingVersions, setIsLoadingVersions] = useState(false)

	// Step 2 state — RAM
	const [ramMb, setRamMb] = useState(isMobile ? 1024 : 4096)

	// Shared
	const [error, setError] = useState<string | null>(null)
	const [isSubmitting, setIsSubmitting] = useState(false)

	// Fetch available port once on mount
	useEffect(() => {
		const nextPort = serverService.getNextAvailablePort()
		setPort(nextPort)
	}, [])

	// Fetch versions whenever core changes
	useEffect(() => {
		let isMounted = true
		setIsLoadingVersions(true)
		setError(null)
		serverService
			.getAvailableServerCoreVersions(core)
			.then((list) => {
				if (!isMounted) return
				setVersions(list)
				const best = list.find((v) => /^\d+\.\d+(\.\d+)?$/.test(v)) || list[0] || ""
				setSelectedVersion(best)
			})
			.catch(() => {
				if (isMounted) setError(i18n.t("wizard.fetchFailed"))
			})
			.finally(() => {
				if (isMounted) setIsLoadingVersions(false)
			})
		return () => {
			isMounted = false
		}
	}, [core])

	const handleSelectCore = (newCore: ServerCoreType) => {
		setCore(newCore)
		const coreObj = SERVER_CORES.find((c) => c.id === newCore)
		if (coreObj && (!name || SERVER_CORES.some((c) => name.startsWith(c.name)))) {
			setName(t("wizard.defaultName", { core: coreObj.name }))
		}
	}

	const isPortConflict = serverService.isPortInUse(port)

	const handleCreate = async () => {
		if (!name.trim()) {
			setError(t("wizard.enterName"))
			return
		}
		if (!selectedVersion) {
			setError(t("wizard.selectVersion"))
			return
		}
		if (!port || port < 1024 || port > 65534) {
			setError(t("wizard.invalidPort"))
			return
		}
		if (isPortConflict) {
			setError(t("wizard.portInUseTry", { port, next: serverService.getNextAvailablePort() }))
			return
		}

		setIsSubmitting(true)
		setError(null)
		try {
			const created = await serverService.createServer(
				name.trim(),
				core,
				selectedVersion,
				null,
				port,
				Math.min(2048, ramMb),
				ramMb,
			)
			onServerCreated?.(created)
		} catch (err: unknown) {
			setError(err instanceof Error ? err.message : t("wizard.createFailed"))
		} finally {
			setIsSubmitting(false)
		}
	}

	const goBack = () => {
		setError(null)
		if (step === 0) onCancel()
		else setStep((s) => s - 1)
	}

	const goNext = () => {
		setError(null)
		if (step === 0) {
			// auto-set a name when advancing
			if (!name) {
				const coreObj = SERVER_CORES.find((c) => c.id === core)
				if (coreObj) setName(t("wizard.defaultName", { core: coreObj.name }))
			}
			setStep(1)
		} else if (step === 1) {
			if (!name.trim()) {
				setError(t("wizard.enterName"))
				return
			}
			if (!selectedVersion) {
				setError(t("wizard.selectVersion"))
				return
			}
			if (!port || port < 1024 || port > 65534) {
				setError(t("wizard.invalidPort"))
				return
			}
			if (isPortConflict) {
				setError(t("wizard.portInUse", { port }))
				return
			}
			setStep(2)
		}
	}

	return (
		<div className="flex h-full min-h-0 flex-col overflow-hidden bg-zinc-950 text-zinc-100">
			{/* ── Header ── */}
			<div className="flex shrink-0 items-center gap-3 px-4 pt-4 pb-3 sm:px-6 sm:pt-5">
				<button
					type="button"
					onClick={goBack}
					className="flex size-9 items-center justify-center rounded-xl text-zinc-400 transition-colors hover:bg-zinc-900 hover:text-zinc-100 active:bg-zinc-800"
				>
					<ArrowLeft className="size-5" />
				</button>
				<div className="flex flex-1 flex-col gap-0.5">
					<p className="font-semibold text-sm text-zinc-100">
						{step === 0
							? t("wizard.stepEngine")
							: step === 1
								? t("wizard.stepName")
								: t("wizard.stepMemory")}
					</p>
					<p className="text-[11px] text-zinc-500">
						{t("wizard.stepOf", { step: step + 1, total: TOTAL_STEPS })}
					</p>
				</div>
				<Steps current={step} total={TOTAL_STEPS} />
			</div>

			{/* ── Body (scrollable with fade) ── */}
			<FadeScroll className="min-h-0 flex-1 px-4 pb-4 sm:px-6">
				{step === 0 && <StepCore core={core} onSelect={handleSelectCore} />}
				{step === 1 && (
					<StepDetails
						name={name}
						onNameChange={setName}
						versions={versions}
						selectedVersion={selectedVersion}
						onVersionChange={setSelectedVersion}
						port={port}
						onPortChange={setPort}
						isPortConflict={isPortConflict}
						isLoadingVersions={isLoadingVersions}
						isMobile={isMobile}
						core={core}
					/>
				)}
				{step === 2 && (
					<StepRam
						core={core}
						isMobile={isMobile}
						ramMb={ramMb}
						onRamChange={setRamMb}
						name={name}
						version={selectedVersion}
					/>
				)}

				{error && (
					<Alert variant="destructive" className="mt-4">
						<AlertCircle />
						<AlertDescription>{error}</AlertDescription>
					</Alert>
				)}
			</FadeScroll>

			{/* ── Footer ── */}
			<div className="flex shrink-0 flex-col gap-2.5 border-zinc-900 border-t p-4 sm:px-6">
				{step === 0 && importSlot}
				{step < 2 ? (
					<Button
						onClick={goNext}
						className="h-12 w-full rounded-2xl bg-emerald-600 font-semibold text-sm text-white hover:bg-emerald-500 active:bg-emerald-700"
					>
						{t("common.continue")}
					</Button>
				) : (
					<Button
						onClick={handleCreate}
						disabled={isSubmitting || !name.trim() || !selectedVersion}
						className="h-12 w-full rounded-2xl bg-emerald-600 font-semibold text-sm text-white hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-50"
					>
						{isSubmitting ? (
							<>
								<Spinner className="mr-2 size-4" />
								{t("wizard.preparing")}
							</>
						) : (
							<>
								<Plus className="mr-2 size-4" />
								{t("servers.createServer")}
							</>
						)}
					</Button>
				)}
			</div>
		</div>
	)
}

// ─── Step 0: Pick core ─────────────────────────────────────────────────────────

function StepCore({
	core,
	onSelect,
}: {
	core: ServerCoreType
	onSelect: (c: ServerCoreType) => void
}) {
	const { t } = useTranslation()
	return (
		<div className="flex flex-col gap-2 pt-1">
			{SERVER_CORES.map((c) => {
				const isSelected = core === c.id
				return (
					<button
						key={c.id}
						type="button"
						onClick={() => onSelect(c.id)}
						className={cn(
							"flex items-center gap-3 rounded-2xl border p-3.5 text-left transition-all duration-150",
							isSelected
								? "border-emerald-500/60 bg-emerald-500/10 ring-1 ring-emerald-500/40"
								: "border-zinc-800 bg-zinc-900/50 active:bg-zinc-900",
						)}
					>
						<div className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900">
							<LoaderIcon loader={c.id} size={22} />
						</div>
						<div className="min-w-0 flex-1">
							<div className="flex items-center gap-2">
								<span className="font-semibold text-sm text-zinc-100">{c.name}</span>
								<Badge variant={isSelected ? "default" : "secondary"}>
									{t(`serverCores.${c.id}.badge`)}
								</Badge>
							</div>
							<p className="mt-0.5 text-[11px] text-zinc-500 leading-snug">
								{t(`serverCores.${c.id}.tagline`)}
							</p>
						</div>
						<div
							className={cn(
								"flex size-5 shrink-0 items-center justify-center rounded-full border transition-colors",
								isSelected ? "border-emerald-500 bg-emerald-500" : "border-zinc-700 bg-transparent",
							)}
						>
							{isSelected && <Check className="size-3 text-zinc-950" strokeWidth={3} />}
						</div>
					</button>
				)
			})}
		</div>
	)
}

// ─── Step 1: Name, version, port ───────────────────────────────────────────────

function StepDetails({
	name,
	onNameChange,
	versions,
	selectedVersion,
	onVersionChange,
	port,
	onPortChange,
	isPortConflict,
	isLoadingVersions,
	isMobile,
	core,
}: {
	name: string
	onNameChange: (v: string) => void
	versions: string[]
	selectedVersion: string
	onVersionChange: (v: string) => void
	port: number
	onPortChange: (v: number) => void
	isPortConflict: boolean
	isLoadingVersions: boolean
	isMobile: boolean
	core: ServerCoreType
}) {
	const { t } = useTranslation()
	const coreObj = SERVER_CORES.find((c) => c.id === core)
	return (
		<div className="flex flex-col gap-4 pt-2">
			{/* Core hint */}
			<div className="flex items-center gap-2.5 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-3">
				<div className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900">
					<LoaderIcon loader={core} size={20} />
				</div>
				<div>
					<p className="font-medium text-sm text-zinc-200">{coreObj?.name}</p>
					<p className="text-[11px] text-zinc-500">{t(`serverCores.${core}.badge`)}</p>
				</div>
			</div>

			{/* Server name */}
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="wiz-name" className="font-medium text-xs text-zinc-300">
					{t("wizard.serverName")}
				</Label>
				<Input
					id="wiz-name"
					placeholder={t("wizard.namePlaceholder", { core: coreObj?.name ?? "Paper" })}
					value={name}
					onChange={(e) => onNameChange(e.target.value)}
					className="h-11 rounded-xl border-zinc-800 bg-zinc-900 text-sm placeholder:text-zinc-600 focus-visible:ring-emerald-500/50"
					autoFocus
				/>
			</div>

			{/* Version */}
			<div className="flex flex-col gap-1.5">
				<span className="font-medium text-xs text-zinc-300">{t("wizard.minecraftVersion")}</span>
				{isLoadingVersions ? (
					<div className="flex h-11 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3 text-xs text-zinc-500">
						<Spinner className="size-3.5 text-emerald-400" />
						{t("newInstance.loadingVersions")}
					</div>
				) : (
					<Select value={selectedVersion} onValueChange={(val) => val && onVersionChange(val)}>
						<SelectTrigger className="h-11 rounded-xl border-zinc-800 bg-zinc-900 text-sm">
							<SelectValue placeholder={t("wizard.selectVersionPlaceholder")} />
						</SelectTrigger>
						<SelectContent className="max-h-64">
							{versions.map((ver) => (
								<SelectItem key={ver} value={ver} className="text-sm">
									{ver}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				)}
			</div>

			{/* Port — hidden on mobile by default (auto-assigned), collapsible */}
			{!isMobile && (
				<div className="flex flex-col gap-1.5">
					<div className="flex items-center justify-between">
						<Label htmlFor="wiz-port" className="font-medium text-xs text-zinc-300">
							{t("wizard.serverPort")}
						</Label>
						{isPortConflict && (
							<span className="font-medium text-[11px] text-rose-400">
								{t("wizard.portInUseShort")}
							</span>
						)}
					</div>
					<Input
						id="wiz-port"
						type="number"
						min={1024}
						max={65535}
						value={port}
						onChange={(e) => onPortChange(Number(e.target.value) || 25565)}
						className={cn(
							"h-11 rounded-xl border-zinc-800 bg-zinc-900 font-mono text-sm",
							isPortConflict && "border-rose-500 focus-visible:ring-rose-500/50",
						)}
					/>
				</div>
			)}
		</div>
	)
}

// ─── Step 2: RAM + summary ─────────────────────────────────────────────────────

function StepRam({
	core,
	isMobile,
	ramMb,
	onRamChange,
	name,
	version,
}: {
	core: ServerCoreType
	isMobile: boolean
	ramMb: number
	onRamChange: (v: number) => void
	name: string
	version: string
}) {
	const { t } = useTranslation()
	const { systemMemory } = useMemorySettings()
	const coreObj = SERVER_CORES.find((c) => c.id === core)
	const ramLabel = formatMegabytes(ramMb)
	const minRam = isMobile ? 512 : 1024
	const maxRam = systemMemory?.totalMb
		? Math.max(minRam, Math.floor(systemMemory.totalMb / 512) * 512)
		: isMobile
			? 4096
			: 16384

	return (
		<div className="flex flex-col gap-4 pt-2">
			{/* Summary card */}
			<div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4">
				<p className="mb-3 font-semibold text-sm text-zinc-200">{t("wizard.ready")}</p>
				<div className="grid grid-cols-2 gap-x-4 gap-y-2 text-xs">
					<div>
						<p className="text-zinc-500">{t("serverSettings.name")}</p>
						<p className="truncate font-medium text-zinc-200">{name}</p>
					</div>
					<div>
						<p className="text-zinc-500">{t("wizard.engine")}</p>
						<p className="font-medium text-zinc-200">{coreObj?.name}</p>
					</div>
					<div>
						<p className="text-zinc-500">{t("wizard.version")}</p>
						<p className="font-medium text-zinc-200">{version}</p>
					</div>
					<div>
						<p className="text-zinc-500">{t("wizard.ram")}</p>
						<p className="font-medium text-emerald-400">{ramLabel}</p>
					</div>
				</div>
			</div>

			{/* RAM slider */}
			<div className="flex flex-col gap-3 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4">
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-1.5">
						<Cpu className="size-3.5 text-emerald-400" />
						<span className="font-medium text-sm text-zinc-200">{t("wizard.memory")}</span>
					</div>
					<span className="font-bold font-mono text-emerald-400 text-sm">{ramLabel}</span>
				</div>
				<Slider
					min={minRam}
					max={maxRam}
					step={512}
					value={[Math.min(ramMb, maxRam)]}
					onValueChange={(val: number | readonly number[]) =>
						onRamChange(Array.isArray(val) ? val[0] : (val as number))
					}
					className="my-1"
				/>
			</div>
		</div>
	)
}

// ─── Empty state (no servers yet) ─────────────────────────────────────────────

interface EmptyStateProps {
	onCreate: () => void
	importSlot?: React.ReactNode
}

/** Full-screen onboarding shown when the user hasn't created any server yet */
export function NoServersState({ onCreate, importSlot }: EmptyStateProps) {
	const { t } = useTranslation()
	return (
		<div className="flex h-full min-h-0 flex-col items-center justify-between gap-6 bg-zinc-950 px-6 py-10 text-center sm:justify-center sm:gap-10">
			{/* Visual */}
			<div className="flex flex-col items-center gap-5">
				<div className="relative flex size-28 items-center justify-center">
					{/* Glow rings */}
					<div className="absolute size-28 rounded-full bg-emerald-500/10 blur-2xl" />
					<div className="absolute size-20 rounded-full bg-emerald-500/15 blur-xl" />
					{/* Icon */}
					<div className="relative flex size-20 items-center justify-center rounded-3xl border border-emerald-500/30 bg-emerald-500/10 shadow-[0_0_40px_rgba(16,185,129,0.15)]">
						<Server className="size-9 text-emerald-400" />
					</div>
				</div>

				<div>
					<h1 className="font-bold text-2xl text-zinc-50 leading-tight">
						{t("wizard.heroLine1")}
						<br />
						{t("wizard.heroLine2")}
					</h1>
					<p className="mt-2 max-w-xs text-sm text-zinc-400 leading-relaxed">
						{t("wizard.heroText")}
					</p>
				</div>
			</div>

			{/* Actions */}
			<div className="flex w-full max-w-sm flex-col gap-3">
				<button
					type="button"
					onClick={onCreate}
					className="group flex w-full items-center gap-4 rounded-2xl border border-emerald-500/30 bg-emerald-500/10 p-4 text-left transition-all hover:bg-emerald-500/15 active:bg-emerald-500/20"
				>
					<div className="flex size-11 shrink-0 items-center justify-center rounded-xl border border-emerald-500/30 bg-emerald-500/15 text-emerald-400">
						<Server className="size-5" />
					</div>
					<div>
						<p className="font-semibold text-sm text-zinc-100">{t("wizard.createAServer")}</p>
						<p className="text-[11px] text-zinc-500">{t("wizard.createAServerHint")}</p>
					</div>
					<Plus className="ml-auto size-5 shrink-0 text-emerald-400" />
				</button>

				{importSlot && (
					<div className="w-full [&>*]:w-full [&>button]:h-12 [&>button]:rounded-2xl [&>button]:border-zinc-800">
						{importSlot}
					</div>
				)}
			</div>
		</div>
	)
}
