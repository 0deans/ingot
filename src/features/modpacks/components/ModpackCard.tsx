import { Link } from "@tanstack/react-router"
import { Check, Download, Package, Sparkles } from "lucide-react"
import { cn } from "@/shared/lib/utils"
import type { InstanceEngine } from "@/shared/stores/useInstancesStore"

export interface ModpackItem {
	id: string
	name: string
	author: string
	provider: "Modrinth" | "CurseForge"
	version: string
	loader: string
	engine: InstanceEngine
	downloads: string
	summary: string
	tags: string[]
	accentClass: string
}

export const MODPACK_CATALOG: ModpackItem[] = [
	{
		id: "mp-fabulously",
		name: "Fabulously Optimized",
		author: "robotkoer",
		provider: "Modrinth",
		version: "1.21.4",
		loader: "Fabric 0.16.10",
		engine: "Java",
		downloads: "14.2M",
		summary: "OptiFine parity, Sodium & Iris shaders, and smooth frame pacing out of the box.",
		tags: ["Performance", "Shaders"],
		accentClass: "from-emerald-500/25 to-teal-500/10 text-emerald-700 dark:text-emerald-300",
	},
	{
		id: "mp-create-astral",
		name: "Create: Astral",
		author: "Lask",
		provider: "CurseForge",
		version: "1.18.2",
		loader: "Fabric 0.15.11",
		engine: "Java",
		downloads: "2.8M",
		summary: "Progression-driven industrial automation leading into Ad Astra space exploration.",
		tags: ["Tech", "Space"],
		accentClass: "from-amber-500/25 to-orange-500/10 text-amber-700 dark:text-amber-300",
	},
	{
		id: "mp-prominence",
		name: "Prominence II RPG",
		author: "LunaPixelStudios",
		provider: "Modrinth",
		version: "1.20.1",
		loader: "Fabric 0.16.9",
		engine: "Java",
		downloads: "4.6M",
		summary: "Overhauled combat trees, boss progression, and curated quests for multiplayer.",
		tags: ["RPG", "Quests"],
		accentClass: "from-violet-500/25 to-indigo-500/10 text-violet-700 dark:text-violet-300",
	},
	{
		id: "mp-horizon-native",
		name: "Horizon Native Shader Kit",
		author: "Ingot Core",
		provider: "Modrinth",
		version: "0.9.4",
		loader: "Horizon Core",
		engine: "Horizon (Rust)",
		downloads: "410K",
		summary: "Zero-GC Vulkan chunk meshing profile with native PBR reflections and spatial audio.",
		tags: ["Rust Engine", "Vulkan"],
		accentClass: "from-cyan-500/25 to-sky-500/10 text-cyan-700 dark:text-cyan-300",
	},
	{
		id: "mp-atm10",
		name: "All the Mods 10",
		author: "ATMTeam",
		provider: "CurseForge",
		version: "1.21.1",
		loader: "NeoForge 21.1",
		engine: "Java",
		downloads: "6.1M",
		summary: "Kitchen-sink mega-pack with custom endgame recipes and guided quest chapters.",
		tags: ["Kitchen Sink", "Endgame"],
		accentClass: "from-rose-500/25 to-pink-500/10 text-rose-700 dark:text-rose-300",
	},
	{
		id: "mp-cobblemon",
		name: "Cobblemon Horizons",
		author: "CobbleDev",
		provider: "Modrinth",
		version: "1.21.1",
		loader: "Fabric 0.16.10",
		engine: "Java",
		downloads: "3.4M",
		summary: "Creature catching seamlessly integrated into vanilla biomes with custom structures.",
		tags: ["Adventure", "Multiplayer"],
		accentClass: "from-sky-500/25 to-emerald-500/10 text-sky-700 dark:text-sky-300",
	},
]

interface ModpackCardProps {
	pack: ModpackItem
	isInstalled: boolean
	onInstall: (pack: ModpackItem) => void
}

const ModpackCard = ({ pack, isInstalled, onInstall }: ModpackCardProps) => {
	return (
		<article className="flex flex-col justify-between gap-3 rounded-2xl border border-border bg-card/65 p-4 transition-colors hover:border-foreground/20 hover:bg-card">
			<div>
				<div className="mb-3 flex items-start justify-between gap-2.5">
					<Link
						to="/modpacks/$projectId"
						params={{ projectId: pack.id }}
						className={cn(
							"flex size-10 shrink-0 items-center justify-center rounded-xl bg-linear-to-br",
							pack.accentClass,
						)}
					>
						{pack.engine === "Horizon (Rust)" ? (
							<Sparkles className="size-4.5" />
						) : (
							<Package className="size-4.5" />
						)}
					</Link>
					<span className="rounded-full border border-border bg-foreground/5 px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
						{pack.provider} · {pack.downloads}
					</span>
				</div>

				<Link
					to="/modpacks/$projectId"
					params={{ projectId: pack.id }}
					className="text-[14px] font-semibold text-foreground transition-colors hover:text-primary"
				>
					{pack.name}
				</Link>
				<p className="mt-0.5 text-[11px] text-muted-foreground">
					by {pack.author} · {pack.version} ({pack.loader})
				</p>
				<p className="mt-2 line-clamp-2 text-[12px] leading-relaxed text-foreground/80">
					{pack.summary}
				</p>
			</div>

			<div className="flex items-center justify-between gap-2 border-t border-border pt-3">
				<div className="flex flex-wrap gap-1">
					{pack.tags.map((tag) => {
						return (
							<span
								key={tag}
								className="rounded-md bg-foreground/6 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground"
							>
								{tag}
							</span>
						)
					})}
				</div>

				<button
					type="button"
					onClick={() => {
						onInstall(pack)
					}}
					className={cn(
						"flex h-7 shrink-0 cursor-pointer items-center gap-1 rounded-lg px-2.5 text-[11.5px] font-semibold transition-all",
						isInstalled
							? "border border-primary/35 bg-primary/15 text-primary hover:bg-primary/25"
							: "bg-primary text-primary-foreground hover:opacity-90",
					)}
				>
					{isInstalled ? (
						<>
							<Check className="size-3.5" />
							<span>Open</span>
						</>
					) : (
						<>
							<Download className="size-3.5" />
							<span>Install</span>
						</>
					)}
				</button>
			</div>
		</article>
	)
}

export default ModpackCard
