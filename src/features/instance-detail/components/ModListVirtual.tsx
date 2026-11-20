import { ArrowUpCircle, Package, Search } from "lucide-react"
import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { Input } from "@/shared/ui/input"
import { Switch } from "@/shared/ui/switch"

interface ModEntry {
	id: string
	name: string
	version: string
	hasUpdate: boolean
	enabled: boolean
}

const INITIAL_MODS: ModEntry[] = [
	{ id: "sodium", name: "Sodium", version: "0.6.5+mc1.21.4", hasUpdate: false, enabled: true },
	{ id: "iris", name: "Iris Shaders", version: "1.8.4+mc1.21.4", hasUpdate: true, enabled: true },
	{ id: "lithium", name: "Lithium", version: "0.14.7+mc1.21.4", hasUpdate: false, enabled: true },
	{ id: "modmenu", name: "Mod Menu", version: "13.0.2", hasUpdate: false, enabled: true },
	{
		id: "ferrite",
		name: "FerriteCore",
		version: "7.1.0-fabric",
		hasUpdate: true,
		enabled: false,
	},
]

const ModListVirtual = () => {
	const { t } = useTranslation()

	const [mods, setMods] = useState<ModEntry[]>(INITIAL_MODS)
	const [query, setQuery] = useState("")

	const filtered = useMemo(() => {
		const normalized = query.trim().toLowerCase()
		if (!normalized) {
			return mods
		}
		return mods.filter((mod) => {
			return mod.name.toLowerCase().includes(normalized)
		})
	}, [mods, query])

	const toggleMod = (id: string) => {
		setMods((prev) => {
			return prev.map((mod) => {
				if (mod.id !== id) {
					return mod
				}

				return { ...mod, enabled: !mod.enabled }
			})
		})
	}

	return (
		<div className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="relative">
				<Search className="pointer-events-none absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-muted-foreground" />
				<Input
					type="search"
					value={query}
					onChange={(event) => {
						setQuery(event.target.value)
					}}
					placeholder={t("instanceDetail.mods.filterPlaceholder")}
					className="h-8 rounded-xl border-border bg-background/80 pr-3 pl-8 text-xs md:text-xs"
				/>
			</div>

			<div className="flex flex-col divide-y divide-border">
				{filtered.map((mod) => {
					return (
						<div key={mod.id} className="flex items-center justify-between gap-3 py-2.5">
							<div className="flex min-w-0 items-center gap-2.5">
								<Package className="size-4 shrink-0 text-primary" />
								<div className="min-w-0">
									<div className="flex items-center gap-1.5">
										<span className="truncate text-[13px] font-medium text-foreground">
											{mod.name}
										</span>
										{mod.hasUpdate ? (
											<span className="inline-flex items-center gap-1 rounded-full bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-medium text-amber-600 dark:text-amber-300">
												<ArrowUpCircle className="size-3" />
												<span>{t("instanceDetail.mods.updateBadge")}</span>
											</span>
										) : null}
									</div>
									<p className="font-mono text-[11px] text-muted-foreground">{mod.version}</p>
								</div>
							</div>

							<Switch
								checked={mod.enabled}
								onCheckedChange={() => {
									toggleMod(mod.id)
								}}
								aria-label={mod.name}
							/>
						</div>
					)
				})}
			</div>
		</div>
	)
}

export default ModListVirtual
