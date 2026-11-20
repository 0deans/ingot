import { Check, ClipboardPaste, Copy, RotateCcw, SlidersHorizontal } from "lucide-react"
import { useCallback, useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
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

type SeedPresetId = Exclude<AppThemeId, "custom">

type SeedPresetLabelKey =
	| "settings.appearance.themes.dark.title"
	| "settings.appearance.themes.light.title"
	| "settings.appearance.themes.purple.title"

const RADIUS_PRESETS: readonly number[] = [0, 0.35, 0.625, 0.75, 1.0]

const SEED_PRESETS: readonly { id: SeedPresetId; labelKey: SeedPresetLabelKey }[] = [
	{ id: "dark", labelKey: "settings.appearance.themes.dark.title" },
	{ id: "light", labelKey: "settings.appearance.themes.light.title" },
	{ id: "purple", labelKey: "settings.appearance.themes.purple.title" },
]

const CustomThemeEditor = () => {
	const { t } = useTranslation()
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

	const baseModeOptions: SegmentedOption<CustomThemeBaseMode>[] = useMemo(() => {
		return [
			{ value: "dark", label: t("settings.customTheme.darkBase"), dotClass: "bg-zinc-900" },
			{ value: "light", label: t("settings.customTheme.lightBase"), dotClass: "bg-zinc-100" },
		]
	}, [t])

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
			toast.success(t("settings.customTheme.copiedToast"))
		} catch {
			toast.error(t("settings.customTheme.copyFailedToast"))
		}
	}, [customTheme, t])

	const handlePasteClipboard = useCallback(async () => {
		try {
			const clipboardText = await navigator.clipboard.readText()
			if (!clipboardText.trim()) {
				toast.info(t("settings.customTheme.emptyClipboardToast"))
				return
			}
			const parsed = parseCustomThemeJson(clipboardText)
			if (!parsed.ok) {
				setJsonEditDraft(clipboardText)
				setJsonErrors(parsed.errors)
				toast.error(t("settings.customTheme.clipboardErrorToast"))
				return
			}
			setCustomTheme(parsed.value)
			resetBulkDrafts()
			toast.success(t("settings.customTheme.appliedClipboardToast"))
		} catch {
			toast.info(t("settings.customTheme.clipboardBlockedToast"))
		}
	}, [resetBulkDrafts, setCustomTheme, t])

	const displayedJson = jsonEditDraft ?? stringifyCustomThemeJson(customTheme)

	const handleApplyJson = useCallback(() => {
		const parsed = parseCustomThemeJson(displayedJson)
		if (!parsed.ok) {
			setJsonErrors(parsed.errors)
			toast.error(t("settings.customTheme.invalidJsonToast"))
			return
		}
		setCustomTheme(parsed.value)
		resetBulkDrafts()
		toast.success(t("settings.customTheme.appliedJsonToast"))
	}, [displayedJson, resetBulkDrafts, setCustomTheme, t])

	const handleSeedPreset = useCallback(
		(presetId: SeedPresetId, presetLabel: string) => {
			resetCustomTheme(presetId)
			resetBulkDrafts()
			toast.success(t("settings.customTheme.loadedPresetToast", { preset: presetLabel }))
		},
		[resetBulkDrafts, resetCustomTheme, t],
	)

	const handleReset = useCallback(() => {
		resetCustomTheme()
		resetBulkDrafts()
		toast.success(t("settings.customTheme.resetDefaultToast"))
	}, [resetBulkDrafts, resetCustomTheme, t])

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
						<h2 className="text-[13.5px] font-semibold text-foreground">
							{t("settings.customTheme.title")}
						</h2>
						<p className="text-[11.5px] text-muted-foreground">
							{t("settings.customTheme.subtitle")}
						</p>
					</div>
				</div>

				<div className="flex flex-wrap items-center gap-1.5">
					<span className="mr-1 text-[11px] font-medium text-muted-foreground">
						{t("settings.customTheme.seedPreset")}
					</span>
					{SEED_PRESETS.map((preset) => {
						const presetLabel = t(preset.labelKey)
						return (
							<button
								key={preset.id}
								type="button"
								onClick={() => {
									handleSeedPreset(preset.id, presetLabel)
								}}
								className="cursor-pointer rounded-lg border border-border bg-foreground/5 px-2.5 py-1 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/10"
							>
								{presetLabel}
							</button>
						)
					})}
					<button
						type="button"
						onClick={handleReset}
						className="inline-flex cursor-pointer items-center gap-1 rounded-lg border border-border bg-foreground/5 px-2.5 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
					>
						<RotateCcw className="size-3" />
						<span>{t("common.reset")}</span>
					</button>
				</div>
			</div>

			<div className="grid grid-cols-1 gap-3 rounded-xl border border-border bg-foreground/3 p-3.5 md:grid-cols-2">
				<div className="flex flex-col gap-2">
					<span className="text-[11.5px] font-medium text-foreground">
						{t("settings.customTheme.baseContrastMode")}
					</span>
					<div>
						<SegmentedControl
							layoutId="custom-theme-base-mode"
							options={baseModeOptions}
							value={customTheme.baseMode}
							onChange={handleBaseModeChange}
							tone="emerald"
						/>
					</div>
					<p className="text-[11px] text-muted-foreground">
						{t("settings.customTheme.baseContrastHint")}
					</p>
				</div>

				<div className="flex flex-col gap-2">
					<div className="flex items-center justify-between gap-2">
						<label
							htmlFor="custom-theme-radius"
							className="text-[11.5px] font-medium text-foreground"
						>
							{t("settings.customTheme.cornerRadius")}
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
					{t("settings.customTheme.colorTokens")}
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
						<h3 className="text-[12.5px] font-semibold text-foreground">
							{t("settings.customTheme.jsonTitle")}
						</h3>
						<p className="text-[11px] text-muted-foreground">
							{t("settings.customTheme.jsonSubtitle")}
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
							<span>{copiedJson ? t("common.copied") : t("settings.customTheme.copyJson")}</span>
						</button>

						<button
							type="button"
							onClick={() => {
								void handlePasteClipboard()
							}}
							className="inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-foreground/6 px-2.5 py-1.5 text-[11.5px] font-medium whitespace-nowrap text-foreground transition-colors hover:bg-foreground/12"
						>
							<ClipboardPaste className="size-3.5 text-muted-foreground" />
							<span>{t("settings.customTheme.pasteClipboard")}</span>
						</button>

						<button
							type="button"
							onClick={handleApplyJson}
							className="inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-[11.5px] font-semibold whitespace-nowrap text-primary-foreground transition-opacity hover:opacity-90"
						>
							<span>{t("settings.customTheme.applyJson")}</span>
						</button>
					</div>
				</div>

				<textarea
					aria-label={t("settings.customTheme.jsonTitle")}
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
						<p className="font-semibold">{t("settings.customTheme.jsonErrorsHeading")}</p>
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
