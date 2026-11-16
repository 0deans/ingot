import { Undo2 } from "lucide-react"
import { useState } from "react"
import ColorPicker from "@/shared/components/ColorPicker"
import { cn } from "@/shared/lib/utils"
import {
	validateCssColor,
	type CustomThemeColors,
	type CustomThemeFieldDescriptor,
} from "@/shared/theme/customTheme"

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
	const [draftState, setDraftState] = useState<FieldDraftState>({
		epoch: resetEpoch,
		value: null,
		error: null,
	})

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
					{field.label}
				</label>
				<span className="font-mono text-[10px] text-muted-foreground">{field.token}</span>
			</div>

			<div className="flex items-center gap-2">
				<ColorPicker
					label={field.label}
					value={committedValue}
					fallback={fallbackValue}
					onChange={handlePickerChange}
				/>
				<input
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
						"h-8 min-w-0 flex-1 rounded-lg border bg-background/70 px-2.5 font-mono text-[11.5px] text-foreground transition-colors outline-none",
						error
							? "border-destructive focus:border-destructive"
							: "border-border focus:border-primary/60",
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
						aria-label={`Revert ${field.label}`}
						title="Revert to last valid color"
						className="inline-flex shrink-0 cursor-pointer items-center gap-1 rounded px-1 py-0.5 text-[10px] font-medium text-muted-foreground hover:bg-foreground/10 hover:text-foreground"
					>
						<Undo2 className="size-2.5" />
						<span>Revert</span>
					</button>
				</div>
			) : null}
		</div>
	)
}

export default CustomThemeColorField
