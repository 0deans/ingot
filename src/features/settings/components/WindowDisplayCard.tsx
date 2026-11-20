import { Maximize2, Monitor, RotateCcw } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/shared/lib/utils"
import { DEFAULT_WINDOW_PREFS, useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { Button } from "@/shared/ui/button"
import { Input } from "@/shared/ui/input"
import { Switch } from "@/shared/ui/switch"

interface ResolutionPreset {
	width: number
	height: number
	tag?: string
	isDefault?: boolean
}

const RESOLUTION_PRESETS: readonly ResolutionPreset[] = [
	{ width: 854, height: 480, isDefault: true },
	{ width: 1024, height: 768, tag: "4:3" },
	{ width: 1280, height: 720, tag: "720p HD" },
	{ width: 1920, height: 1080, tag: "1080p FHD" },
	{ width: 2560, height: 1440, tag: "1440p QHD" },
]

const MIN_WIDTH = 320
const MAX_WIDTH = 7680
const MIN_HEIGHT = 240
const MAX_HEIGHT = 4320

const clampDimension = (raw: string, min: number, max: number, fallback: number): number => {
	const parsed = Number.parseInt(raw, 10)
	if (!Number.isFinite(parsed)) {
		return fallback
	}
	return Math.min(max, Math.max(min, parsed))
}

const WindowDisplayCard = () => {
	const { t } = useTranslation()
	const { windowSettings, setWindowSettings } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				windowSettings: s.windowSettings,
				setWindowSettings: s.setWindowSettings,
			}
		}),
	)

	const [draftWidth, setDraftWidth] = useState<string | null>(null)
	const [draftHeight, setDraftHeight] = useState<string | null>(null)

	const widthInput = draftWidth ?? String(windowSettings.width)
	const heightInput = draftHeight ?? String(windowSettings.height)

	const handleToggleFullscreen = () => {
		setWindowSettings({
			...windowSettings,
			fullscreen: !windowSettings.fullscreen,
		})
	}

	const commitDimensions = (nextWidthRaw: string, nextHeightRaw: string) => {
		const nextWidth = clampDimension(nextWidthRaw, MIN_WIDTH, MAX_WIDTH, windowSettings.width)
		const nextHeight = clampDimension(nextHeightRaw, MIN_HEIGHT, MAX_HEIGHT, windowSettings.height)
		setDraftWidth(null)
		setDraftHeight(null)
		setWindowSettings({
			...windowSettings,
			width: nextWidth,
			height: nextHeight,
		})
	}

	const handleSelectPreset = (preset: ResolutionPreset) => {
		setDraftWidth(null)
		setDraftHeight(null)
		setWindowSettings({
			...windowSettings,
			width: preset.width,
			height: preset.height,
		})
	}

	const handleResetResolution = () => {
		setDraftWidth(null)
		setDraftHeight(null)
		setWindowSettings({
			...windowSettings,
			width: DEFAULT_WINDOW_PREFS.width,
			height: DEFAULT_WINDOW_PREFS.height,
		})
	}

	const isDefaultResolution =
		windowSettings.width === DEFAULT_WINDOW_PREFS.width &&
		windowSettings.height === DEFAULT_WINDOW_PREFS.height

	return (
		<section className="@container flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-3.5 backdrop-blur-md sm:p-4">
			<div className="flex items-start gap-2.5">
				<div className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg border border-border bg-foreground/5 text-primary">
					<Monitor className="size-4" />
				</div>
				<div className="min-w-0">
					<h2 className="text-[13.5px] font-semibold text-foreground">
						{t("settings.general.window.title")}
					</h2>
					<p className="text-[11.5px] leading-relaxed text-muted-foreground">
						{t("settings.general.window.description")}
					</p>
				</div>
			</div>

			<div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="flex min-w-0 flex-1 items-center gap-3">
					<div className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-border bg-foreground/5 text-primary">
						<Maximize2 className="size-4" />
					</div>
					<div className="min-w-0">
						<p className="text-[12.5px] font-medium text-foreground">
							{t("settings.general.window.fullscreenTitle")}
						</p>
						<p className="text-[11px] leading-relaxed text-muted-foreground">
							{t("settings.general.window.fullscreenDescription")}
						</p>
					</div>
				</div>

				<Switch
					checked={windowSettings.fullscreen}
					onCheckedChange={handleToggleFullscreen}
					aria-label={t("settings.general.window.fullscreenTitle")}
				/>
			</div>

			<div className="flex flex-col gap-3 rounded-xl border border-border bg-foreground/3 p-3.5">
				<div className="flex flex-col gap-3 @lg:flex-row @lg:items-center @lg:justify-between">
					<div className="min-w-0 flex-1">
						<p className="text-[12.5px] font-medium text-foreground">
							{t("settings.general.window.resolutionTitle")}
						</p>
						<p className="text-[11px] leading-relaxed text-muted-foreground">
							{t("settings.general.window.resolutionDescription")}
						</p>
					</div>

					<div className="flex flex-wrap items-center gap-2">
						<div className="flex items-center gap-1.5">
							<label className="flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground">
								<span>W</span>
								<Input
									type="number"
									min={MIN_WIDTH}
									max={MAX_WIDTH}
									value={widthInput}
									aria-label={t("settings.general.window.widthAriaLabel")}
									onChange={(event) => {
										setDraftWidth(event.target.value)
									}}
									onBlur={() => {
										commitDimensions(widthInput, heightInput)
									}}
									onKeyDown={(event) => {
										if (event.key === "Enter") {
											commitDimensions(widthInput, heightInput)
										}
									}}
									className="w-20 [appearance:textfield] rounded-md text-center font-mono text-[12px] font-medium md:text-[12px] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
								/>
							</label>

							<span className="text-[11px] text-muted-foreground">×</span>

							<label className="flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground">
								<span>H</span>
								<Input
									type="number"
									min={MIN_HEIGHT}
									max={MAX_HEIGHT}
									value={heightInput}
									aria-label={t("settings.general.window.heightAriaLabel")}
									onChange={(event) => {
										setDraftHeight(event.target.value)
									}}
									onBlur={() => {
										commitDimensions(widthInput, heightInput)
									}}
									onKeyDown={(event) => {
										if (event.key === "Enter") {
											commitDimensions(widthInput, heightInput)
										}
									}}
									className="w-20 [appearance:textfield] rounded-md text-center font-mono text-[12px] font-medium md:text-[12px] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
								/>
							</label>
						</div>

						<Button
							type="button"
							variant="outline"
							size="icon"
							disabled={isDefaultResolution}
							onClick={handleResetResolution}
							aria-label={t("settings.general.window.resetAriaLabel")}
							title={t("settings.general.window.resetAriaLabel")}
							className="cursor-pointer rounded-md"
						>
							<RotateCcw className="size-3.5" />
						</Button>
					</div>
				</div>

				<div className="flex flex-col gap-2 border-t border-border/60 pt-2.5">
					<span className="text-[11px] font-medium text-muted-foreground">
						{t("settings.general.window.presetsLabel")}
					</span>

					<div className="grid grid-cols-2 gap-2 @md:grid-cols-3 @2xl:grid-cols-5">
						{RESOLUTION_PRESETS.map((preset) => {
							const isSelected =
								windowSettings.width === preset.width && windowSettings.height === preset.height
							const tagLabel = preset.isDefault
								? t("settings.general.window.presetDefault")
								: preset.tag

							return (
								<button
									key={`${preset.width}x${preset.height}`}
									type="button"
									onClick={() => {
										handleSelectPreset(preset)
									}}
									className={cn(
										"flex cursor-pointer flex-col items-start gap-0.5 rounded-md border px-2.5 py-2 text-left transition-colors last:col-span-2 @md:last:col-span-1",
										isSelected
											? "border-primary/50 bg-primary/12 text-foreground"
											: "border-border bg-foreground/4 text-foreground/85 hover:bg-foreground/8",
									)}
								>
									<span className="font-mono text-[12px] font-semibold">
										{preset.width} × {preset.height}
									</span>
									{tagLabel ? (
										<span
											className={cn(
												"text-[10.5px]",
												isSelected ? "text-primary" : "text-muted-foreground",
											)}
										>
											{tagLabel}
										</span>
									) : null}
								</button>
							)
						})}
					</div>
				</div>
			</div>
		</section>
	)
}

export default WindowDisplayCard
