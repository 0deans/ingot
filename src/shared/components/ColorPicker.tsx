import {
	useCallback,
	useMemo,
	useRef,
	useState,
	type KeyboardEvent,
	type PointerEvent,
} from "react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"
import {
	hsvToRgb,
	parseCssColorToRgb,
	rgbToHex,
	rgbToHsv,
	type RgbColor,
} from "@/shared/theme/colorConversion"
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover"

interface ColorPickerProps {
	value: string
	fallback?: string
	label: string
	onChange: (hex: string) => void
}

const SWATCH_PALETTE: readonly string[] = [
	"#09090b",
	"#18181b",
	"#ffffff",
	"#f4f4f5",
	"#10b981",
	"#059669",
	"#38bdf8",
	"#a855f7",
	"#f43f5e",
	"#f59e0b",
]

const clamp = (value: number, min: number, max: number): number => {
	return Math.min(max, Math.max(min, value))
}

const ColorPicker = ({ value, fallback = "#000000", label, onChange }: ColorPickerProps) => {
	const { t } = useTranslation()

	const svAreaRef = useRef<HTMLDivElement | null>(null)
	const hueTrackRef = useRef<HTMLDivElement | null>(null)

	const [open, setOpen] = useState<boolean>(false)
	const [preservedHue, setPreservedHue] = useState<number>(() => {
		const initialRgb = parseCssColorToRgb(value) ??
			parseCssColorToRgb(fallback) ?? { r: 0, g: 0, b: 0 }
		return rgbToHsv(initialRgb).h
	})

	const resolvedRgb = useMemo<RgbColor>(() => {
		return parseCssColorToRgb(value) ?? parseCssColorToRgb(fallback) ?? { r: 0, g: 0, b: 0 }
	}, [fallback, value])

	const parsedHsv = useMemo(() => {
		return rgbToHsv(resolvedRgb)
	}, [resolvedRgb])

	const effectiveHue = useMemo(() => {
		if (parsedHsv.s === 0 || parsedHsv.v === 0) {
			return preservedHue
		}

		return parsedHsv.h
	}, [parsedHsv.h, parsedHsv.s, parsedHsv.v, preservedHue])

	const currentHex = useMemo(() => {
		return rgbToHex(resolvedRgb)
	}, [resolvedRgb])

	const commitHsv = useCallback(
		(nextH: number, nextS: number, nextV: number) => {
			const clampedH = clamp(nextH, 0, 360) % 360
			const clampedS = clamp(nextS, 0, 100)
			const clampedV = clamp(nextV, 0, 100)
			setPreservedHue(clampedH)
			const nextHex = rgbToHex(hsvToRgb({ h: clampedH, s: clampedS, v: clampedV }))
			onChange(nextHex)
		},
		[onChange],
	)

	const updateSvFromPointer = useCallback(
		(clientX: number, clientY: number) => {
			const area = svAreaRef.current
			if (!area) {
				return
			}
			const rect = area.getBoundingClientRect()
			if (rect.width <= 0 || rect.height <= 0) {
				return
			}
			const ratioX = clamp((clientX - rect.left) / rect.width, 0, 1)
			const ratioY = clamp((clientY - rect.top) / rect.height, 0, 1)
			commitHsv(effectiveHue, ratioX * 100, (1 - ratioY) * 100)
		},
		[commitHsv, effectiveHue],
	)

	const updateHueFromPointer = useCallback(
		(clientX: number) => {
			const track = hueTrackRef.current
			if (!track) {
				return
			}
			const rect = track.getBoundingClientRect()
			if (rect.width <= 0) {
				return
			}
			const ratioX = clamp((clientX - rect.left) / rect.width, 0, 1)
			const nextHue = Math.round(ratioX * 359)
			commitHsv(nextHue, parsedHsv.s, parsedHsv.v)
		},
		[commitHsv, parsedHsv.s, parsedHsv.v],
	)

	const handleSvPointerDown = useCallback(
		(event: PointerEvent<HTMLDivElement>) => {
			event.preventDefault()
			event.currentTarget.setPointerCapture(event.pointerId)
			updateSvFromPointer(event.clientX, event.clientY)
		},
		[updateSvFromPointer],
	)

	const handleSvPointerMove = useCallback(
		(event: PointerEvent<HTMLDivElement>) => {
			if (!event.currentTarget.hasPointerCapture(event.pointerId)) {
				return
			}
			updateSvFromPointer(event.clientX, event.clientY)
		},
		[updateSvFromPointer],
	)

	const handleHuePointerDown = useCallback(
		(event: PointerEvent<HTMLDivElement>) => {
			event.preventDefault()
			event.currentTarget.setPointerCapture(event.pointerId)
			updateHueFromPointer(event.clientX)
		},
		[updateHueFromPointer],
	)

	const handleHuePointerMove = useCallback(
		(event: PointerEvent<HTMLDivElement>) => {
			if (!event.currentTarget.hasPointerCapture(event.pointerId)) {
				return
			}
			updateHueFromPointer(event.clientX)
		},
		[updateHueFromPointer],
	)

	const handleSvKeyDown = useCallback(
		(event: KeyboardEvent<HTMLDivElement>) => {
			const step = event.shiftKey ? 10 : 2
			if (event.key === "ArrowLeft") {
				event.preventDefault()
				commitHsv(effectiveHue, parsedHsv.s - step, parsedHsv.v)
			} else if (event.key === "ArrowRight") {
				event.preventDefault()
				commitHsv(effectiveHue, parsedHsv.s + step, parsedHsv.v)
			} else if (event.key === "ArrowUp") {
				event.preventDefault()
				commitHsv(effectiveHue, parsedHsv.s, parsedHsv.v + step)
			} else if (event.key === "ArrowDown") {
				event.preventDefault()
				commitHsv(effectiveHue, parsedHsv.s, parsedHsv.v - step)
			}
		},
		[commitHsv, effectiveHue, parsedHsv.s, parsedHsv.v],
	)

	const handleHueKeyDown = useCallback(
		(event: KeyboardEvent<HTMLDivElement>) => {
			const step = event.shiftKey ? 15 : 3
			if (event.key === "ArrowLeft" || event.key === "ArrowDown") {
				event.preventDefault()
				commitHsv((effectiveHue - step + 360) % 360, parsedHsv.s, parsedHsv.v)
			} else if (event.key === "ArrowRight" || event.key === "ArrowUp") {
				event.preventDefault()
				commitHsv((effectiveHue + step) % 360, parsedHsv.s, parsedHsv.v)
			}
		},
		[commitHsv, effectiveHue, parsedHsv.s, parsedHsv.v],
	)

	const commitChannelByte = useCallback(
		(channel: keyof RgbColor, nextByte: number) => {
			const clamped = clamp(Math.round(nextByte), 0, 255)
			const nextRgb: RgbColor = { ...resolvedRgb, [channel]: clamped }
			const nextHsv = rgbToHsv(nextRgb)
			if (nextHsv.s > 0 && nextHsv.v > 0) {
				setPreservedHue(nextHsv.h)
			}
			onChange(rgbToHex(nextRgb))
		},
		[onChange, resolvedRgb],
	)

	const handleChannelChange = useCallback(
		(channel: keyof RgbColor, rawValue: string) => {
			const digitsOnly = rawValue.replace(/\D+/g, "")
			if (digitsOnly.length === 0) {
				commitChannelByte(channel, 0)
				return
			}
			const parsed = Number.parseInt(digitsOnly, 10)
			if (!Number.isNaN(parsed)) {
				commitChannelByte(channel, parsed)
			}
		},
		[commitChannelByte],
	)

	const handleChannelKeyDown = useCallback(
		(channel: keyof RgbColor, event: KeyboardEvent<HTMLInputElement>) => {
			const step = event.shiftKey ? 10 : 1
			if (event.key === "ArrowUp") {
				event.preventDefault()
				commitChannelByte(channel, resolvedRgb[channel] + step)
			} else if (event.key === "ArrowDown") {
				event.preventDefault()
				commitChannelByte(channel, resolvedRgb[channel] - step)
			}
		},
		[commitChannelByte, resolvedRgb],
	)

	const huePreviewColor = `hsl(${effectiveHue} 100% 50%)`

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger
				type="button"
				aria-label={t("settings.customTheme.picker.triggerAria", { label })}
				className="flex h-8 w-9 shrink-0 cursor-pointer items-center justify-center rounded-lg border border-border bg-background/70 p-1 transition-colors outline-none hover:border-foreground/30 focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40"
			>
				<span
					aria-hidden="true"
					className="h-full w-full rounded-[calc(var(--radius-sm)*0.7)] border border-foreground/15 shadow-xs"
					style={{ backgroundColor: currentHex }}
				/>
			</PopoverTrigger>

			<PopoverContent align="start" side="bottom" sideOffset={8} className="w-64 p-3">
				<div className="flex flex-col gap-3">
					<div className="flex items-center justify-between gap-2">
						<span className="truncate text-[11.5px] font-semibold text-foreground">{label}</span>
						<span className="font-mono text-[10.5px] text-muted-foreground">{currentHex}</span>
					</div>

					<div
						ref={svAreaRef}
						role="slider"
						tabIndex={0}
						aria-label={t("settings.customTheme.picker.svAria", { label })}
						aria-valuemin={0}
						aria-valuemax={100}
						aria-valuenow={Math.round(parsedHsv.s)}
						onPointerDown={handleSvPointerDown}
						onPointerMove={handleSvPointerMove}
						onKeyDown={handleSvKeyDown}
						style={{ backgroundColor: huePreviewColor }}
						className="relative h-36 w-full cursor-crosshair touch-none overflow-hidden rounded-lg border border-border outline-none select-none focus-visible:ring-2 focus-visible:ring-ring"
					>
						<div
							aria-hidden="true"
							className="pointer-events-none absolute inset-0"
							style={{ background: "linear-gradient(to right, #ffffff, transparent)" }}
						/>
						<div
							aria-hidden="true"
							className="pointer-events-none absolute inset-0"
							style={{ background: "linear-gradient(to top, #000000, transparent)" }}
						/>
						<span
							aria-hidden="true"
							style={{
								left: `${parsedHsv.s}%`,
								top: `${100 - parsedHsv.v}%`,
								backgroundColor: currentHex,
							}}
							className="pointer-events-none absolute size-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-md ring-1 ring-black/40"
						/>
					</div>

					<div className="flex items-center gap-2">
						<span
							aria-hidden="true"
							className="size-6 shrink-0 rounded-md border border-border shadow-xs"
							style={{ backgroundColor: currentHex }}
						/>

						<div
							ref={hueTrackRef}
							role="slider"
							tabIndex={0}
							aria-label={t("settings.customTheme.picker.hueAria", { label })}
							aria-valuemin={0}
							aria-valuemax={359}
							aria-valuenow={Math.round(effectiveHue)}
							onPointerDown={handleHuePointerDown}
							onPointerMove={handleHuePointerMove}
							onKeyDown={handleHueKeyDown}
							style={{
								background:
									"linear-gradient(to right, #ff0000, #ffff00, #00ff00, #00ffff, #0000ff, #ff00ff, #ff0000)",
							}}
							className="relative h-3 flex-1 cursor-pointer touch-none rounded-full border border-border outline-none select-none focus-visible:ring-2 focus-visible:ring-ring"
						>
							<span
								aria-hidden="true"
								style={{
									left: `${(effectiveHue / 359) * 100}%`,
									backgroundColor: huePreviewColor,
								}}
								className="pointer-events-none absolute top-1/2 size-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-md ring-1 ring-black/40"
							/>
						</div>
					</div>

					<div className="grid grid-cols-3 gap-1.5">
						{(["r", "g", "b"] as const).map((channel) => {
							return (
								<label
									key={channel}
									className="flex flex-col items-center gap-0.5 rounded-md border border-border bg-background/60 px-1.5 py-1 transition-colors focus-within:border-primary/60"
								>
									<input
										type="text"
										inputMode="numeric"
										pattern="[0-9]*"
										maxLength={3}
										aria-label={t("settings.customTheme.picker.channelAria", {
											label,
											channel: channel.toUpperCase(),
										})}
										value={String(resolvedRgb[channel])}
										onChange={(event) => {
											handleChannelChange(channel, event.currentTarget.value)
										}}
										onKeyDown={(event) => {
											handleChannelKeyDown(channel, event)
										}}
										className="w-full bg-transparent text-center font-mono text-[11px] font-medium text-foreground outline-none"
									/>
									<span className="text-[9.5px] font-semibold tracking-wider text-muted-foreground uppercase">
										{channel}
									</span>
								</label>
							)
						})}
					</div>

					<div className="grid grid-cols-10 gap-1">
						{SWATCH_PALETTE.map((swatchHex) => {
							const isCurrent = currentHex === swatchHex
							return (
								<button
									key={swatchHex}
									type="button"
									aria-label={t("settings.customTheme.picker.swatchAria", { hex: swatchHex })}
									onClick={() => {
										const rgb = parseCssColorToRgb(swatchHex)
										if (rgb) {
											const hsv = rgbToHsv(rgb)
											if (hsv.s > 0 && hsv.v > 0) {
												setPreservedHue(hsv.h)
											}
										}
										onChange(swatchHex)
									}}
									style={{ backgroundColor: swatchHex }}
									className={cn(
										"aspect-square w-full cursor-pointer rounded-sm border transition-transform hover:scale-110",
										isCurrent ? "border-primary ring-1 ring-primary" : "border-foreground/20",
									)}
								/>
							)
						})}
					</div>
				</div>
			</PopoverContent>
		</Popover>
	)
}

export default ColorPicker
