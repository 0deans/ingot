import { AlertTriangle, Cpu, Info, Sparkles } from "lucide-react"
import { memo, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { alertTone } from "@/components/common/alert-tones"
import { SectionCardHeader } from "@/components/common/section-card"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Slider } from "@/components/ui/slider"
import { formatMegabytes, formatNumber } from "@/lib/format"
import { useMemorySettings } from "@/services/settings-service"

const STEP_MB = 256
const MIN_POSSIBLE_RAM_MB = 512

/** Gigabytes as a plain number ("4,5"), for texts that write the unit themselves */
function formatMbToGb(mb: number): string {
	return formatNumber(mb / 1024, { minimumFractionDigits: 1, maximumFractionDigits: 1 })
}

const MemoryAllocation = () => {
	const { t } = useTranslation()
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
		<Card>
			<SectionCardHeader
				icon={Cpu}
				title={t("settings.memory.title")}
				description={t("settings.memory.description")}
				action={
					<Badge variant="outline">
						<span className="font-medium">{formatMegabytes(totalRamMb)}</span>
						<span className="text-muted-foreground">{t("settings.memory.totalRam")}</span>
					</Badge>
				}
			/>
			<CardContent className="flex flex-col gap-4">
				{/* System Memory Status Banner */}
				<div className="flex items-center justify-between rounded-lg border border-border/40 bg-background/50 px-3 py-2 text-xs">
					<div className="flex items-center gap-2">
						<span className="relative flex size-2">
							<span className="absolute inline-flex size-full animate-ping rounded-full bg-primary opacity-75" />
							<span className="relative inline-flex size-2 rounded-full bg-primary" />
						</span>
						<span className="text-muted-foreground">
							{t("settings.memory.availableNow")}{" "}
							<strong className="text-foreground">{formatMegabytes(availableRamMb)}</strong>
							{" ("}
							{availableRamMb} MB{")"}
						</span>
					</div>
					<span className="font-mono text-2xs text-muted-foreground">
						{t("settings.memory.freePercent", { percent: Math.round(availablePercent) })}
					</span>
				</div>

				{/* Visual Allocation Scale Bar */}
				<div className="flex flex-col gap-1.5 pt-1">
					<div className="flex justify-between text-2xs">
						<span className="text-muted-foreground">{t("settings.memory.distribution")}</span>
						<span className="font-mono text-muted-foreground">
							{t("settings.memory.allocated")}{" "}
							<strong className="text-primary">{formatMegabytes(maxRam)}</strong> /{" "}
							{formatMegabytes(totalRamMb)}
						</span>
					</div>
					<div className="relative h-3 w-full overflow-hidden rounded-full bg-muted/80">
						{/* Safe 0-60% zone marker */}
						<div
							className="absolute inset-y-0 left-0 w-3/5 bg-primary/10"
							title={t("settings.memory.zoneSafe")}
						/>
						{/* Warning 60-80% zone marker */}
						<div
							className="absolute inset-y-0 left-3/5 w-1/5 bg-warning/10"
							title={t("settings.memory.zoneHigh")}
						/>
						{/* Danger >80% zone marker */}
						<div
							className="absolute inset-y-0 left-4/5 w-1/5 bg-destructive/10"
							title={t("settings.memory.zoneCritical")}
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
					<div className="flex justify-between font-mono text-3xs text-muted-foreground">
						<span>{formatMegabytes(512)}</span>
						<span>{t("settings.memory.minShort", { value: formatMegabytes(minRam) })}</span>
						<span>{t("settings.memory.maxShort", { value: formatMegabytes(maxRam) })}</span>
						<span>{t("settings.memory.totalShort", { value: formatMegabytes(totalRamMb) })}</span>
					</div>
				</div>

				{/* Dual Slider Control */}
				<div className="flex flex-col gap-2 pt-2">
					<div className="flex items-center justify-between text-xs">
						<span className="font-medium text-foreground">{t("settings.memory.sliderTitle")}</span>
						<span className="text-2xs text-muted-foreground">
							{t("settings.memory.sliderHint")}
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
					<div className="flex flex-col gap-1.5 rounded-lg border border-border/40 bg-background/40 p-3">
						<div className="flex items-center justify-between">
							<Label htmlFor="min-ram-input" className="font-medium text-foreground text-xs">
								{t("settings.memory.minMemoryLabel")}
							</Label>
							<span className="font-mono text-muted-foreground text-xs">
								{formatMegabytes(minRam)}
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
							<span className="text-muted-foreground text-xs">{t("settings.memory.mb")}</span>
						</div>
					</div>

					<div className="flex flex-col gap-1.5 rounded-lg border border-border/40 bg-background/40 p-3">
						<div className="flex items-center justify-between">
							<Label htmlFor="max-ram-input" className="font-medium text-foreground text-xs">
								{t("settings.memory.maxMemoryLabel")}
							</Label>
							<span className="font-mono text-muted-foreground text-xs">
								{formatMegabytes(maxRam)}
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
							<span className="text-muted-foreground text-xs">{t("settings.memory.mb")}</span>
						</div>
					</div>
				</div>

				{/* Safety Warnings */}
				{isHighRam && (
					<Alert className={alertTone.warning}>
						<AlertTriangle />
						<AlertTitle>{t("settings.memory.highMemoryWarningTitle")}</AlertTitle>
						<AlertDescription>
							{t("settings.memory.highMemoryWarningDesc", {
								maxGb: formatMbToGb(maxRam),
								totalGb: formatMbToGb(totalRamMb),
							})}
						</AlertDescription>
					</Alert>
				)}

				{!isHighRam && exceedsAvailable && (
					<Alert className={alertTone.info}>
						<Info />
						<AlertTitle>{t("settings.memory.memoryNoticeTitle")}</AlertTitle>
						<AlertDescription>
							{t("settings.memory.memoryNoticeDesc", {
								availableGb: formatMbToGb(availableRamMb),
							})}
						</AlertDescription>
					</Alert>
				)}

				{/* Quick Presets */}
				<div className="flex flex-col gap-2 pt-1">
					<div className="flex items-center gap-1 text-muted-foreground text-xs">
						<Sparkles className="size-3.5 text-primary" />
						<span className="font-medium text-foreground">
							{t("settings.memory.recommendedPresets")}
						</span>
					</div>
					<div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
						<Button
							variant="outline"
							size="xs"
							onClick={() => applyPreset(1024, 3072)}
							className="flex h-auto flex-col items-start gap-0.5 py-1.5 text-left"
						>
							<span className="font-semibold text-xs">{t("settings.memory.presets.vanilla")}</span>
							<span className="text-3xs text-muted-foreground">
								{formatMegabytes(1024)} &ndash; {formatMegabytes(3072)}
							</span>
						</Button>
						<Button
							variant="outline"
							size="xs"
							onClick={() => applyPreset(2048, 6144)}
							className="flex h-auto flex-col items-start gap-0.5 py-1.5 text-left"
						>
							<span className="font-semibold text-xs">{t("settings.memory.presets.modded")}</span>
							<span className="text-3xs text-muted-foreground">
								{formatMegabytes(2048)} &ndash; {formatMegabytes(6144)}
							</span>
						</Button>
						<Button
							variant="outline"
							size="xs"
							onClick={() => applyPreset(4096, 8192)}
							className="flex h-auto flex-col items-start gap-0.5 py-1.5 text-left"
						>
							<span className="font-semibold text-xs">{t("settings.memory.presets.heavy")}</span>
							<span className="text-3xs text-muted-foreground">
								{formatMegabytes(4096)} &ndash; {formatMegabytes(8192)}
							</span>
						</Button>
						<Button
							variant="outline"
							size="xs"
							onClick={() => applyPreset(2048, Math.floor((totalRamMb * 0.75) / 512) * 512)}
							className="flex h-auto flex-col items-start gap-0.5 py-1.5 text-left"
						>
							<span className="font-semibold text-xs">{t("settings.memory.presets.safeMax")}</span>
							<span className="text-3xs text-muted-foreground">
								{formatMegabytes(2048)} &ndash;{" "}
								{formatMegabytes(Math.floor((totalRamMb * 0.75) / 512) * 512)}
							</span>
						</Button>
					</div>
				</div>
			</CardContent>
		</Card>
	)
}

MemoryAllocation.displayName = "MemoryAllocation"

export default memo(MemoryAllocation)
