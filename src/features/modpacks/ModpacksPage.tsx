import { useNavigate } from "@tanstack/react-router"
import { Search } from "lucide-react"
import { useMemo, useState } from "react"
import { useShallow } from "zustand/react/shallow"
import ModpackCard, {
	MODPACK_CATALOG,
	type ModpackItem,
} from "@/features/modpacks/components/ModpackCard"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { useInstancesStore } from "@/shared/stores/useInstancesStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

type ProviderFilter = "all" | "Modrinth" | "CurseForge"

const PROVIDER_OPTIONS: SegmentedOption<ProviderFilter>[] = [
	{ value: "all", label: "All Sources" },
	{ value: "Modrinth", label: "Modrinth" },
	{ value: "CurseForge", label: "CurseForge" },
]

const ModpacksPage = () => {
	const navigate = useNavigate()
	const { instances, selectOrCreateByName } = useInstancesStore(
		useShallow((s) => {
			return {
				instances: s.instances,
				selectOrCreateByName: s.selectOrCreateByName,
			}
		}),
	)
	const spawnMockProcess = useProcessStore((s) => {
		return s.spawnMockProcess
	})

	const [provider, setProvider] = useState<ProviderFilter>("all")
	const [query, setQuery] = useState("")

	const installedNames = useMemo(() => {
		const names = instances.map((inst) => {
			return inst.name.toLowerCase()
		})

		return new Set(names)
	}, [instances])

	const filteredPacks = useMemo(() => {
		const normalized = query.trim().toLowerCase()
		return MODPACK_CATALOG.filter((pack) => {
			if (provider !== "all" && pack.provider !== provider) {
				return false
			}
			if (normalized === "") {
				return true
			}
			return (
				pack.name.toLowerCase().includes(normalized) ||
				pack.summary.toLowerCase().includes(normalized) ||
				pack.tags.some((tag) => {
					return tag.toLowerCase().includes(normalized)
				})
			)
		})
	}, [provider, query])

	const handleInstallPack = (pack: ModpackItem) => {
		const isAlreadyInstalled = installedNames.has(pack.name.toLowerCase())
		selectOrCreateByName({
			name: pack.name,
			version: pack.version,
			loader: pack.loader,
			engine: pack.engine,
			colorTag: pack.engine === "Horizon (Rust)" ? "cyan" : "emerald",
		})
		if (!isAlreadyInstalled) {
			spawnMockProcess(`Install ${pack.name}`, "instance", pack.engine)
		}
		void navigate({ to: "/" })
	}

	return (
		<div className="flex flex-col gap-4">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">Modpacks Browser</h1>
					<p className="text-[12px] text-muted-foreground">
						Curated Modrinth and CurseForge profiles ready for 1-click instance creation.
					</p>
				</div>
				<SegmentedControl
					layoutId="modpacks-provider-filter"
					options={PROVIDER_OPTIONS}
					value={provider}
					onChange={setProvider}
					tone="emerald"
				/>
			</div>

			<div className="relative">
				<Search className="pointer-events-none absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-muted-foreground" />
				<input
					type="search"
					value={query}
					onChange={(event) => {
						setQuery(event.target.value)
					}}
					placeholder="Search modpacks by title, tag, or gameplay style..."
					className="h-9 w-full rounded-xl border border-border bg-card/70 pr-3 pl-8.5 text-[12.5px] text-foreground placeholder:text-muted-foreground focus:border-primary/40 focus:outline-none"
				/>
			</div>

			<div className="grid grid-cols-1 gap-3 md:grid-cols-2 lg:grid-cols-3">
				{filteredPacks.map((pack) => {
					return (
						<ModpackCard
							key={pack.id}
							pack={pack}
							isInstalled={installedNames.has(pack.name.toLowerCase())}
							onInstall={handleInstallPack}
						/>
					)
				})}
			</div>
		</div>
	)
}

export default ModpacksPage
