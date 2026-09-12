import { createFileRoute } from "@tanstack/react-router"
import { Download } from "lucide-react"
import { memo } from "react"
import { Button } from "@/components/ui/button"

const ModpacksPage = () => {
	const packs = [
		{
			title: "Fabulously Optimized",
			author: "Fabulously Team",
			downloads: "2.4M",
			version: "1.21.4",
			description: "A simple modpack focusing on boosting performance and adding graphic features.",
		},
		{
			title: "Better MC [Fabric]",
			author: "SHXRKIE",
			downloads: "5.1M",
			version: "1.21.1",
			description: "The successor of Minecraft with hundreds of new biomes, bosses, and quests.",
		},
		{
			title: "All the Mods 10",
			author: "ATM Team",
			downloads: "1.8M",
			version: "1.21.1",
			description: "All the best tech, magic, and adventure mods packed into one epic journey.",
		},
	]

	return (
		<div className="flex flex-1 flex-col gap-6">
			<div>
				<h1 className="font-bold text-2xl text-foreground tracking-tight">Browse Modpacks</h1>
				<p className="text-muted-foreground text-sm">
					Discover and install curated modpacks from CurseForge and Modrinth.
				</p>
			</div>

			<div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{packs.map((pack) => (
					<div
						key={pack.title}
						className="flex flex-col justify-between rounded-xl border border-border/40 bg-zinc-900/40 p-5 backdrop-blur-sm transition-all hover:border-primary/40 hover:bg-zinc-900/60"
					>
						<div className="flex flex-col gap-2">
							<div className="flex items-center justify-between">
								<span className="rounded-full bg-primary/10 px-2 py-0.5 font-medium text-primary text-xs">
									{pack.version}
								</span>
								<span className="text-muted-foreground text-xs">{pack.downloads} downloads</span>
							</div>
							<h3 className="font-semibold text-foreground text-lg">{pack.title}</h3>
							<p className="text-muted-foreground text-xs leading-relaxed">{pack.description}</p>
						</div>

						<div className="mt-4 flex items-center justify-between border-border/30 border-t pt-3">
							<span className="text-muted-foreground text-xs">by {pack.author}</span>
							<Button size="sm" variant="outline" className="gap-1.5 text-xs">
								<Download className="size-3.5" />
								Install
							</Button>
						</div>
					</div>
				))}
			</div>
		</div>
	)
}

ModpacksPage.displayName = "ModpacksPage"

const MemoizedModpacksPage = memo(ModpacksPage)

export const Route = createFileRoute("/modpacks")({
	component: MemoizedModpacksPage,
})
