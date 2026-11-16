import { Check, Sparkles } from "lucide-react"
import { cn } from "@/shared/lib/utils"
import type { AppThemeId, CustomThemeColors } from "@/shared/theme/customTheme"

export interface ThemeOptionMeta {
	id: AppThemeId
	title: string
	description: string
}

interface ThemeOptionCardProps {
	option: ThemeOptionMeta
	colors: CustomThemeColors
	isSelected: boolean
	onSelect: (id: AppThemeId) => void
}

const ThemeOptionCard = ({ option, colors, isSelected, onSelect }: ThemeOptionCardProps) => {
	const isCustom = option.id === "custom"

	return (
		<button
			type="button"
			onClick={() => {
				onSelect(option.id)
			}}
			aria-pressed={isSelected}
			className={cn(
				"flex cursor-pointer flex-col items-start gap-2 rounded-xl border p-3.5 text-left transition-colors",
				isSelected
					? "border-primary/50 bg-primary/12 ring-1 ring-primary/25"
					: "border-border bg-foreground/3 hover:border-foreground/20 hover:bg-foreground/6",
			)}
		>
			<div className="flex w-full items-center justify-between gap-2">
				<div className="flex items-center gap-1.5">
					{isCustom ? <Sparkles className="size-3.5 text-primary" /> : null}
					<span className="text-[13px] font-semibold text-foreground">{option.title}</span>
				</div>
				{isSelected ? <Check className="size-3.5 text-primary" /> : null}
			</div>

			<div className="flex items-center gap-1.5">
				<span
					aria-hidden="true"
					className="h-4 w-8 shrink-0 rounded-full border border-border"
					style={{ backgroundColor: colors.background }}
				/>
				<span
					aria-hidden="true"
					className="h-4 w-8 shrink-0 rounded-full border border-border"
					style={{ backgroundColor: colors.sidebar }}
				/>
				<span
					aria-hidden="true"
					className="h-4 w-8 shrink-0 rounded-full border border-border"
					style={{ backgroundColor: colors.card }}
				/>
				<span
					aria-hidden="true"
					className="h-4 w-8 shrink-0 rounded-full border border-border"
					style={{ backgroundColor: colors.primary }}
				/>
			</div>

			<p className="text-[11px] leading-relaxed text-muted-foreground">{option.description}</p>
		</button>
	)
}

export default ThemeOptionCard
