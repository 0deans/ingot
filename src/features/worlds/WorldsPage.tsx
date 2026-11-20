import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import BackupSnapshotModal from "@/features/worlds/components/BackupSnapshotModal"
import WorldCard, { type WorldSave } from "@/features/worlds/components/WorldCard"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { selectActiveAccount, useAccountStore } from "@/shared/stores/useAccountStore"
import { useInstancesStore } from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

type GameModeFilter = "all" | "Survival" | "Creative" | "Hardcore"

const WORLD_SAVES: WorldSave[] = [
	{
		id: "w-1",
		name: "Aldervale SMP Season 4",
		instanceName: "Fabulously Optimized",
		version: "1.21.4",
		loader: "Fabric 0.16.10",
		engine: "Java",
		mode: "Survival",
		seed: "-482910481924",
		sizeMb: 1420,
		lastPlayed: "2 hours ago",
	},
	{
		id: "w-2",
		name: "Rust Mesh Benchmark",
		instanceName: "Horizon Engine Nightly",
		version: "0.9.4-alpha",
		loader: "Horizon Core",
		engine: "Horizon (Rust)",
		mode: "Creative",
		seed: "991024881",
		sizeMb: 380,
		lastPlayed: "Yesterday",
	},
	{
		id: "w-3",
		name: "Trial Chambers Permadeath",
		instanceName: "Hardcore Vanilla",
		version: "1.21.4",
		loader: "Vanilla",
		engine: "Java",
		mode: "Hardcore",
		seed: "7720491038",
		sizeMb: 645,
		lastPlayed: "3 days ago",
	},
	{
		id: "w-4",
		name: "Orbital Brass Foundry",
		instanceName: "Create: Astral",
		version: "1.18.2",
		loader: "Fabric 0.15.11",
		engine: "Java",
		mode: "Survival",
		seed: "1129482710",
		sizeMb: 2150,
		lastPlayed: "Last week",
	},
]

const WorldsPage = () => {
	const { t } = useTranslation()
	const selectOrCreateByName = useInstancesStore((s) => {
		return s.selectOrCreateByName
	})
	const launchInstance = useProcessStore((s) => {
		return s.launchInstance
	})
	const activeAccount = useAccountStore(selectActiveAccount)
	const ramGb = useLauncherPrefsStore((s) => {
		return s.ramGb
	})

	const [modeFilter, setModeFilter] = useState<GameModeFilter>("all")
	const [backupWorld, setBackupWorld] = useState<WorldSave | null>(null)

	const modeOptions: SegmentedOption<GameModeFilter>[] = useMemo(() => {
		return [
			{ value: "all", label: t("worlds.modes.all") },
			{ value: "Survival", label: t("worlds.modes.survival") },
			{ value: "Creative", label: t("worlds.modes.creative") },
			{ value: "Hardcore", label: t("worlds.modes.hardcore") },
		]
	}, [t])

	const filteredWorlds = useMemo(() => {
		if (modeFilter === "all") {
			return WORLD_SAVES
		}
		return WORLD_SAVES.filter((world) => {
			return world.mode === modeFilter
		})
	}, [modeFilter])

	const handleLaunchWorld = (world: WorldSave) => {
		selectOrCreateByName({
			name: world.instanceName,
			version: world.version,
			loader: world.loader,
			engine: world.engine,
			colorTag: world.engine === "Horizon (Rust)" ? "rose" : "emerald",
		})
		const { instances, selectedId } = useInstancesStore.getState()
		const targetInstance = instances.find((inst) => {
			return inst.id === selectedId
		})
		if (targetInstance) {
			launchInstance({
				instance: targetInstance,
				username: activeAccount.username,
				ramGb,
			})
		}
	}

	return (
		<div className="flex flex-col gap-4">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">
						{t("worlds.title")}
					</h1>
					<p className="text-[12px] text-muted-foreground">{t("worlds.subtitle")}</p>
				</div>
				<SegmentedControl
					layoutId="worlds-mode-filter"
					options={modeOptions}
					value={modeFilter}
					onChange={setModeFilter}
					tone="emerald"
				/>
			</div>

			<div className="grid grid-cols-1 gap-3 md:grid-cols-2">
				{filteredWorlds.map((world) => {
					return (
						<WorldCard
							key={world.id}
							world={world}
							onPlay={handleLaunchWorld}
							onBackup={setBackupWorld}
						/>
					)
				})}
			</div>

			<BackupSnapshotModal
				world={backupWorld}
				onClose={() => {
					setBackupWorld(null)
				}}
			/>
		</div>
	)
}

export default WorldsPage
