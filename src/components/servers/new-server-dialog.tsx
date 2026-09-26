import { AlertCircle, Cpu, Loader2, Plus, Server } from "lucide-react"
import { useEffect, useState } from "react"
import type { ServerCoreType } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import Slider from "@/components/ui/slider"
import { isMobileEnvironment } from "@/lib/platform"
import { cn } from "@/lib/utils"
import { serverService } from "@/services/server-service"

export interface NewServerDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	onServerCreated?: () => void
}

const SERVER_CORES: {
	id: ServerCoreType
	name: string
	tagline: string
	badge: string
	recommended?: boolean
}[] = [
	{
		id: "paper",
		name: "Paper",
		tagline: "High-performance Spigot/Bukkit server with plugin support",
		badge: "Fast & Stable",
		recommended: true,
	},
	{
		id: "purpur",
		name: "Purpur",
		tagline: "Feature-rich Paper fork with incredible customization",
		badge: "Customizable",
	},
	{
		id: "fabric",
		name: "Fabric Server",
		tagline: "Lightweight, modular modded server for Fabric mods",
		badge: "Modded",
	},
	{
		id: "folia",
		name: "Folia",
		tagline: "Multi-threaded regionized server by PaperMC for massive player counts",
		badge: "Multi-threaded",
	},
	{
		id: "vanilla",
		name: "Vanilla",
		tagline: "Official unmodded Minecraft server directly from Mojang",
		badge: "Original",
	},
	{
		id: "pumpkin",
		name: "PumpkinMC (Rust)",
		tagline: "Ultra lightweight modern core in Rust (< 60MB RAM, instant boot)",
		badge: "Ultra Low RAM",
	},
]

export default function NewServerDialog({
	open,
	onOpenChange,
	onServerCreated,
}: NewServerDialogProps) {
	const isMobile = isMobileEnvironment()
	const [name, setName] = useState("")
	const [core, setCore] = useState<ServerCoreType>(isMobile ? "pumpkin" : "paper")
	const [versions, setVersions] = useState<string[]>([])
	const [selectedVersion, setSelectedVersion] = useState("")
	const [port, setPort] = useState(25565)
	const [ramMb, setRamMb] = useState(isMobile ? 1024 : 4096)
	const [isLoadingVersions, setIsLoadingVersions] = useState(false)
	const [isSubmitting, setIsSubmitting] = useState(false)
	const [error, setError] = useState<string | null>(null)

	// Auto-select next available port when dialog opens
	useEffect(() => {
		if (!open) return
		const nextPort = serverService.getNextAvailablePort()
		setPort(nextPort)
	}, [open])

	// Fetch versions whenever core changes
	useEffect(() => {
		if (!open) return
		let isMounted = true
		setIsLoadingVersions(true)
		setError(null)

		serverService
			.getAvailableServerCoreVersions(core)
			.then((list) => {
				if (!isMounted) return
				setVersions(list)
				if (list.length > 0) {
					// Default to latest version or 1.21.4
					const best = list.find((v) => v === "1.21.4") || list[0]
					setSelectedVersion(best)
				}
			})
			.catch((err) => {
				if (!isMounted) return
				console.error("Failed to fetch versions:", err)
				setError("Failed to fetch core versions. Please check your internet connection.")
			})
			.finally(() => {
				if (isMounted) setIsLoadingVersions(false)
			})

		return () => {
			isMounted = false
		}
	}, [core, open])

	// Auto-generate name based on core & version if user hasn't typed custom name
	const handleSelectCore = (newCore: ServerCoreType) => {
		setCore(newCore)
		const coreObj = SERVER_CORES.find((c) => c.id === newCore)
		if (coreObj && (!name || SERVER_CORES.some((c) => name.startsWith(c.name)))) {
			setName(`${coreObj.name} Server`)
		}
	}

	const isPortConflict = serverService.isPortInUse(port)

	const handleCreate = async () => {
		if (!name.trim()) {
			setError("Please enter a server name.")
			return
		}
		if (!selectedVersion) {
			setError("Please select a Minecraft version.")
			return
		}
		if (!port || port < 1024 || port > 65535) {
			setError("Please enter a valid port between 1024 and 65535.")
			return
		}
		if (isPortConflict) {
			setError(
				`Port ${port} is already assigned to another server. Please choose a different port.`,
			)
			return
		}

		setIsSubmitting(true)
		setError(null)

		try {
			await serverService.createServer(
				name.trim(),
				core,
				selectedVersion,
				null,
				port,
				Math.min(2048, ramMb),
				ramMb,
			)
			onOpenChange(false)
			onServerCreated?.()
			// Reset
			setName("")
			setPort(25565)
			setRamMb(4096)
		} catch (err: unknown) {
			console.error("Failed to create server:", err)
			setError(err instanceof Error ? err.message : "Failed to create server")
		} finally {
			setIsSubmitting(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
				<DialogHeader>
					<div className="flex items-center gap-2.5">
						<div className="flex size-9 items-center justify-center rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-400">
							<Server className="size-4" />
						</div>
						<div>
							<DialogTitle className="font-semibold text-base text-foreground">
								Create Minecraft Server
							</DialogTitle>
							<DialogDescription className="text-xs">
								Choose a server core and version. Ingot will automatically download and prepare the
								server.
							</DialogDescription>
						</div>
					</div>
				</DialogHeader>

				<div className="flex flex-col gap-4 py-2">
					{/* Server Name */}
					<div className="flex flex-col gap-1.5">
						<label htmlFor="server-name" className="font-medium text-foreground text-xs">
							Server Name
						</label>
						<Input
							id="server-name"
							placeholder="e.g. My Paper Survival Server"
							value={name}
							onChange={(e) => setName(e.target.value)}
							className="h-9 text-xs"
						/>
					</div>

					{/* Core Selection */}
					<div className="flex flex-col gap-1.5">
						<span className="font-medium text-foreground text-xs">Select Server Core</span>
						<div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
							{SERVER_CORES.map((c) => {
								const isSelected = core === c.id
								const isRecommended = isMobile ? c.id === "pumpkin" : c.recommended
								return (
									<button
										key={c.id}
										type="button"
										onClick={() => handleSelectCore(c.id)}
										className={cn(
											"flex flex-col rounded-xl border p-3 text-left transition-all",
											isSelected
												? "border-emerald-500/60 bg-emerald-500/10 shadow-emerald-950/20 shadow-sm ring-1 ring-emerald-500/40"
												: "border-border/50 bg-zinc-900/40 hover:border-border hover:bg-zinc-900/80",
										)}
									>
										<div className="flex items-center justify-between">
											<span className="font-semibold text-foreground text-xs">{c.name}</span>
											<span
												className={cn(
													"rounded-full px-2 py-0.5 font-medium text-[10px]",
													isSelected
														? "bg-emerald-500/20 text-emerald-300"
														: isRecommended
															? "border border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
															: "bg-zinc-800 text-zinc-400",
												)}
											>
												{isMobile && c.id === "pumpkin" ? "Mobile Native" : c.badge}
											</span>
										</div>
										<p className="mt-1 line-clamp-2 text-[11px] text-muted-foreground leading-relaxed">
											{c.tagline}
										</p>
									</button>
								)
							})}
						</div>
						{isMobile && core !== "pumpkin" && (
							<p className="mt-1 rounded-lg border border-amber-500/20 bg-amber-500/10 p-2 text-[11px] text-amber-400/90 leading-relaxed">
								Note: Java cores require the unprivileged Linux runtime environment. PumpkinMC is
								recommended on Android as it runs natively with instant boot and &lt; 60MB RAM.
							</p>
						)}
					</div>

					{/* Version & Port Row */}
					<div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
						{/* Version Picker */}
						<div className="flex flex-col gap-1.5">
							<span className="font-medium text-foreground text-xs">Minecraft Version</span>
							{isLoadingVersions ? (
								<div className="flex h-9 items-center gap-2 rounded-md border border-border/50 bg-zinc-900/40 px-3 text-muted-foreground text-xs">
									<Loader2 className="size-3.5 animate-spin text-emerald-400" />
									<span>Loading versions...</span>
								</div>
							) : (
								<Select
									value={selectedVersion}
									onValueChange={(val) => {
										if (val) setSelectedVersion(val)
									}}
								>
									<SelectTrigger className="h-9 text-xs">
										<SelectValue placeholder="Select version" />
									</SelectTrigger>
									<SelectContent className="max-h-60">
										{versions.map((ver) => (
											<SelectItem key={ver} value={ver} className="text-xs">
												{ver}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							)}
						</div>

						{/* Server Port */}
						<div className="flex flex-col gap-1.5">
							<div className="flex items-center justify-between">
								<label htmlFor="server-port" className="font-medium text-foreground text-xs">
									Server Port
								</label>
								{isPortConflict && (
									<span className="font-medium text-[11px] text-rose-400">Port already in use</span>
								)}
							</div>
							<Input
								id="server-port"
								type="number"
								min={1024}
								max={65535}
								value={port}
								onChange={(e) => setPort(Number(e.target.value) || 25565)}
								className={cn(
									"h-9 font-mono text-xs",
									isPortConflict && "border-rose-500 focus-visible:ring-rose-500/50",
								)}
							/>
						</div>
					</div>

					{/* RAM Allocation Slider */}
					<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/30 p-3.5">
						<div className="flex items-center justify-between">
							<div className="flex items-center gap-1.5 font-medium text-foreground text-xs">
								<Cpu className="size-3.5 text-emerald-400" />
								<span>Memory Allocation (RAM)</span>
							</div>
							<span className="font-bold font-mono text-emerald-400 text-xs">
								{(ramMb / 1024).toFixed(1)} GB ({ramMb} MB)
							</span>
						</div>
						<Slider
							min={1024}
							max={16384}
							step={512}
							value={[ramMb]}
							onValueChange={(val: number | readonly number[]) =>
								setRamMb(Array.isArray(val) ? val[0] : (val as number))
							}
							className="my-1"
						/>
						<p className="text-[11px] text-muted-foreground">
							Aikar's optimized G1GC flags will automatically be applied for top performance.
						</p>
					</div>

					{/* Error Message */}
					{error && (
						<div className="flex items-center gap-2 rounded-lg border border-rose-500/20 bg-rose-500/10 p-2.5 text-rose-300 text-xs">
							<AlertCircle className="size-4 shrink-0 text-rose-400" />
							<span>{error}</span>
						</div>
					)}
				</div>

				<DialogFooter className="gap-2 sm:gap-0">
					<Button
						variant="ghost"
						size="sm"
						onClick={() => onOpenChange(false)}
						disabled={isSubmitting}
						className="text-xs"
					>
						Cancel
					</Button>
					<Button
						size="sm"
						onClick={handleCreate}
						disabled={isSubmitting || !name.trim() || !selectedVersion || isPortConflict}
						className="gap-1.5 bg-emerald-600 text-white text-xs hover:bg-emerald-500"
					>
						{isSubmitting ? (
							<>
								<Loader2 className="size-3.5 animate-spin" />
								<span>Preparing Server...</span>
							</>
						) : (
							<>
								<Plus className="size-3.5" />
								<span>Create Server</span>
							</>
						)}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
