import { useQuery } from "@tanstack/react-query"
import { save as saveDialog } from "@tauri-apps/plugin-dialog"
import { ArrowUpDown, Copy, FileArchive, Loader2, Package, Share2, Upload } from "lucide-react"
import { useRef, useState } from "react"
import type { ExportMode, ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { canShareFiles, shareFile } from "@/lib/share"
import { useServerStatus } from "@/services/server-data"
import { rpc, serverService } from "@/services/server-service"
import { Card, CardHeader, ErrorNote } from "../shared/primitives"

type Busy = "export-configs" | "export-full" | "copy" | "version" | null

/** Export, copy and change version */
export function TransferCard({ server }: { server: ServerConfig }) {
	const { status, isRunning } = useServerStatus(server.id)
	const stopped = status === "stopped"
	const [busy, setBusy] = useState<Busy>(null)
	const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null)
	const [versionOpen, setVersionOpen] = useState(false)

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
				filters: [{ name: "Zip archive", extensions: ["zip"] }],
			})
			if (!dest) return null
			const path = await rpc.export_server(server.id, mode, dest)
			return `Saved to ${path}`
		})

	const copy = () =>
		run("copy", async () => {
			const created = await rpc.duplicate_server(server.id, `${server.name} copy`, null, null)
			await serverService.refreshServers()
			return `Created "${created.name}".`
		})

	return (
		<Card>
			<CardHeader
				icon={FileArchive}
				title="Backup & version"
				description="Export to share or back up, copy the server, or move to another Minecraft version."
			/>
			<div className="flex flex-col gap-2 px-4 pb-4">
				<div className="grid grid-cols-2 gap-2">
					<ActionButton
						icon={canShareFiles() ? Share2 : Package}
						label="Export configs"
						hint="Settings only"
						busy={busy === "export-configs"}
						disabled={busy !== null}
						onClick={() => exportServer("configs")}
					/>
					<ActionButton
						icon={canShareFiles() ? Share2 : FileArchive}
						label="Export all"
						hint="With worlds"
						busy={busy === "export-full"}
						disabled={busy !== null}
						onClick={() => exportServer("full")}
					/>
					<ActionButton
						icon={Copy}
						label="Make a copy"
						hint={stopped ? "A second server" : "Stop the server first"}
						busy={busy === "copy"}
						disabled={busy !== null || !stopped}
						onClick={copy}
					/>
					<ActionButton
						icon={ArrowUpDown}
						label="Change version"
						hint={stopped ? `Now ${server.gameVersion}` : "Stop the server first"}
						busy={busy === "version"}
						disabled={busy !== null || !stopped}
						onClick={() => setVersionOpen(true)}
					/>
				</div>
				{notice &&
					(notice.ok ? (
						<p className="break-all rounded-xl border border-emerald-500/20 bg-emerald-500/10 px-3 py-2.5 text-emerald-200 text-xs">
							{notice.text}
						</p>
					) : (
						<ErrorNote>{notice.text}</ErrorNote>
					))}
			</div>
			<ChangeVersionDialog
				server={server}
				open={versionOpen}
				onOpenChange={setVersionOpen}
				onDone={(text) => setNotice({ ok: true, text })}
			/>
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

/** Numeric comparison of versions like "1.21.4" and "26.2" */
function compareVersions(a: string, b: string): number {
	const pa = a.split(".").map((n) => Number.parseInt(n, 10) || 0)
	const pb = b.split(".").map((n) => Number.parseInt(n, 10) || 0)
	for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
		const d = (pa[i] ?? 0) - (pb[i] ?? 0)
		if (d !== 0) return d
	}
	return 0
}

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
	const { data: versions = [], isLoading } = useQuery({
		queryKey: ["core-versions", server.core],
		queryFn: () => serverService.getAvailableServerCoreVersions(server.core),
		enabled: open,
		staleTime: 10 * 60_000,
	})
	const [version, setVersion] = useState<string | null>(null)
	const [keepBackup, setKeepBackup] = useState(true)
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const target = version ?? versions.find((v) => v !== server.gameVersion) ?? null
	const downgrade = target !== null && compareVersions(target, server.gameVersion) < 0

	const apply = async () => {
		if (!target) return
		setBusy(true)
		setError(null)
		try {
			let backupName: string | null = null
			if (keepBackup) {
				const backup = await rpc.duplicate_server(
					server.id,
					`${server.name} (${server.gameVersion} backup)`,
					null,
					null,
				)
				backupName = backup.name
			}
			await rpc.change_server_version(server.id, target, null)
			await serverService.refreshServers()
			onOpenChange(false)
			onDone(
				`${server.name} will start on ${target}.${backupName ? ` Your ${server.gameVersion} version is saved as "${backupName}".` : ""}`,
			)
		} catch (e) {
			setError(String(e))
		} finally {
			setBusy(false)
		}
	}

	const options = versions.map((v) => ({
		value: v,
		label: v === server.gameVersion ? `${v} (current)` : v,
	}))

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="gap-4 p-5 sm:max-w-md">
				<DialogTitle className="text-base">Change Minecraft version</DialogTitle>
				<p className="text-sm text-zinc-400 leading-relaxed">
					The server downloads the new version on its next start, and Minecraft converts the world.
					Converted worlds can't be opened by older versions.
				</p>
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
								<SelectItem key={o.value} value={o.value} disabled={o.value === server.gameVersion}>
									{o.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				)}
				<div className="flex items-center justify-between gap-3 rounded-xl bg-zinc-900/60 p-3">
					<div>
						<p className="font-medium text-sm text-zinc-100">Keep a backup copy</p>
						<p className="text-xs text-zinc-500">
							Saves the current server so you can switch back.
						</p>
					</div>
					<Switch
						checked={keepBackup}
						onCheckedChange={setKeepBackup}
						aria-label="Keep a backup copy"
					/>
				</div>
				{downgrade && (
					<ErrorNote>
						{target} is older than {server.gameVersion}. Downgrading usually breaks the world
						{keepBackup ? "; the backup keeps your current world safe." : ". Keep a backup!"}
					</ErrorNote>
				)}
				{error && <ErrorNote>{error}</ErrorNote>}
				<Button
					onClick={apply}
					disabled={!target || busy}
					className="h-11 gap-2 rounded-xl bg-emerald-600 font-semibold text-white hover:bg-emerald-500"
				>
					{busy ? <Loader2 className="size-4 animate-spin" /> : <ArrowUpDown className="size-4" />}
					{keepBackup ? "Back up and switch" : "Switch version"}
				</Button>
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
	const inputRef = useRef<HTMLInputElement>(null)
	const [progress, setProgress] = useState<number | null>(null)
	const [error, setError] = useState<string | null>(null)

	const upload = async (file: File) => {
		const chunkSize = 2 * 1024 * 1024
		const uploadId = crypto.randomUUID()
		setError(null)
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
			const created = await rpc.import_server(uploadId)
			await serverService.refreshServers()
			onImported?.(created)
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
				{progress !== null ? `Importing ${Math.round(progress * 100)}%` : "Import server"}
			</Button>
			{error && <ErrorNote>{error}</ErrorNote>}
		</>
	)
}
