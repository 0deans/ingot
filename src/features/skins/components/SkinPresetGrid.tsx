import { Check, Sparkles } from "lucide-react"
import type { ArmModel } from "@/features/skins/components/SkinCanvas3D"
import { cn } from "@/shared/lib/utils"

export interface SkinPreset {
	id: string
	name: string
	model: ArmModel
	cape: string
	gradient: string
	subtitle: string
}

interface SkinPresetGridProps {
	presets: readonly SkinPreset[]
	selectedId: string
	onSelect: (preset: SkinPreset) => void
}

const SkinPresetGrid = ({ presets, selectedId, onSelect }: SkinPresetGridProps) => {
	return (
		<section className="grid grid-cols-1 gap-3 sm:grid-cols-2">
			{presets.map((preset) => {
				const isSelected = preset.id === selectedId
				return (
					<button
						key={preset.id}
						type="button"
						onClick={() => {
							onSelect(preset)
						}}
						className={cn(
							"flex cursor-pointer flex-col justify-between rounded-2xl border p-4 text-left transition-all",
							isSelected
								? "border-primary/45 bg-primary/10 ring-1 ring-primary/25"
								: "border-border bg-card/65 hover:border-foreground/20 hover:bg-card",
						)}
					>
						<div>
							<div className="mb-3 flex items-center justify-between">
								<span
									className={cn(
										"inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium",
										isSelected
											? "bg-primary/20 text-primary"
											: "bg-foreground/8 text-muted-foreground",
									)}
								>
									<Sparkles className="size-3" />
									<span>{preset.model === "classic" ? "Classic" : "Slim"}</span>
								</span>
								{isSelected ? <Check className="size-4 text-primary" /> : null}
							</div>
							<h2 className="text-[14px] font-semibold text-foreground">{preset.name}</h2>
							<p className="mt-1 text-[12px] text-muted-foreground">{preset.subtitle}</p>
						</div>

						<div className="mt-4 flex items-center justify-between border-t border-border pt-2.5 text-[11px] text-muted-foreground">
							<span>Cape preset: {preset.cape}</span>
							<span className="font-medium text-primary">{isSelected ? "Equipped" : "Select"}</span>
						</div>
					</button>
				)
			})}
		</section>
	)
}

export default SkinPresetGrid
