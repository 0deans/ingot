import { Check, ClipboardPaste, Copy, RotateCcw, SlidersHorizontal } from "lucide-react"
import { useCallback, useEffect, useState } from "react"
import { toast } from "sonner"
import { useShallow } from "zustand/react/shallow"
import CustomThemeColorField from "@/features/settings/components/CustomThemeColorField"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import {
	CUSTOM_THEME_FIELDS,
	THEME_PRESETS,
	parseCustomThemeJson,
	stringifyCustomThemeJson,
	type AppThemeId,
	type CustomThemeBaseMode,
	type CustomThemeColors,
} from "@/shared/theme/customTheme"

const BASE_MODE_OPTIONS: SegmentedOption<CustomThemeBaseMode>[] = [
	{ value: "dark", label: "Dark Base", dotClass: "bg-zinc-900" },
	{ value: "light", label: "Light Base", dotClass: "bg-zinc-100" },
]

const RADIUS_PRESETS: readonly number[] = [0, 0.35, 0.625, 0.75, 1.0]

const SEED_PRESETS: readonly { id: Exclude<AppThemeId, "custom">; label: string }[] = [
	{ id: "dark", label: "Dark" },
	{ id: "light", label: "Light" },
	{ id: "purple", label: "Purple" },
]

const CustomThemeEditor = () => {
	const {
		customTheme,
		setCustomTheme,
		updateCustomThemeColor,
		setCustomThemeBaseMode,
		setCustomThemeRadius,
		resetCustomTheme,
	} = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				customTheme: s.customTheme,
				setCustomTheme: s.setCustomTheme,
				updateCustomThemeColor: s.updateCustomThemeColor,
				setCustomThemeBaseMode: s.setCustomThemeBaseMode,
				setCustomThemeRadius: s.setCustomThemeRadius,
				resetCustomTheme: s.resetCustomTheme,
			}
		}),
	)

	const [jsonEditDraft, setJsonEditDraft] = useState<string | null>(null)
	const [jsonErrors, setJsonErrors] = useState<string[]>([])
	const [copiedJson, setCopiedJson] = useState<boolean>(false)
	const [fieldResetEpoch, setFieldResetEpoch] = useState<number>(0)

	const displayedJson = jsonEditDraft ?? stringifyCustomThemeJson(customTheme)

	const handleCommitColor = useCallback(
		(key: keyof CustomThemeColors, value: string) => {
			updateCustomThemeColor(key, value)
			setJsonEditDraft(null)
			setJsonErrors([])
		},
		[updateCustomThemeColor],
	)

	const resetBulkDrafts = useCallback(() => {
		setFieldResetEpoch((prev) => {
			return prev + 1
		})
		setJsonEditDraft(null)
		setJsonErrors([])
	}, [])

	const handleBaseModeChange = useCallback(
		(baseMode: CustomThemeBaseMode) => {
			setCustomThemeBaseMode(baseMode)
			setJsonEditDraft(null)
			setJsonErrors([])
		},
		[setCustomThemeBaseMode],
	)

	const handleRadiusChange = useCallback(
		(radiusRem: number) => {
			setCustomThemeRadius(radiusRem)
			setJsonEditDraft(null)
			setJsonErrors([])
		},
		[setCustomThemeRadius],
	)

	const handleCopyJson = useCallback(async () => {
		const payload = stringifyCustomThemeJson(customTheme)
		try {
			await navigator.clipboard.writeText(payload)
			setCopiedJson(true)
			toast.success("Custom theme JSON copied to clipboard.")
		} catch {
			toast.error("Clipboard write failed. Copy from the JSON editor below.")
		}
	}, [customTheme])

	const handlePasteClipboard = useCallback(async () => {
		try {
			const clipboardText = await navigator.clipboard.readText()
			if (!clipboardText.trim()) {
				toast.info("Clipboard is empty. Paste JSON directly into the box below.")
				return
			}
			const parsed = parseCustomThemeJson(clipboardText)
			if (!parsed.ok) {
				setJsonEditDraft(clipboardText)
				setJsonErrors(parsed.errors)
				toast.error("Clipboard JSON has validation errors.")
				return
			}
			setCustomTheme(parsed.value)
			resetBulkDrafts()
			toast.success("Applied custom theme from clipboard JSON.")
		} catch {
			toast.info("Clipboard read blocked. Paste JSON manually into the editor box.")
		}
	}, [resetBulkDrafts, setCustomTheme])

	const handleApplyJson = useCallback(() => {
		const parsed = parseCustomThemeJson(displayedJson)
		if (!parsed.ok) {
			setJsonErrors(parsed.errors)
			toast.error("Custom theme JSON is invalid.")
			return
		}
		setCustomTheme(parsed.value)
		resetBulkDrafts()
		toast.success("Custom theme JSON applied.")
	}, [displayedJson, resetBulkDrafts, setCustomTheme])

	const handleSeedPreset = useCallback(
		(presetId: Exclude<AppThemeId, "custom">) => {
			resetCustomTheme(presetId)
			resetBulkDrafts()
			toast.success(`Loaded ${presetId} preset into Custom theme.`)
		},
		[resetBulkDrafts, resetCustomTheme],
	)

	const handleReset = useCallback(() => {
		resetCustomTheme()
		resetBulkDrafts()
		toast.success("Reset custom theme to default preset.")
	}, [resetBulkDrafts, resetCustomTheme])

	useEffect(() => {
		if (!copiedJson) {
			return
		}
		const timer = window.setTimeout(() => {
			setCopiedJson(false)
		}, 1600)
		return () => {
			window.clearTimeout(timer)
		}
	}, [copiedJson])

	const fallbackPreset = THEME_PRESETS[customTheme.baseMode]

	return (
		<section className="flex flex-col gap-5 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div className="flex min-w-0 items-center gap-2">
					<SlidersHorizontal className="size-4 shrink-0 text-primary" />
					<div className="min-w-0">
						<h2 className="text-[13.5px] font-semibold text-foreground">Custom Theme Builder</h2>
						<p className="text-[11.5px] text-muted-foreground">
							Tune color tokens live or import/export portable JSON themes.
						</p>
					</div>
				</div>

				<div className="flex flex-wrap items-center gap-1.5">
					<span className="mr-1 text-[11px] font-medium text-muted-foreground">Seed preset:</span>
					{SEED_PRESETS.map((preset) => {
						return (
							<button
								key={preset.id}
								type="button"
								onClick={() => {
									handleSeedPreset(preset.id)
								}}
								className="cursor-pointer rounded-lg border border-border bg-foreground/5 px-2.5 py-1 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/10"
							>
								{preset.label}
							</button>
						)
					})}
					<button
						type="button"
						onClick={handleReset}
						className="inline-flex cursor-pointer items-center gap-1 rounded-lg border border-border bg-foreground/5 px-2.5 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
					>
						<RotateCcw className="size-3" />
						<span>Reset</span>
					</button>
				</div>
			</div>

			<div className="grid grid-cols-1 gap-3 rounded-xl border border-border bg-foreground/3 p-3.5 md:grid-cols-2">
				<div className="flex flex-col gap-2">
					<span className="text-[11.5px] font-medium text-foreground">Base Contrast Mode</span>
					<div>
						<SegmentedControl
							layoutId="custom-theme-base-mode"
							options={BASE_MODE_OPTIONS}
							value={customTheme.baseMode}
							onChange={handleBaseModeChange}
							tone="emerald"
						/>
					</div>
					<p className="text-[11px] text-muted-foreground">
						Controls adaptive foreground ink, glass highlights, and toast contrast.
					</p>
				</div>

				<div className="flex flex-col gap-2">
					<div className="flex items-center justify-between gap-2">
						<label
							htmlFor="custom-theme-radius"
							className="text-[11.5px] font-medium text-foreground"
						>
							Corner Radius
						</label>
						<span className="font-mono text-[11px] font-semibold text-primary">
							{customTheme.radiusRem.toFixed(2)}rem
						</span>
					</div>
					<input
						id="custom-theme-radius"
						type="range"
						min={0}
						max={1.25}
						step={0.05}
						value={customTheme.radiusRem}
						onChange={(event) => {
							const parsed = Number.parseFloat(event.currentTarget.value)
							if (!Number.isNaN(parsed)) {
								handleRadiusChange(parsed)
							}
						}}
						className="h-1.5 w-full cursor-pointer rounded-full bg-foreground/15 accent-primary"
					/>
					<div className="flex flex-wrap gap-1.5">
						{RADIUS_PRESETS.map((radiusValue) => {
							const isSelected = Math.abs(customTheme.radiusRem - radiusValue) < 0.02
							return (
								<button
									key={radiusValue}
									type="button"
									onClick={() => {
										handleRadiusChange(radiusValue)
									}}
									className={cn(
										"cursor-pointer rounded-md border px-2 py-0.5 font-mono text-[10.5px] transition-colors",
										isSelected
											? "border-primary/45 bg-primary/15 text-primary"
											: "border-border bg-foreground/4 text-muted-foreground hover:text-foreground",
									)}
								>
									{radiusValue}rem
								</button>
							)
						})}
					</div>
				</div>
			</div>

			<div className="flex flex-col gap-2.5">
				<h3 className="text-[12px] font-semibold tracking-wide text-muted-foreground uppercase">
					Color Tokens
				</h3>
				<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-3">
					{CUSTOM_THEME_FIELDS.map((field) => {
						return (
							<CustomThemeColorField
								key={field.key}
								field={field}
								committedValue={customTheme.colors[field.key]}
								fallbackValue={fallbackPreset.colors[field.key]}
								resetEpoch={fieldResetEpoch}
								onCommitColor={handleCommitColor}
							/>
						)
					})}
				</div>
			</div>

			<div className="flex flex-col gap-2.5 rounded-xl border border-border bg-foreground/3 p-3.5">
				<div className="flex flex-wrap items-center justify-between gap-3">
					<div className="min-w-0 flex-1">
						<h3 className="text-[12.5px] font-semibold text-foreground">JSON Import / Export</h3>
						<p className="text-[11px] text-muted-foreground">
							Copy your current configuration or paste shared JSON styles to apply them.
						</p>
					</div>

					<div className="flex flex-wrap items-center gap-1.5">
						<button
							type="button"
							onClick={() => {
								void handleCopyJson()
							}}
							className="inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-foreground/6 px-2.5 py-1.5 text-[11.5px] font-medium whitespace-nowrap text-foreground transition-colors hover:bg-foreground/12"
						>
							{copiedJson ? (
								<Check className="size-3.5 text-primary" />
							) : (
								<Copy className="size-3.5 text-muted-foreground" />
							)}
							<span>{copiedJson ? "Copied" : "Copy JSON"}</span>
						</button>

						<button
							type="button"
							onClick={() => {
								void handlePasteClipboard()
							}}
							className="inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-foreground/6 px-2.5 py-1.5 text-[11.5px] font-medium whitespace-nowrap text-foreground transition-colors hover:bg-foreground/12"
						>
							<ClipboardPaste className="size-3.5 text-muted-foreground" />
							<span>Paste Clipboard</span>
						</button>

						<button
							type="button"
							onClick={handleApplyJson}
							className="inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-[11.5px] font-semibold whitespace-nowrap text-primary-foreground transition-opacity hover:opacity-90"
						>
							<span>Apply JSON</span>
						</button>
					</div>
				</div>

				<textarea
					aria-label="Custom theme JSON"
					value={displayedJson}
					onChange={(event) => {
						setJsonEditDraft(event.currentTarget.value)
						if (jsonErrors.length > 0) {
							setJsonErrors([])
						}
					}}
					rows={10}
					spellCheck={false}
					className="w-full resize-y rounded-xl border border-border bg-background/80 p-3 font-mono text-[11.5px] leading-relaxed text-foreground outline-none focus:border-primary/60"
				/>

				{jsonErrors.length > 0 ? (
					<div
						role="alert"
						className="rounded-xl border border-destructive/40 bg-destructive/12 px-3 py-2 text-[11.5px] text-destructive"
					>
						<p className="font-semibold">Unable to apply theme JSON:</p>
						<ul className="mt-1 list-disc pl-4">
							{jsonErrors.map((errorMessage) => {
								return <li key={errorMessage}>{errorMessage}</li>
							})}
						</ul>
					</div>
				) : null}
			</div>
		</section>
	)
}

export default CustomThemeEditor
