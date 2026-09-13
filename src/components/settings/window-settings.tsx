import { Check, Maximize2, Monitor, RotateCcw } from "lucide-react"
import { memo, useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Switch } from "@/components/ui/switch"
import { useWindowSettings } from "@/services/settings-service"

const RESOLUTION_PRESETS = [
	{ label: "854 × 480", description: "Default", width: 854, height: 480 },
	{ label: "1024 × 768", description: "4:3", width: 1024, height: 768 },
	{ label: "1280 × 720", description: "720p HD", width: 1280, height: 720 },
	{ label: "1920 × 1080", description: "1080p FHD", width: 1920, height: 1080 },
	{ label: "2560 × 1440", description: "1440p QHD", width: 2560, height: 1440 },
]

export const WindowSettings = () => {
	const { windowSettings, setWindowSettings } = useWindowSettings()
	const [width, setWidth] = useState(windowSettings.width)
	const [height, setHeight] = useState(windowSettings.height)

	useEffect(() => {
		setWidth(windowSettings.width)
		setHeight(windowSettings.height)
	}, [windowSettings.width, windowSettings.height])

	const handleToggleFullscreen = async (checked: boolean) => {
		try {
			await setWindowSettings({
				...windowSettings,
				fullscreen: checked,
			})
		} catch (error) {
			console.error("Failed to update fullscreen setting:", error)
		}
	}

	const handleCommitResolution = async (newWidth: number, newHeight: number) => {
		const clampedW = Math.max(320, Math.min(newWidth, 7680))
		const clampedH = Math.max(240, Math.min(newHeight, 4320))
		setWidth(clampedW)
		setHeight(clampedH)
		try {
			await setWindowSettings({
				...windowSettings,
				width: clampedW,
				height: clampedH,
			})
		} catch (error) {
			console.error("Failed to update resolution setting:", error)
		}
	}

	const handleReset = () => {
		handleCommitResolution(854, 480)
	}

	return (
		<div className="flex flex-col gap-4 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
			<div className="flex flex-col gap-1">
				<div className="flex items-center gap-2">
					<Monitor className="size-4 text-sky-400" />
					<h3 className="font-semibold text-foreground text-sm">Window & Display</h3>
				</div>
				<p className="text-muted-foreground text-xs">
					Configure default window dimensions and fullscreen startup mode for your instances.
				</p>
			</div>

			{/* Fullscreen Option */}
			<div className="flex items-center justify-between rounded-lg border border-border/30 bg-zinc-950/60 p-3.5 transition-colors">
				<div className="flex items-center gap-3">
					<div className="flex size-8 items-center justify-center rounded-md bg-zinc-900 text-sky-400">
						<Maximize2 className="size-4" />
					</div>
					<div>
						<div className="font-medium text-foreground text-xs sm:text-sm">
							Start in Fullscreen
						</div>
						<div className="text-[11px] text-muted-foreground">
							Launch Minecraft directly into borderless or exclusive fullscreen
						</div>
					</div>
				</div>

				<Switch checked={windowSettings.fullscreen} onCheckedChange={handleToggleFullscreen} />
			</div>

			{/* Resolution Option (when not in fullscreen) */}
			{!windowSettings.fullscreen && (
				<div className="flex flex-col gap-3 rounded-lg border border-border/30 bg-zinc-950/60 p-3.5">
					<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
						<div>
							<div className="font-medium text-foreground text-xs sm:text-sm">
								Default Window Resolution
							</div>
							<div className="text-[11px] text-muted-foreground">
								Set custom width and height for game window
							</div>
						</div>

						{/* Inputs */}
						<div className="flex items-center gap-2">
							<div className="flex items-center gap-1.5">
								<span className="text-[11px] text-muted-foreground">W:</span>
								<Input
									type="number"
									value={width}
									onChange={(e) => setWidth(Number(e.target.value))}
									onBlur={() => handleCommitResolution(width, height)}
									onKeyDown={(e) => {
										if (e.key === "Enter") handleCommitResolution(width, height)
									}}
									className="h-8 w-20 border-zinc-800 bg-zinc-900 px-2 text-center font-mono text-xs"
								/>
							</div>
							<span className="text-muted-foreground text-xs">×</span>
							<div className="flex items-center gap-1.5">
								<span className="text-[11px] text-muted-foreground">H:</span>
								<Input
									type="number"
									value={height}
									onChange={(e) => setHeight(Number(e.target.value))}
									onBlur={() => handleCommitResolution(width, height)}
									onKeyDown={(e) => {
										if (e.key === "Enter") handleCommitResolution(width, height)
									}}
									className="h-8 w-20 border-zinc-800 bg-zinc-900 px-2 text-center font-mono text-xs"
								/>
							</div>
							<Button
								variant="ghost"
								size="sm"
								onClick={handleReset}
								title="Reset to 854 × 480"
								className="h-8 px-2 text-muted-foreground hover:text-foreground"
							>
								<RotateCcw className="size-3.5" />
							</Button>
						</div>
					</div>

					{/* Quick Presets */}
					<div className="flex flex-wrap items-center gap-1.5 pt-1">
						<span className="mr-1 text-[11px] text-muted-foreground">Presets:</span>
						{RESOLUTION_PRESETS.map((preset) => {
							const isCurrent =
								windowSettings.width === preset.width && windowSettings.height === preset.height
							return (
								<button
									key={preset.label}
									type="button"
									onClick={() => handleCommitResolution(preset.width, preset.height)}
									className={`flex items-center gap-1 rounded-md border px-2.5 py-1 text-[11px] transition-colors ${
										isCurrent
											? "border-sky-500/40 bg-sky-500/10 font-medium text-sky-400"
											: "border-zinc-800 bg-zinc-900/80 text-zinc-300 hover:border-zinc-700 hover:text-white"
									}`}
								>
									{isCurrent && <Check className="size-3" />}
									<span>{preset.label}</span>
									<span className="text-[10px] text-muted-foreground">({preset.description})</span>
								</button>
							)
						})}
					</div>
				</div>
			)}
		</div>
	)
}

WindowSettings.displayName = "WindowSettings"
export default memo(WindowSettings)
