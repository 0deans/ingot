import { Check } from "lucide-react"
import { cn } from "@/shared/lib/utils"
import type { NavVariant } from "@/shared/stores/useLauncherPrefsStore"

export interface NavVariantOption {
	id: NavVariant
	title: string
	description: string
}

interface NavVariantCardProps {
	variant: NavVariantOption
	isSelected: boolean
	onSelect: (id: NavVariant) => void
}

const NavVariantCard = ({ variant, isSelected, onSelect }: NavVariantCardProps) => {
	return (
		<button
			type="button"
			onClick={() => {
				onSelect(variant.id)
			}}
			aria-pressed={isSelected}
			className={cn(
				"flex min-w-0 cursor-pointer flex-col items-start gap-1 rounded-xl border p-3.5 text-left transition-colors",
				isSelected
					? "border-primary/45 bg-primary/12 ring-1 ring-primary/25"
					: "border-border bg-foreground/3 hover:border-foreground/20 hover:bg-foreground/6",
			)}
		>
			<div className="flex w-full items-center justify-between gap-2">
				<span className="text-[13px] font-semibold text-foreground">{variant.title}</span>
				{isSelected ? <Check className="size-3.5 text-primary" /> : null}
			</div>
			<p className="text-[11.5px] leading-relaxed text-muted-foreground">{variant.description}</p>
		</button>
	)
}

export default NavVariantCard
