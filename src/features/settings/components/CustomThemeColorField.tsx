import { Undo2 } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import ColorPicker from "@/shared/components/ColorPicker"
import { cn } from "@/shared/lib/utils"
import {
	validateCssColor,
	type CustomThemeColors,
	type CustomThemeFieldDescriptor,
} from "@/shared/theme/customTheme"
import { Input } from "@/shared/ui/input"

type CustomThemeFieldLabelKey =
	| "settings.customTheme.fields.background"
	| "settings.customTheme.fields.foreground"
	| "settings.customTheme.fields.card"
	| "settings.customTheme.fields.cardForeground"
	| "settings.customTheme.fields.sidebar"
	| "settings.customTheme.fields.sidebarForeground"
	| "settings.customTheme.fields.popover"
	| "settings.customTheme.fields.popoverForeground"
	| "settings.customTheme.fields.primary"
	| "settings.customTheme.fields.primaryForeground"
	| "settings.customTheme.fields.muted"
	| "settings.customTheme.fields.mutedForeground"
	| "settings.customTheme.fields.border"
	| "settings.customTheme.fields.destructive"

const FIELD_LABEL_KEYS: Record<keyof CustomThemeColors, CustomThemeFieldLabelKey> = {
	background: "settings.customTheme.fields.background",
	foreground: "settings.customTheme.fields.foreground",
	card: "settings.customTheme.fields.card",
	cardForeground: "settings.customTheme.fields.cardForeground",
	sidebar: "settings.customTheme.fields.sidebar",
	sidebarForeground: "settings.customTheme.fields.sidebarForeground",
	popover: "settings.customTheme.fields.popover",
	popoverForeground: "settings.customTheme.fields.popoverForeground",
	primary: "settings.customTheme.fields.primary",
	primaryForeground: "settings.customTheme.fields.primaryForeground",
	muted: "settings.customTheme.fields.muted",
	mutedForeground: "settings.customTheme.fields.mutedForeground",
	border: "settings.customTheme.fields.border",
	destructive: "settings.customTheme.fields.destructive",
}

interface FieldDraftState {
	epoch: number
	value: string | null
	error: string | null
}

interface CustomThemeColorFieldProps {
	field: CustomThemeFieldDescriptor
	committedValue: string
	fallbackValue: string
	resetEpoch: number
	onCommitColor: (key: keyof CustomThemeColors, value: string) => void
}

const CustomThemeColorField = ({
	field,
	committedValue,
	fallbackValue,
	resetEpoch,
	onCommitColor,
}: CustomThemeColorFieldProps) => {
	const { t } = useTranslation()

	const [draftState, setDraftState] = useState<FieldDraftState>({
		epoch: resetEpoch,
		value: null,
		error: null,
	})

	const fieldLabel = t(FIELD_LABEL_KEYS[field.key])
	const revertLabel = t("settings.customTheme.revert")
	const isCurrentEpoch = draftState.epoch === resetEpoch
	const draft = isCurrentEpoch ? draftState.value : null
	const error = isCurrentEpoch ? draftState.error : null
	const displayValue = draft ?? committedValue
	const inputId = `custom-color-${field.key}`
	const errorId = `custom-color-error-${field.key}`

	const handleClearDraft = () => {
		setDraftState({ epoch: resetEpoch, value: null, error: null })
	}

	const handlePickerChange = (hex: string) => {
		handleClearDraft()
		onCommitColor(field.key, hex)
	}

	const handleTextChange = (nextValue: string) => {
		const validation = validateCssColor(nextValue)
		if (!validation.ok) {
			setDraftState({ epoch: resetEpoch, value: nextValue, error: validation.error })
			return
		}

		setDraftState({ epoch: resetEpoch, value: nextValue, error: null })
		onCommitColor(field.key, validation.normalized)
	}

	const handleBlur = () => {
		if (draft === null) {
			return
		}
		const validation = validateCssColor(draft)
		if (validation.ok) {
			handleClearDraft()
		}
	}

	return (
		<div
			className={cn(
				"flex flex-col gap-1.5 rounded-xl border p-2.5 transition-colors",
				error ? "border-destructive/60 bg-destructive/6" : "border-border bg-foreground/3",
			)}
		>
			<div className="flex items-center justify-between gap-2">
				<label htmlFor={inputId} className="text-[12px] font-medium text-foreground">
					{fieldLabel}
				</label>
				<span className="font-mono text-[10px] text-muted-foreground">{field.token}</span>
			</div>

			<div className="flex items-center gap-2">
				<ColorPicker
					label={fieldLabel}
					value={committedValue}
					fallback={fallbackValue}
					onChange={handlePickerChange}
				/>
				<Input
					id={inputId}
					type="text"
					value={displayValue}
					maxLength={64}
					aria-invalid={error ? true : undefined}
					aria-describedby={error ? errorId : undefined}
					onChange={(event) => {
						handleTextChange(event.currentTarget.value)
					}}
					onBlur={handleBlur}
					onKeyDown={(event) => {
						if (event.key === "Escape") {
							handleClearDraft()
						}
					}}
					spellCheck={false}
					placeholder={fallbackValue}
					className={cn(
						"flex-1 bg-background/70 font-mono text-[11.5px] md:text-[11.5px]",
						error ? "border-destructive" : "border-border",
					)}
				/>
			</div>

			{error ? (
				<div className="flex items-start justify-between gap-2">
					<p id={errorId} className="text-[10.5px] leading-snug text-destructive">
						{error}
					</p>
					<button
						type="button"
						onClick={handleClearDraft}
						aria-label={`${revertLabel} ${fieldLabel}`}
						title={t("settings.customTheme.revertTitle")}
						className="inline-flex shrink-0 cursor-pointer items-center gap-1 rounded px-1 py-0.5 text-[10px] font-medium text-muted-foreground hover:bg-foreground/10 hover:text-foreground"
					>
						<Undo2 className="size-2.5" />
						<span>{revertLabel}</span>
					</button>
				</div>
			) : null}
		</div>
	)
}

export default CustomThemeColorField
