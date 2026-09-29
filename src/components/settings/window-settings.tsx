import { Check, Maximize2, Monitor, RotateCcw } from "lucide-react"
import { memo, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { SectionCardHeader } from "@/components/common/section-card"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
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
	const { t } = useTranslation()
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
		<Card>
			<SectionCardHeader
				icon={Monitor}
				title={t("settings.window.title")}
				description={t("settings.window.description")}
			/>
			<CardContent className="flex flex-col gap-4">
				{/* Fullscreen Option */}
				<div className="flex items-center justify-between rounded-lg border border-border/30 bg-background/60 p-3.5 transition-colors">
					<div className="flex items-center gap-3">
						<div className="flex size-8 items-center justify-center rounded-md bg-card text-info">
							<Maximize2 className="size-4" />
						</div>
						<div>
							<div className="font-medium text-foreground text-xs sm:text-sm">
								{t("settings.window.fullscreenTitle")}
							</div>
							<div className="text-[11px] text-muted-foreground">
								{t("settings.window.fullscreenDesc")}
							</div>
						</div>
					</div>

					<Switch checked={windowSettings.fullscreen} onCheckedChange={handleToggleFullscreen} />
				</div>

				{/* Resolution Option (when not in fullscreen) */}
				{!windowSettings.fullscreen && (
					<div className="flex flex-col gap-3 rounded-lg border border-border/30 bg-background/60 p-3.5">
						<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
							<div>
								<div className="font-medium text-foreground text-xs sm:text-sm">
									{t("settings.window.resolutionTitle")}
								</div>
								<div className="text-[11px] text-muted-foreground">
									{t("settings.window.resolutionDesc")}
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
										className="h-8 w-20 px-2 text-center font-mono text-xs"
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
										className="h-8 w-20 px-2 text-center font-mono text-xs"
									/>
								</div>
								<Button
									variant="ghost"
									size="sm"
									onClick={handleReset}
									title={t("settings.window.resetTo", { size: "854 × 480" })}
									className="h-8 px-2 text-muted-foreground"
								>
									<RotateCcw className="size-3.5" />
								</Button>
							</div>
						</div>

						{/* Quick Presets */}
						<div className="flex flex-wrap items-center gap-1.5 pt-1">
							<span className="mr-1 text-[11px] text-muted-foreground">
								{t("settings.window.presets")}
							</span>
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
												? "border-info/40 bg-info/10 font-medium text-info"
												: "border-border bg-card/80 text-foreground/80 hover:border-input hover:text-foreground"
										}`}
									>
										{isCurrent && <Check className="size-3" />}
										<span>{preset.label}</span>
										<span className="text-[10px] text-muted-foreground">
											(
											{preset.description === "Default"
												? t("settings.window.defaultPreset")
												: preset.description}
											)
										</span>
									</button>
								)
							})}
						</div>
					</div>
				)}
			</CardContent>
		</Card>
	)
}

WindowSettings.displayName = "WindowSettings"
export default memo(WindowSettings)
