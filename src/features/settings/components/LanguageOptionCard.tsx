import { Check } from "lucide-react"
import { type AppLanguageMeta, type AppLocaleCode } from "@/shared/i18n/languages"
import { cn } from "@/shared/lib/utils"

interface LanguageOptionCardProps {
	language: AppLanguageMeta
	isSelected: boolean
	isSystemDetected: boolean
	onSelect: (code: AppLocaleCode) => void
}

const LanguageOptionCard = ({
	language,
	isSelected,
	isSystemDetected,
	onSelect,
}: LanguageOptionCardProps) => {
	return (
		<button
			type="button"
			aria-pressed={isSelected}
			onClick={() => {
				onSelect(language.code)
			}}
			className={cn(
				"flex min-w-0 cursor-pointer items-center gap-3 rounded-xl border p-2.5 text-left transition-colors",
				isSelected
					? "border-primary/45 bg-primary/12 ring-1 ring-primary/25"
					: "border-border bg-foreground/3 hover:border-foreground/20 hover:bg-foreground/6",
			)}
		>
			<span
				className={cn(
					"flex h-8 w-10 shrink-0 items-center justify-center rounded-lg border font-mono text-[11px] font-semibold uppercase",
					isSelected
						? "border-primary/35 bg-primary/18 text-primary"
						: "border-border bg-foreground/5 text-muted-foreground",
				)}
			>
				{language.code}
			</span>

			<div className="min-w-0 flex-1">
				<div className="flex items-center gap-1.5">
					<span className="truncate text-[12.5px] font-semibold text-foreground">
						{language.nativeName}
					</span>
					{isSystemDetected ? (
						<span className="shrink-0 rounded-full border border-border bg-foreground/5 px-1.5 py-0.5 text-[9.5px] font-medium text-muted-foreground">
							System
						</span>
					) : null}
				</div>
				<span className="block truncate text-[11px] text-muted-foreground">
					{language.englishName}
				</span>
			</div>

			{isSelected ? <Check className="size-3.5 shrink-0 text-primary" /> : null}
		</button>
	)
}

export default LanguageOptionCard
