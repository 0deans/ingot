import { Check, Sparkles } from "lucide-react"
import { useTranslation } from "react-i18next"
import {
	CAPE_LABEL_KEYS,
	type ArmModel,
	type CapeId,
} from "@/features/skins/components/SkinCanvas3D"
import { cn } from "@/shared/lib/utils"

export type SkinPresetId =
	| "skin-emerald-smith"
	| "skin-horizon-scout"
	| "skin-nether-wanderer"
	| "skin-amethyst-mage"

export interface SkinPreset {
	id: SkinPresetId
	nameKey: `skins.presets.${SkinPresetId}.name`
	model: ArmModel
	cape: CapeId
	gradient: string
	subtitleKey: `skins.presets.${SkinPresetId}.subtitle`
}

interface SkinPresetGridProps {
	presets: readonly SkinPreset[]
	selectedId: SkinPresetId
	onSelect: (preset: SkinPreset) => void
}

const SkinPresetGrid = ({ presets, selectedId, onSelect }: SkinPresetGridProps) => {
	const { t } = useTranslation()

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
									<span>
										{preset.model === "classic" ? t("skins.classicBadge") : t("skins.slimBadge")}
									</span>
								</span>
								{isSelected ? <Check className="size-4 text-primary" /> : null}
							</div>
							<h2 className="text-[14px] font-semibold text-foreground">{t(preset.nameKey)}</h2>
							<p className="mt-1 text-[12px] text-muted-foreground">{t(preset.subtitleKey)}</p>
						</div>

						<div className="mt-4 flex items-center justify-between border-t border-border pt-2.5 text-[11px] text-muted-foreground">
							<span>{t("skins.capePreset", { cape: t(CAPE_LABEL_KEYS[preset.cape]) })}</span>
							<span className="font-medium text-primary">
								{isSelected ? t("skins.equipped") : t("skins.select")}
							</span>
						</div>
					</button>
				)
			})}
		</section>
	)
}

export default SkinPresetGrid
