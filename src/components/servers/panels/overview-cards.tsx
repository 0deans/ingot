import i18n from "i18next"
import {
	Activity,
	Check,
	Copy,
	Cpu,
	ExternalLink,
	Gauge,
	Globe,
	HardDrive,
	MemoryStick,
	QrCode,
	Share2,
	Wifi,
} from "lucide-react"
import { type ReactNode, useRef, useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import type { ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { Spinner } from "@/components/ui/spinner"
import { formatDuration, formatMegabytes, formatNumber, formatPercent } from "@/lib/format"
import { formatBytes } from "@/lib/minecraft"
import { shareText } from "@/lib/share"
import { cn } from "@/lib/utils"
import { usePlayitTunnel } from "@/services/hosting"
import {
	useLanAddress,
	useServerStats,
	useServerStatus,
	useServerStorage,
} from "@/services/server-data"
import { Card, ErrorNote } from "../shared/primitives"

// ─── Join / share ─────────────────────────────────────────────────────────────

function CopyButton({ text, label }: { text: string; label: string }) {
	const [copied, setCopied] = useState(false)
	return (
		<Button
			size="icon-sm"
			variant="ghost"
			aria-label={label}
			onClick={async () => {
				await navigator.clipboard.writeText(text)
				setCopied(true)
				setTimeout(() => setCopied(false), 1500)
			}}
		>
			{copied ? <Check className="size-3.5 text-primary" /> : <Copy className="size-3.5" />}
		</Button>
	)
}

function AddressRow({
	icon: Icon,
	label,
	children,
}: {
	icon: typeof Wifi
	label: string
	children: ReactNode
}) {
	return (
		<div className="flex min-w-0 items-center gap-3 rounded-xl bg-background/60 py-2 pr-2 pl-3">
			<Icon className="size-4 shrink-0 text-muted-foreground" />
			<div className="min-w-0 flex-1">
				<p className="text-3xs text-muted-foreground uppercase tracking-wider">{label}</p>
				{children}
			</div>
		</div>
	)
}

/** How friends can join: LAN and playit.gg addresses, plus a share button */
export function JoinCard({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
	const { tunnel, toggle, isBusy } = usePlayitTunnel()
	const { data: lanIp } = useLanAddress()
	const [qrOpen, setQrOpen] = useState(false)
	const [shared, setShared] = useState<"shared" | "copied" | null>(null)
	const lanAddress = lanIp ? `${lanIp}:${server.port}` : null
	const bestAddress = tunnel.publicAddress ?? lanAddress

	const share = async () => {
		if (!bestAddress) return
		const how = await shareText(
			t("join.shareTitle", { name: server.name }),
			t("join.shareText", { name: server.name, address: bestAddress, version: server.gameVersion }),
		)
		setShared(how)
		setTimeout(() => setShared(null), 2000)
	}

	return (
		<Card className="min-w-0 gap-3 p-4">
			<div className="flex items-start gap-3">
				<div className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-info/10 text-info">
					<Globe className="size-4" />
				</div>
				<div className="min-w-0">
					<h3 className="font-semibold text-foreground text-sm">{t("join.title")}</h3>
					<p className="text-muted-foreground text-xs">{t("join.description")}</p>
				</div>
			</div>

			{lanAddress && (
				<AddressRow icon={Wifi} label={t("join.sameWifi")}>
					<div className="flex items-center justify-between gap-2">
						<code className="truncate font-mono text-foreground text-sm">{lanAddress}</code>
						<CopyButton text={lanAddress} label={t("join.copyLocal")} />
					</div>
				</AddressRow>
			)}

			<AddressRow icon={Globe} label={t("join.anywhere")}>
				{tunnel.isRunning && tunnel.publicAddress ? (
					<div className="flex items-center justify-between gap-1">
						<code className="truncate font-mono font-semibold text-primary text-sm">
							{tunnel.publicAddress}
						</code>
						<div className="flex shrink-0">
							<CopyButton text={tunnel.publicAddress} label={t("join.copyPublic")} />
							<Button
								size="icon-sm"
								variant="ghost"
								aria-label={t("join.showQr")}
								onClick={() => setQrOpen(true)}
							>
								<QrCode className="size-3.5" />
							</Button>
						</div>
					</div>
				) : tunnel.isRunning && tunnel.status === "claiming" && tunnel.claimUrl ? (
					<a
						href={tunnel.claimUrl}
						target="_blank"
						rel="noopener noreferrer"
						className="inline-flex items-center gap-1 font-medium text-info text-sm hover:underline"
					>
						{t("join.linkPlayit")} <ExternalLink className="size-3" />
					</a>
				) : tunnel.isRunning && tunnel.status === "no_tunnel" ? (
					<p className="text-warning text-xs leading-relaxed">
						<Trans
							i18nKey="join.noTunnel"
							values={{ port: server.port }}
							components={{
								link: (
									// biome-ignore lint/a11y/useAnchorContent: Trans fills in the link text
									<a
										href="https://playit.gg/account/tunnels"
										target="_blank"
										rel="noopener noreferrer"
										className="font-semibold text-warning underline"
									/>
								),
							}}
						/>
					</p>
				) : tunnel.isRunning ? (
					<p className="flex items-center gap-1.5 text-muted-foreground text-sm">
						<Spinner className="size-3.5 text-info" />
						{tunnel.message ?? t("join.connecting")}
					</p>
				) : (
					<p className="text-muted-foreground text-sm">{t("join.notConnected")}</p>
				)}
			</AddressRow>

			{tunnel.status === "error" && tunnel.message && <ErrorNote>{tunnel.message}</ErrorNote>}

			<div className="grid grid-cols-2 gap-2">
				<Button
					variant="outline"
					disabled={isBusy}
					onClick={() => toggle(server.playitSecretKey)}
					className="h-auto min-h-10 min-w-0 gap-1.5 whitespace-normal rounded-xl py-2 text-center leading-tight"
				>
					{isBusy && <Spinner className="size-4" />}
					{tunnel.isRunning ? t("join.stopPlayit") : t("join.goPublic")}
				</Button>
				<Button
					onClick={share}
					disabled={!bestAddress}
					className="h-auto min-h-10 min-w-0 gap-1.5 whitespace-normal rounded-xl bg-info py-2 text-center text-info-foreground leading-tight hover:bg-info/80"
				>
					{shared === "copied" ? <Check className="size-4" /> : <Share2 className="size-4" />}
					{shared === "copied" ? t("skinPreview.copied") : t("join.share")}
				</Button>
			</div>

			<Dialog open={qrOpen} onOpenChange={setQrOpen}>
				<DialogContent className="items-center p-6 text-center sm:max-w-xs">
					<DialogTitle className="text-sm">{t("join.scan")}</DialogTitle>
					<div className="rounded-2xl bg-white p-3">
						<img
							src={`https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodeURIComponent(tunnel.publicAddress ?? "")}`}
							alt={t("join.qrAlt")}
							className="size-48"
						/>
					</div>
					<code className="font-mono font-semibold text-primary text-sm">
						{tunnel.publicAddress}
					</code>
				</DialogContent>
			</Dialog>
		</Card>
	)
}

// ─── Performance ──────────────────────────────────────────────────────────────

const HISTORY = 60

/**
 * Label sitting on a chart line. The solid background hides whatever passes behind it
 * (the graph, gridlines, reference lines), so text never gets crossed out.
 */
const CHART_CHIP =
	"pointer-events-none absolute -translate-y-1/2 whitespace-nowrap rounded-sm bg-card px-1 text-3xs tabular-nums leading-3.5 ring-1 ring-border"

/** Seconds between stat samples (useServerStats polls every 2s) */
const SAMPLE_SECONDS = 2

type Marker = { value: number; label: string; color: string }

/**
 * Area chart of the last HISTORY samples, filling in from the right like a task manager.
 * Scale and reference labels sit on their lines as chips, optional reference lines, and hover/touch to read any sample.
 */
function Chart({
	values,
	max,
	color,
	format,
	markers = [],
}: {
	values: number[]
	max: number
	color: string
	format: (v: number) => string
	markers?: Marker[]
}) {
	const { t } = useTranslation()
	const [hover, setHover] = useState<number | null>(null)
	const ref = useRef<HTMLDivElement>(null)
	const w = 300
	const h = 64
	const step = w / (HISTORY - 1)
	const offset = (HISTORY - values.length) * step
	const y = (v: number) => h - (Math.min(Math.max(v, 0), max) / max) * (h - 4) - 2
	const points = values.map((v, i) => [offset + i * step, y(v)])
	// Curve through midpoints so steps between samples look soft
	let line = ""
	points.forEach(([x, py], i) => {
		if (i === 0) line = `M${x.toFixed(1)},${py.toFixed(1)}`
		else {
			const [px, ppy] = points[i - 1]
			const mx = ((px + x) / 2).toFixed(1)
			line += ` C${mx},${ppy.toFixed(1)} ${mx},${py.toFixed(1)} ${x.toFixed(1)},${py.toFixed(1)}`
		}
	})
	const gradient = `chart-${color.slice(1)}`
	const active = hover ?? values.length - 1
	const dot = points[active]

	const pick = (clientX: number) => {
		const rect = ref.current?.getBoundingClientRect()
		if (!rect || values.length === 0) return
		const slot = Math.round(((clientX - rect.left) / rect.width) * (HISTORY - 1))
		const index = slot - (HISTORY - values.length)
		setHover(index < 0 ? null : Math.min(index, values.length - 1))
	}
	const secondsAgo = (values.length - 1 - active) * SAMPLE_SECONDS
	return (
		<div className="flex flex-col gap-1">
			<div
				ref={ref}
				role="img"
				aria-label={
					values.length > 0
						? t("stats.latest", { value: format(values[values.length - 1]) })
						: t("stats.noData")
				}
				// Press-and-hold reads values: no text selection or long-press menu on touch
				className="relative h-20 w-full touch-pan-y select-none [-webkit-touch-callout:none]"
				onContextMenu={(e) => e.preventDefault()}
				onPointerMove={(e) => pick(e.clientX)}
				onPointerDown={(e) => pick(e.clientX)}
				onPointerLeave={() => setHover(null)}
				onPointerCancel={() => setHover(null)}
			>
				<svg
					viewBox={`0 0 ${w} ${h}`}
					preserveAspectRatio="none"
					className="absolute inset-0 size-full overflow-visible"
					aria-hidden
				>
					<defs>
						<linearGradient id={gradient} x1="0" x2="0" y1="0" y2="1">
							<stop offset="0" stopColor={color} stopOpacity={0.3} />
							<stop offset="1" stopColor={color} stopOpacity={0} />
						</linearGradient>
					</defs>
					{[max, max / 2].map((v) => (
						<line
							key={v}
							x1={0}
							x2={w}
							y1={y(v)}
							y2={y(v)}
							stroke="currentColor"
							strokeDasharray="2 4"
							vectorEffect="non-scaling-stroke"
							className="text-border"
						/>
					))}
					<line
						x1={0}
						x2={w}
						y1={h - 0.5}
						y2={h - 0.5}
						stroke="currentColor"
						vectorEffect="non-scaling-stroke"
						className="text-border"
					/>
					{markers.map((m) => (
						<line
							key={m.label}
							x1={0}
							x2={w}
							y1={y(m.value)}
							y2={y(m.value)}
							stroke={m.color}
							strokeOpacity={0.6}
							strokeDasharray="4 3"
							vectorEffect="non-scaling-stroke"
						/>
					))}
					{points.length >= 2 && (
						<>
							<path
								d={`${line} L${w},${h} L${offset.toFixed(1)},${h} Z`}
								fill={`url(#${gradient})`}
							/>
							<path
								d={line}
								fill="none"
								stroke={color}
								strokeWidth={1.75}
								strokeLinejoin="round"
								vectorEffect="non-scaling-stroke"
							/>
						</>
					)}
					{hover !== null && dot && (
						<line
							x1={dot[0]}
							x2={dot[0]}
							y1={0}
							y2={h}
							stroke="currentColor"
							vectorEffect="non-scaling-stroke"
							className="text-muted-foreground/60"
						/>
					)}
				</svg>
				{/* HTML overlays stay undistorted by the stretched SVG */}
				{[max, max / 2].map((v) => (
					<span
						key={v}
						className={cn(CHART_CHIP, "left-0 text-muted-foreground")}
						style={{ top: `${(y(v) / h) * 100}%` }}
					>
						{format(v)}
					</span>
				))}
				{markers.map((m) => (
					<span
						key={m.label}
						className={cn(CHART_CHIP, "right-0")}
						style={{ top: `${(y(m.value) / h) * 100}%`, color: m.color }}
					>
						{m.label} {format(m.value)}
					</span>
				))}
				{dot && (
					<span
						className="-translate-1/2 pointer-events-none absolute size-2 rounded-full ring-2 ring-border"
						style={{
							left: `${(dot[0] / w) * 100}%`,
							top: `${(dot[1] / h) * 100}%`,
							background: color,
						}}
					/>
				)}
				{hover !== null && dot && (
					<span
						className={cn(
							"pointer-events-none absolute top-0 ml-1.5 whitespace-nowrap rounded-md border border-border bg-card px-1.5 py-0.5 text-3xs text-foreground tabular-nums shadow-lg",
							dot[0] / w > 0.6 && "-ml-1.5 -translate-x-full",
						)}
						style={{ left: `${(dot[0] / w) * 100}%` }}
					>
						{format(values[active])}
						<span className="text-muted-foreground">
							{" · "}
							{secondsAgo === 0 ? t("stats.now") : t("stats.ago", { time: formatAgo(secondsAgo) })}
						</span>
					</span>
				)}
			</div>
			<div className="flex justify-between text-3xs text-muted-foreground/60 leading-none">
				<span>{t("stats.ago", { time: formatAgo(HISTORY * SAMPLE_SECONDS) })}</span>
				<span>{t("stats.now")}</span>
			</div>
		</div>
	)
}

function formatAgo(seconds: number): string {
	return formatDuration(seconds)
}

function Metric({
	icon: Icon,
	label,
	value,
	sub,
	values,
	max,
	color,
	format,
	markers,
	footer,
}: {
	icon: typeof Cpu
	label: string
	value: string
	sub?: string
	values: number[]
	max: number
	color: string
	format: (v: number) => string
	markers?: Marker[]
	footer?: ReactNode
}) {
	const { t } = useTranslation()
	const summary =
		values.length > 0
			? {
					min: Math.min(...values),
					avg: values.reduce((a, b) => a + b, 0) / values.length,
					peak: Math.max(...values),
				}
			: null
	return (
		<div className="flex min-w-0 select-none flex-col gap-3 rounded-xl bg-background/60 p-3">
			<div className="flex min-w-0 items-baseline justify-between gap-3">
				<p className="flex shrink-0 items-center gap-1.5 text-2xs text-muted-foreground uppercase tracking-wider">
					<Icon className="size-3.5 self-center" style={{ color }} /> {label}
				</p>
				<p className="min-w-0 truncate text-right font-semibold text-base text-foreground tabular-nums">
					{value}
					{sub && <span className="font-normal text-muted-foreground text-xs"> {sub}</span>}
				</p>
			</div>
			<Chart values={values} max={max} color={color} format={format} markers={markers} />
			{summary && (
				<dl className="grid grid-cols-3 gap-2 border-border border-t pt-2.5 text-2xs">
					{(["min", "avg", "peak"] as const).map((k) => (
						<div key={k} className="min-w-0">
							<dt className="text-muted-foreground/60">{t(`stats.${k}`)}</dt>
							<dd className="truncate text-foreground/80 tabular-nums">{format(summary[k])}</dd>
						</div>
					))}
				</dl>
			)}
			{footer}
		</div>
	)
}

/** CPU, memory and TPS of the running server, with the last two minutes as graphs */
export function PerformanceCard({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
	const { isRunning } = useServerStatus(server.id)
	// The backend records samples while the server runs, so history survives leaving this page
	const { data: history = [] } = useServerStats(server.id, isRunning)

	if (!isRunning) return null
	const latest = history[history.length - 1]
	const pumpkin = server.core === "pumpkin"
	const memoryLimit = pumpkin ? (latest?.systemMemoryTotalMb ?? 1) : server.memoryMaxMb
	const memoryValues = history.map((s) => s.memoryMb)
	// Round the scale up so labels are tidy and the allocation line sits below the top
	const memoryMax = Math.ceil((Math.max(memoryLimit, ...memoryValues) * 1.1) / 512) * 512
	// TPS is unknown until the server finishes starting; start its graph at the first reading
	const firstTps = history.findIndex((s) => s.tps !== null)
	const tpsValues = firstTps < 0 ? [] : history.slice(firstTps).map((s) => s.tps ?? 0)

	return (
		<Card className="min-w-0 gap-3 p-4">
			<div className="flex items-center gap-2">
				<Activity className="size-4 text-muted-foreground" />
				<h3 className="font-semibold text-foreground text-sm">{t("stats.performance")}</h3>
				{!latest && <Spinner className="size-3.5 text-muted-foreground" />}
				<span className="ml-auto text-2xs text-muted-foreground/60">{t("stats.touchHint")}</span>
			</div>
			<div className="grid gap-2 lg:grid-cols-3">
				<Metric
					icon={Cpu}
					label="CPU"
					value={latest ? formatPercent(latest.cpuPercent) : "—"}
					sub={t("stats.ofDevice")}
					values={history.map((s) => s.cpuPercent)}
					max={100}
					color="#38bdf8"
					format={(v) => formatPercent(v)}
				/>
				<Metric
					icon={MemoryStick}
					label={t("wizard.ram")}
					value={latest ? formatMb(latest.memoryMb) : "—"}
					sub={`/ ${formatMb(memoryLimit)}`}
					values={memoryValues}
					max={memoryMax}
					color="#a78bfa"
					format={formatMb}
					markers={
						pumpkin ? [] : [{ value: memoryLimit, label: t("stats.allocated"), color: "#a78bfa" }]
					}
					footer={
						latest && (
							<DeviceBar
								label={t("stats.deviceRam")}
								used={latest.systemMemoryUsedMb}
								part={latest.memoryMb}
								total={latest.systemMemoryTotalMb}
								color="#a78bfa"
								format={formatMb}
								note={t("stats.free", {
									size: formatMb(
										Math.max(0, latest.systemMemoryTotalMb - latest.systemMemoryUsedMb),
									),
								})}
							/>
						)
					}
				/>
				{firstTps >= 0 && (
					<Metric
						icon={Gauge}
						label="TPS"
						value={latest?.tps != null ? formatTps(latest.tps) : "—"}
						sub={tpsHealth(latest?.tps ?? null)}
						values={tpsValues}
						max={20}
						color={latest?.tps != null && latest.tps < 15 ? "#f59e0b" : "#34d399"}
						format={formatTps}
						markers={[{ value: 15, label: t("stats.lag"), color: "#f59e0b" }]}
					/>
				)}
			</div>
		</Card>
	)
}

function tpsHealth(tps: number | null): string {
	if (tps === null) return ""
	if (tps >= 19) return i18n.t("stats.smooth")
	if (tps >= 15) return i18n.t("stats.slightLag")
	return i18n.t("stats.lagging")
}

function formatTps(tps: number): string {
	return formatNumber(tps, { minimumFractionDigits: 1, maximumFractionDigits: 1 })
}

/** Bar of a device resource: this server's part, everything else in use, then free */
function DeviceBar({
	label,
	used,
	part,
	total,
	color,
	format,
	note,
}: {
	label: string
	used: number
	part: number
	total: number
	color: string
	format: (v: number) => string
	note?: string
}) {
	const { t } = useTranslation()
	const pct = (v: number) => `${Math.min(100, (v / Math.max(1, total)) * 100)}%`
	return (
		<div className="flex flex-col gap-1.5">
			<div className="flex h-1.5 overflow-hidden rounded-full bg-muted/80">
				<div style={{ width: pct(part), background: color }} />
				<div className="bg-accent" style={{ width: pct(Math.max(0, used - part)) }} />
			</div>
			<p className="flex flex-wrap justify-between gap-x-3 text-2xs text-muted-foreground">
				<span>{t("stats.used", { label, used: format(used), total: format(total) })}</span>
				{note && <span>{note}</span>}
			</p>
		</div>
	)
}

// ─── Storage ──────────────────────────────────────────────────────────────────

/** Labels are storage.<id> in the locale files */
const STORAGE_CATEGORIES = {
	worlds: { color: "#34d399" },
	plugins: { color: "#38bdf8" },
	server: { color: "#a78bfa" },
	backups: { color: "#f59e0b" },
	logs: { color: "#f472b6" },
	other: { color: "#71717a" },
} as const
type StorageCategory = keyof typeof STORAGE_CATEGORIES
const category = (id: string): StorageCategory =>
	id in STORAGE_CATEGORIES ? (id as StorageCategory) : "other"

function formatShare(share: number): string {
	return share > 0 && share < 0.01 ? `<${formatPercent(1)}` : formatPercent(share * 100)
}

/** How much disk space the server folder takes, split by what's in it */
export function StorageCard({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
	const { data, isLoading } = useServerStorage(server.id)
	const total = data?.totalBytes ?? 0
	return (
		<Card className="min-w-0 gap-3 p-4">
			<div className="flex items-center gap-2">
				<HardDrive className="size-4 text-muted-foreground" />
				<h3 className="font-semibold text-foreground text-sm">{t("storage.title")}</h3>
				{isLoading && <Spinner className="size-3.5 text-muted-foreground" />}
				{data && (
					<span className="ml-auto font-semibold text-foreground text-sm tabular-nums">
						{formatBytes(total)}
					</span>
				)}
			</div>
			{data && (
				<>
					<div className="flex h-2.5 gap-0.5 overflow-hidden rounded-full bg-muted/80">
						{data.categories.map((c) => (
							<div
								key={c.id}
								className="min-w-1"
								style={{
									width: `${(c.bytes / Math.max(1, total)) * 100}%`,
									background: STORAGE_CATEGORIES[category(c.id)].color,
								}}
							/>
						))}
					</div>
					<ul className="grid gap-x-6 gap-y-2 sm:grid-cols-2">
						{data.categories.map((c) => {
							const meta = STORAGE_CATEGORIES[category(c.id)]
							return (
								<li key={c.id} className="flex min-w-0 flex-col gap-1">
									<div className="flex items-center gap-2 text-xs">
										<span
											className="size-2 shrink-0 rounded-full"
											style={{ background: meta.color }}
										/>
										<span className="min-w-0 flex-1 truncate text-foreground/80">
											{t(`storage.${category(c.id)}`)}
										</span>
										<span className="text-muted-foreground tabular-nums">
											{formatShare(c.bytes / Math.max(1, total))}
										</span>
										<span className="w-16 text-right text-foreground tabular-nums">
											{formatBytes(c.bytes)}
										</span>
									</div>
									{c.id === "worlds" && data.worlds.length > 1 && (
										<ul className="ml-4 flex flex-col gap-0.5">
											{data.worlds.map((w) => (
												<li key={w.name} className="flex gap-2 text-2xs text-muted-foreground">
													<span className="min-w-0 flex-1 truncate">{w.name}</span>
													<span className="tabular-nums">{formatBytes(w.bytes)}</span>
												</li>
											))}
										</ul>
									)}
								</li>
							)
						})}
					</ul>
					{data.deviceTotalBytes != null && data.deviceFreeBytes != null && (
						<DeviceBar
							label={t("storage.device")}
							used={data.deviceTotalBytes - data.deviceFreeBytes}
							part={total}
							total={data.deviceTotalBytes}
							color="#34d399"
							format={formatBytes}
							note={t("stats.free", { size: formatBytes(data.deviceFreeBytes) })}
						/>
					)}
				</>
			)}
		</Card>
	)
}

function formatMb(mb: number): string {
	return formatMegabytes(mb)
}
