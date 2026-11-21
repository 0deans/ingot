import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { useActiveAccount } from "@/features/accounts/useActiveAccount"
import SkinCanvas3D, { type ArmModel, type CapeId } from "@/features/skins/components/SkinCanvas3D"
import SkinPresetGrid, {
	type SkinPreset,
	type SkinPresetId,
} from "@/features/skins/components/SkinPresetGrid"

const SKIN_PRESETS: readonly SkinPreset[] = [
	{
		id: "skin-emerald-smith",
		nameKey: "skins.presets.skin-emerald-smith.name",
		model: "classic",
		cape: "Migrator",
		gradient: "from-emerald-500/35 via-teal-600/20 to-zinc-900",
		subtitleKey: "skins.presets.skin-emerald-smith.subtitle",
	},
	{
		id: "skin-horizon-scout",
		nameKey: "skins.presets.skin-horizon-scout.name",
		model: "slim",
		cape: "15th Anniversary",
		gradient: "from-cyan-500/35 via-sky-600/20 to-zinc-900",
		subtitleKey: "skins.presets.skin-horizon-scout.subtitle",
	},
	{
		id: "skin-nether-wanderer",
		nameKey: "skins.presets.skin-nether-wanderer.name",
		model: "classic",
		cape: "Cherry Blossom",
		gradient: "from-rose-500/35 via-amber-600/20 to-zinc-900",
		subtitleKey: "skins.presets.skin-nether-wanderer.subtitle",
	},
	{
		id: "skin-amethyst-mage",
		nameKey: "skins.presets.skin-amethyst-mage.name",
		model: "slim",
		cape: "None",
		gradient: "from-violet-500/35 via-indigo-600/20 to-zinc-900",
		subtitleKey: "skins.presets.skin-amethyst-mage.subtitle",
	},
]

const CAPES: readonly CapeId[] = ["Migrator", "15th Anniversary", "Cherry Blossom", "None"]

const SkinsPage = () => {
	const { t } = useTranslation()
	const { activeAccount } = useActiveAccount()

	const [selectedSkinId, setSelectedSkinId] = useState<SkinPresetId>("skin-emerald-smith")
	const [armModel, setArmModel] = useState<ArmModel>("classic")
	const [selectedCape, setSelectedCape] = useState<CapeId>("Migrator")

	const activeSkin = useMemo(() => {
		const foundPreset = SKIN_PRESETS.find((preset) => {
			return preset.id === selectedSkinId
		})
		return foundPreset ?? SKIN_PRESETS[0]
	}, [selectedSkinId])

	const handleSelectPreset = (preset: SkinPreset) => {
		setSelectedSkinId(preset.id)
		setArmModel(preset.model)
		setSelectedCape(preset.cape)
	}

	return (
		<div className="flex flex-col gap-4">
			<div>
				<h1 className="text-lg font-semibold tracking-tight text-foreground">{t("skins.title")}</h1>
				<p className="text-[12px] text-muted-foreground">
					{t("skins.subtitlePrefix")}{" "}
					<span className="font-medium text-foreground">{activeAccount?.username ?? "Player"}</span>
					.
				</p>
			</div>

			<div className="grid grid-cols-1 gap-4 lg:grid-cols-[18rem_1fr]">
				<SkinCanvas3D
					skinName={t(activeSkin.nameKey)}
					gradient={activeSkin.gradient}
					armModel={armModel}
					selectedCape={selectedCape}
					capes={CAPES}
					onArmModelChange={setArmModel}
					onSelectCape={setSelectedCape}
				/>
				<SkinPresetGrid
					presets={SKIN_PRESETS}
					selectedId={selectedSkinId}
					onSelect={handleSelectPreset}
				/>
			</div>
		</div>
	)
}

export default SkinsPage
