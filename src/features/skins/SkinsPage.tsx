import { useMemo, useState } from "react"
import SkinCanvas3D, { type ArmModel } from "@/features/skins/components/SkinCanvas3D"
import SkinPresetGrid, { type SkinPreset } from "@/features/skins/components/SkinPresetGrid"
import { selectActiveAccount, useAccountStore } from "@/shared/stores/useAccountStore"

const SKIN_PRESETS: SkinPreset[] = [
	{
		id: "skin-emerald-smith",
		name: "Ingot Artificer",
		model: "classic",
		cape: "Migrator",
		gradient: "from-emerald-500/35 via-teal-600/20 to-zinc-900",
		subtitle: "Brushed dark iron armor with glowing emerald trim",
	},
	{
		id: "skin-horizon-scout",
		name: "Horizon Scout",
		model: "slim",
		cape: "15th Anniversary",
		gradient: "from-cyan-500/35 via-sky-600/20 to-zinc-900",
		subtitle: "Rust-native pilot suit with visor reflection layer",
	},
	{
		id: "skin-nether-wanderer",
		name: "Basalt Wanderer",
		model: "classic",
		cape: "Cherry Blossom",
		gradient: "from-rose-500/35 via-amber-600/20 to-zinc-900",
		subtitle: "Weathered expedition cloak for late-game survival",
	},
	{
		id: "skin-amethyst-mage",
		name: "Amethyst Archivist",
		model: "slim",
		cape: "None",
		gradient: "from-violet-500/35 via-indigo-600/20 to-zinc-900",
		subtitle: "Lightweight robes with layered crystal pauldrons",
	},
]

const CAPES = ["Migrator", "15th Anniversary", "Cherry Blossom", "None"]

const SkinsPage = () => {
	const activeAccount = useAccountStore(selectActiveAccount)

	const [selectedSkinId, setSelectedSkinId] = useState("skin-emerald-smith")
	const [armModel, setArmModel] = useState<ArmModel>("classic")
	const [selectedCape, setSelectedCape] = useState("Migrator")

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
				<h1 className="text-lg font-semibold tracking-tight text-foreground">
					Skins &amp; Wardrobe
				</h1>
				<p className="text-[12px] text-muted-foreground">
					Manage player models and capes synced to{" "}
					<span className="font-medium text-foreground">{activeAccount.username}</span>.
				</p>
			</div>

			<div className="grid grid-cols-1 gap-4 lg:grid-cols-[18rem_1fr]">
				<SkinCanvas3D
					skinName={activeSkin.name}
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
