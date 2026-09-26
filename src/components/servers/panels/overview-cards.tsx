import {
	Activity,
	Check,
	Copy,
	Cpu,
	ExternalLink,
	Gauge,
	Globe,
	Loader2,
	MemoryStick,
	QrCode,
	Share2,
	Wifi,
} from "lucide-react"
import { type ReactNode, useEffect, useState } from "react"
import type { ServerConfig, ServerStats } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { shareText } from "@/lib/share"
import { cn } from "@/lib/utils"
import { usePlayitTunnel } from "@/services/hosting"
import { useLanAddress, useServerStats, useServerStatus } from "@/services/server-data"
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
			{copied ? <Check className="size-3.5 text-emerald-400" /> : <Copy className="size-3.5" />}
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
		<div className="flex min-w-0 items-center gap-3 rounded-xl bg-zinc-950/60 py-2 pr-2 pl-3">
			<Icon className="size-4 shrink-0 text-zinc-500" />
			<div className="min-w-0 flex-1">
				<p className="text-[10px] text-zinc-500 uppercase tracking-wider">{label}</p>
				{children}
			</div>
		</div>
	)
}

/** How friends can join: LAN and playit.gg addresses, plus a share button */
export function JoinCard({ server }: { server: ServerConfig }) {
	const { tunnel, toggle, isBusy } = usePlayitTunnel()
	const { data: lanIp } = useLanAddress()
	const [qrOpen, setQrOpen] = useState(false)
	const [shared, setShared] = useState<"shared" | "copied" | null>(null)
	const lanAddress = lanIp ? `${lanIp}:${server.port}` : null
	const bestAddress = tunnel.publicAddress ?? lanAddress

	const share = async () => {
		if (!bestAddress) return
		const how = await shareText(
			`Join ${server.name}`,
			`Join my Minecraft server "${server.name}"!\nAddress: ${bestAddress}\nVersion: Java Edition ${server.gameVersion}`,
		)
		setShared(how)
		setTimeout(() => setShared(null), 2000)
	}

	return (
		<Card className="flex min-w-0 flex-col gap-3 p-4">
			<div className="flex items-start gap-3">
				<div className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-sky-500/10 text-sky-400">
					<Globe className="size-4" />
				</div>
				<div className="min-w-0">
					<h3 className="font-semibold text-sm text-zinc-100">Invite players</h3>
					<p className="text-xs text-zinc-500">Share an address so friends can join.</p>
				</div>
			</div>

			{lanAddress && (
				<AddressRow icon={Wifi} label="Same Wi-Fi">
					<div className="flex items-center justify-between gap-2">
						<code className="truncate font-mono text-sm text-zinc-200">{lanAddress}</code>
						<CopyButton text={lanAddress} label="Copy local address" />
					</div>
				</AddressRow>
			)}

			<AddressRow icon={Globe} label="Anywhere (playit.gg)">
				{tunnel.isRunning && tunnel.publicAddress ? (
					<div className="flex items-center justify-between gap-1">
						<code className="truncate font-mono font-semibold text-emerald-300 text-sm">
							{tunnel.publicAddress}
						</code>
						<div className="flex shrink-0">
							<CopyButton text={tunnel.publicAddress} label="Copy public address" />
							<Button
								size="icon-sm"
								variant="ghost"
								aria-label="Show QR code"
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
						className="inline-flex items-center gap-1 font-medium text-sky-300 text-sm hover:underline"
					>
						Link your playit.gg account <ExternalLink className="size-3" />
					</a>
				) : tunnel.isRunning && tunnel.status === "no_tunnel" ? (
					<p className="text-amber-200 text-xs leading-relaxed">
						Connected, but no tunnel yet.{" "}
						<a
							href="https://playit.gg/account/tunnels"
							target="_blank"
							rel="noopener noreferrer"
							className="font-semibold text-amber-300 underline"
						>
							Add one
						</a>{" "}
						for port {server.port}.
					</p>
				) : tunnel.isRunning ? (
					<p className="flex items-center gap-1.5 text-sm text-zinc-400">
						<Loader2 className="size-3.5 animate-spin text-sky-400" />
						{tunnel.message ?? "Connecting..."}
					</p>
				) : (
					<p className="text-sm text-zinc-500">Not connected</p>
				)}
			</AddressRow>

			{tunnel.status === "error" && tunnel.message && <ErrorNote>{tunnel.message}</ErrorNote>}

			<div className="grid grid-cols-2 gap-2">
				<Button
					variant="outline"
					disabled={isBusy}
					onClick={() => toggle(server.playitSecretKey)}
					className="h-10 gap-1.5 rounded-xl border-zinc-800"
				>
					{isBusy && <Loader2 className="size-4 animate-spin" />}
					{tunnel.isRunning ? "Stop playit" : "Go public"}
				</Button>
				<Button
					onClick={share}
					disabled={!bestAddress}
					className="h-10 gap-1.5 rounded-xl bg-sky-600 text-white hover:bg-sky-500"
				>
					{shared === "copied" ? <Check className="size-4" /> : <Share2 className="size-4" />}
					{shared === "copied" ? "Copied" : "Share invite"}
				</Button>
			</div>

			<Dialog open={qrOpen} onOpenChange={setQrOpen}>
				<DialogContent className="items-center p-6 text-center sm:max-w-xs">
					<DialogTitle className="text-sm">Scan to copy the address</DialogTitle>
					<div className="rounded-2xl bg-white p-3">
						<img
							src={`https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodeURIComponent(tunnel.publicAddress ?? "")}`}
							alt="QR code"
							className="size-48"
						/>
					</div>
					<code className="font-mono font-semibold text-emerald-300 text-sm">
						{tunnel.publicAddress}
					</code>
				</DialogContent>
			</Dialog>
		</Card>
	)
}

// ─── Performance ──────────────────────────────────────────────────────────────

const HISTORY = 60

/** Tiny line chart; values scaled to `max` */
function Sparkline({ values, max, color }: { values: number[]; max: number; color: string }) {
	if (values.length < 2) return <div className="h-10" />
	const w = 100
	const h = 32
	const step = w / (HISTORY - 1)
	const offset = (HISTORY - values.length) * step
	const points = values.map((v, i) => [
		offset + i * step,
		h - (Math.min(v, max) / max) * (h - 2) - 1,
	])
	const line = points.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" ")
	const area = `${offset},${h} ${line} ${w},${h}`
	return (
		<svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" className="h-10 w-full" aria-hidden>
			<polygon points={area} fill={color} opacity={0.12} />
			<polyline
				points={line}
				fill="none"
				stroke={color}
				strokeWidth={1.5}
				vectorEffect="non-scaling-stroke"
			/>
		</svg>
	)
}

function Metric({
	icon: Icon,
	label,
	value,
	sub,
	values,
	max,
	color,
}: {
	icon: typeof Cpu
	label: string
	value: string
	sub?: string
	values: number[]
	max: number
	color: string
}) {
	return (
		<div className="flex min-w-0 flex-col gap-1 rounded-xl bg-zinc-950/60 p-3">
			<p className="flex items-center gap-1.5 text-[10px] text-zinc-500 uppercase tracking-wider">
				<Icon className="size-3" /> {label}
			</p>
			<p className="font-semibold text-base text-zinc-100">
				{value}
				{sub && <span className="font-normal text-xs text-zinc-500"> {sub}</span>}
			</p>
			<Sparkline values={values} max={max} color={color} />
		</div>
	)
}

/** CPU, memory and TPS of the running server, with the last two minutes as graphs */
export function PerformanceCard({ server }: { server: ServerConfig }) {
	const { isRunning } = useServerStatus(server.id)
	const { data } = useServerStats(server.id, isRunning)
	const [history, setHistory] = useState<ServerStats[]>([])

	useEffect(() => {
		if (!isRunning) setHistory([])
	}, [isRunning])
	useEffect(() => {
		if (data) setHistory((h) => [...h.slice(-(HISTORY - 1)), data])
	}, [data])

	if (!isRunning) return null
	const latest = history[history.length - 1]
	const memoryLimit =
		server.core === "pumpkin" ? (latest?.systemMemoryTotalMb ?? 1) : server.memoryMaxMb
	const hasTps = history.some((s) => s.tps !== null)

	return (
		<Card className="flex min-w-0 flex-col gap-3 p-4">
			<div className="flex items-center gap-2">
				<Activity className="size-4 text-zinc-400" />
				<h3 className="font-semibold text-sm text-zinc-100">Performance</h3>
				{!latest && <Loader2 className="size-3.5 animate-spin text-zinc-500" />}
			</div>
			<div className={cn("grid gap-2", hasTps ? "grid-cols-3" : "grid-cols-2")}>
				<Metric
					icon={Cpu}
					label="CPU"
					value={latest ? `${Math.round(latest.cpuPercent)}%` : "—"}
					values={history.map((s) => s.cpuPercent)}
					max={100}
					color="#38bdf8"
				/>
				<Metric
					icon={MemoryStick}
					label="RAM"
					value={latest ? formatMb(latest.memoryMb) : "—"}
					sub={`/ ${formatMb(memoryLimit)}`}
					values={history.map((s) => s.memoryMb)}
					max={Math.max(memoryLimit, ...history.map((s) => s.memoryMb))}
					color="#a78bfa"
				/>
				{hasTps && (
					<Metric
						icon={Gauge}
						label="TPS"
						value={latest?.tps != null ? latest.tps.toFixed(1) : "—"}
						values={history.map((s) => s.tps ?? 0)}
						max={20}
						color={latest?.tps != null && latest.tps < 15 ? "#f59e0b" : "#34d399"}
					/>
				)}
			</div>
			{latest && (
				<p className="text-[11px] text-zinc-500">
					Device memory: {formatMb(latest.systemMemoryUsedMb)} of{" "}
					{formatMb(latest.systemMemoryTotalMb)} in use
				</p>
			)}
		</Card>
	)
}

function formatMb(mb: number): string {
	return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`
}
