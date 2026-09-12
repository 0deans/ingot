import { AlertTriangle, Cpu, Info, Sparkles } from "lucide-react"
import { memo, useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import Slider from "@/components/ui/slider"
import { useMemorySettings } from "@/services/settings-service"

const STEP_MB = 256
const MIN_POSSIBLE_RAM_MB = 512

function formatMbToGb(mb: number): string {
	return (mb / 1024).toFixed(1)
}

const MemoryAllocation = () => {
	const { memory, systemMemory, setMemorySettings } = useMemorySettings()

	const [minRam, setMinRam] = useState(memory.minRamMb)
	const [maxRam, setMaxRam] = useState(memory.maxRamMb)

	useEffect(() => {
		setMinRam(memory.minRamMb)
		setMaxRam(memory.maxRamMb)
	}, [memory.minRamMb, memory.maxRamMb])

	const totalRamMb = systemMemory?.totalMb || 16384
	const availableRamMb = systemMemory?.availableMb || 10240
	const maxSliderLimit = totalRamMb

	const handleSliderChange = (values: number | readonly number[]) => {
		if (Array.isArray(values) && values.length >= 2) {
			const newMin = Math.round(values[0] / STEP_MB) * STEP_MB
			const newMax = Math.round(values[1] / STEP_MB) * STEP_MB
			setMinRam(newMin)
			setMaxRam(newMax)
		}
	}

	const handleSliderCommit = (values: number | readonly number[]) => {
		if (Array.isArray(values) && values.length >= 2) {
			const newMin = Math.round(values[0] / STEP_MB) * STEP_MB
			const newMax = Math.round(values[1] / STEP_MB) * STEP_MB
			setMemorySettings(newMin, newMax).catch((err) => {
				console.error("Failed to save memory settings:", err)
			})
		}
	}

	const handleMinInputChange = (val: number) => {
		const clamped = Math.max(MIN_POSSIBLE_RAM_MB, Math.min(val, maxRam))
		setMinRam(clamped)
		setMemorySettings(clamped, maxRam).catch(console.error)
	}

	const handleMaxInputChange = (val: number) => {
		const clamped = Math.max(minRam, Math.min(val, totalRamMb))
		setMaxRam(clamped)
		setMemorySettings(minRam, clamped).catch(console.error)
	}

	const applyPreset = (presetMin: number, presetMax: number) => {
		const safeMax = Math.min(presetMax, totalRamMb)
		const safeMin = Math.min(presetMin, safeMax)
		setMinRam(safeMin)
		setMaxRam(safeMax)
		setMemorySettings(safeMin, safeMax).catch(console.error)
	}

	const isHighRam = maxRam > totalRamMb * 0.8
	const exceedsAvailable = maxRam > availableRamMb
	const minPercent = Math.min(100, (minRam / totalRamMb) * 100)
	const maxPercent = Math.min(100, (maxRam / totalRamMb) * 100)
	const availablePercent = Math.min(100, (availableRamMb / totalRamMb) * 100)

	return (
		<div className="flex flex-col gap-4 rounded-xl border border-border/40 bg-zinc-900/40 p-5">
			<div className="flex items-start justify-between">
				<div>
					<h3 className="font-semibold text-foreground text-sm">Memory Allocation (RAM)</h3>
					<p className="text-muted-foreground text-xs">
						Configure initial (Min) and maximum (Max) RAM limits for Java and Minecraft instances.
					</p>
				</div>
				<div className="flex items-center gap-1.5 rounded-md border border-border/60 bg-zinc-950/80 px-2.5 py-1 text-xs">
					<Cpu className="size-3.5 text-primary" />
					<span className="font-medium text-foreground">{formatMbToGb(totalRamMb)} GB</span>
					<span className="text-muted-foreground">Total RAM</span>
				</div>
			</div>

			{/* System Memory Status Banner */}
			<div className="flex items-center justify-between rounded-lg border border-border/40 bg-zinc-950/50 px-3 py-2 text-xs">
				<div className="flex items-center gap-2">
					<span className="relative flex size-2">
						<span className="absolute inline-flex size-full animate-ping rounded-full bg-emerald-400 opacity-75" />
						<span className="relative inline-flex size-2 rounded-full bg-emerald-500" />
					</span>
					<span className="text-muted-foreground">
						Available machine memory right now:{" "}
						<strong className="text-foreground">{formatMbToGb(availableRamMb)} GB</strong>
						{" ("}
						{availableRamMb} MB{")"}
					</span>
				</div>
				<span className="font-mono text-[11px] text-muted-foreground">
					{Math.round(availablePercent)}% Free
				</span>
			</div>

			{/* Visual Allocation Scale Bar */}
			<div className="flex flex-col gap-1.5 pt-1">
				<div className="flex justify-between text-[11px]">
					<span className="text-muted-foreground">Machine RAM Distribution</span>
					<span className="font-mono text-muted-foreground">
						Allocated: <strong className="text-primary">{formatMbToGb(maxRam)} GB</strong> /{" "}
						{formatMbToGb(totalRamMb)} GB
					</span>
				</div>
				<div className="relative h-3 w-full overflow-hidden rounded-full bg-zinc-800/80">
					{/* Safe 0-60% zone marker */}
					<div
						className="absolute inset-y-0 left-0 bg-emerald-500/10"
						style={{ width: "60%" }}
						title="Safe Operating Zone"
					/>
					{/* Warning 60-80% zone marker */}
					<div
						className="absolute inset-y-0 left-[60%] bg-amber-500/10"
						style={{ width: "20%" }}
						title="High Usage Zone"
					/>
					{/* Danger >80% zone marker */}
					<div
						className="absolute inset-y-0 left-[80%] bg-rose-500/10"
						style={{ width: "20%" }}
						title="System Critical Zone"
					/>

					{/* Active Min-Max allocation range indicator */}
					<div
						className="absolute inset-y-0 rounded-full bg-primary/75 shadow-xs"
						style={{
							left: `${minPercent}%`,
							width: `${Math.max(1, maxPercent - minPercent)}%`,
						}}
					/>
				</div>
				<div className="flex justify-between font-mono text-[10px] text-muted-foreground">
					<span>512 MB</span>
					<span>Min: {formatMbToGb(minRam)} GB</span>
					<span>Max: {formatMbToGb(maxRam)} GB</span>
					<span>{formatMbToGb(totalRamMb)} GB (Max)</span>
				</div>
			</div>

			{/* Dual Slider Control */}
			<div className="flex flex-col gap-2 pt-2">
				<div className="flex items-center justify-between text-xs">
					<span className="font-medium text-foreground">Range Allocation Slider</span>
					<span className="text-[11px] text-muted-foreground">
						Thumb 1: Min (-Xms) &bull; Thumb 2: Max (-Xmx)
					</span>
				</div>
				<Slider
					value={[minRam, maxRam]}
					min={MIN_POSSIBLE_RAM_MB}
					max={maxSliderLimit}
					step={STEP_MB}
					minStepsBetweenValues={1}
					onValueChange={handleSliderChange}
					onValueCommitted={handleSliderCommit}
					className="py-1"
				/>
			</div>

			{/* Direct Numeric Inputs */}
			<div className="grid grid-cols-1 gap-4 pt-1 sm:grid-cols-2">
				<div className="flex flex-col gap-1.5 rounded-lg border border-border/40 bg-zinc-950/40 p-3">
					<div className="flex items-center justify-between">
						<label htmlFor="min-ram-input" className="font-medium text-foreground text-xs">
							Initial Memory (Min / -Xms)
						</label>
						<span className="font-mono text-muted-foreground text-xs">
							{formatMbToGb(minRam)} GB
						</span>
					</div>
					<div className="flex items-center gap-2">
						<Input
							id="min-ram-input"
							type="number"
							value={minRam}
							step={256}
							min={MIN_POSSIBLE_RAM_MB}
							max={maxRam}
							onChange={(e) => handleMinInputChange(Number(e.target.value))}
							className="font-mono text-xs"
						/>
						<span className="text-muted-foreground text-xs">MB</span>
					</div>
				</div>

				<div className="flex flex-col gap-1.5 rounded-lg border border-border/40 bg-zinc-950/40 p-3">
					<div className="flex items-center justify-between">
						<label htmlFor="max-ram-input" className="font-medium text-foreground text-xs">
							Maximum Memory (Max / -Xmx)
						</label>
						<span className="font-mono text-muted-foreground text-xs">
							{formatMbToGb(maxRam)} GB
						</span>
					</div>
					<div className="flex items-center gap-2">
						<Input
							id="max-ram-input"
							type="number"
							value={maxRam}
							step={256}
							min={minRam}
							max={totalRamMb}
							onChange={(e) => handleMaxInputChange(Number(e.target.value))}
							className="font-mono text-xs"
						/>
						<span className="text-muted-foreground text-xs">MB</span>
					</div>
				</div>
			</div>

			{/* Safety Warnings */}
			{isHighRam && (
				<div className="flex items-start gap-2.5 rounded-lg border border-amber-500/20 bg-amber-500/10 p-2.5 text-amber-400 text-xs">
					<AlertTriangle className="mt-0.5 size-4 shrink-0" />
					<div className="flex flex-col gap-0.5">
						<span className="font-semibold">High Memory Warning</span>
						<span className="text-[11px] text-amber-300/90">
							Allocating {formatMbToGb(maxRam)} GB (&gt; 80% of your {formatMbToGb(totalRamMb)} GB
							total RAM) may cause background applications, Discord, or Windows to become
							unresponsive.
						</span>
					</div>
				</div>
			)}

			{!isHighRam && exceedsAvailable && (
				<div className="flex items-start gap-2.5 rounded-lg border border-sky-500/20 bg-sky-500/10 p-2.5 text-sky-400 text-xs">
					<Info className="mt-0.5 size-4 shrink-0" />
					<div className="flex flex-col gap-0.5">
						<span className="font-semibold">Memory Notice</span>
						<span className="text-[11px] text-sky-300/90">
							Max RAM is greater than currently free memory ({formatMbToGb(availableRamMb)} GB).
							Ensure heavy background apps are closed before launching heavy instances.
						</span>
					</div>
				</div>
			)}

			{/* Quick Presets */}
			<div className="flex flex-col gap-2 pt-1">
				<div className="flex items-center gap-1 text-muted-foreground text-xs">
					<Sparkles className="size-3.5 text-primary" />
					<span className="font-medium text-foreground">Recommended Presets:</span>
				</div>
				<div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
					<Button
						variant="outline"
						size="xs"
						onClick={() => applyPreset(1024, 3072)}
						className="flex h-auto flex-col items-start gap-0.5 py-1.5 text-left"
					>
						<span className="font-semibold text-xs">Vanilla</span>
						<span className="text-[10px] text-muted-foreground">1.0 GB &ndash; 3.0 GB</span>
					</Button>
					<Button
						variant="outline"
						size="xs"
						onClick={() => applyPreset(2048, 6144)}
						className="flex h-auto flex-col items-start gap-0.5 py-1.5 text-left"
					>
						<span className="font-semibold text-xs">Modded</span>
						<span className="text-[10px] text-muted-foreground">2.0 GB &ndash; 6.0 GB</span>
					</Button>
					<Button
						variant="outline"
						size="xs"
						onClick={() => applyPreset(4096, 8192)}
						className="flex h-auto flex-col items-start gap-0.5 py-1.5 text-left"
					>
						<span className="font-semibold text-xs">Heavy Pack</span>
						<span className="text-[10px] text-muted-foreground">4.0 GB &ndash; 8.0 GB</span>
					</Button>
					<Button
						variant="outline"
						size="xs"
						onClick={() => applyPreset(2048, Math.floor((totalRamMb * 0.75) / 512) * 512)}
						className="flex h-auto flex-col items-start gap-0.5 py-1.5 text-left"
					>
						<span className="font-semibold text-xs">Safe Max</span>
						<span className="text-[10px] text-muted-foreground">
							2.0 GB &ndash; {formatMbToGb(Math.floor((totalRamMb * 0.75) / 512) * 512)} GB
						</span>
					</Button>
				</div>
			</div>
		</div>
	)
}

MemoryAllocation.displayName = "MemoryAllocation"

export default memo(MemoryAllocation)
