import { createFileRoute } from "@tanstack/react-router"
import { Folder, HardDrive, Image, Layers, Map as MapIcon } from "lucide-react"
import { memo } from "react"
import { Button } from "@/components/ui/button"

const StoragePage = () => {
	const items = [
		{
			name: "Instances Directory",
			path: "C:\\Users\\...\\.ingot\\instances",
			size: "14.2 GB",
			icon: HardDrive,
		},
		{
			name: "Worlds & Saves",
			path: "C:\\Users\\...\\.ingot\\saves",
			size: "4.8 GB",
			icon: MapIcon,
		},
		{
			name: "Screenshots",
			path: "C:\\Users\\...\\.ingot\\screenshots",
			size: "340 MB",
			icon: Image,
		},
		{
			name: "Resource Packs & Shaders",
			path: "C:\\Users\\...\\.ingot\\resourcepacks",
			size: "1.6 GB",
			icon: Layers,
		},
	]

	return (
		<div className="flex flex-1 flex-col gap-6">
			<div>
				<h1 className="font-bold text-2xl text-foreground tracking-tight">Storage & Saves</h1>
				<p className="text-muted-foreground text-sm">
					Manage disk usage, worlds, screenshots, and downloaded mods.
				</p>
			</div>

			<div className="grid grid-cols-1 gap-4 md:grid-cols-2">
				{items.map((item) => {
					const Icon = item.icon
					return (
						<div
							key={item.name}
							className="flex items-center justify-between rounded-xl border border-border/40 bg-zinc-900/40 p-5 backdrop-blur-sm"
						>
							<div className="flex items-center gap-4">
								<div className="flex size-11 items-center justify-center rounded-xl bg-primary/10 text-primary">
									<Icon className="size-5" />
								</div>
								<div>
									<h4 className="font-semibold text-foreground text-sm">{item.name}</h4>
									<span className="font-mono text-muted-foreground text-xs">{item.path}</span>
								</div>
							</div>

							<div className="flex items-center gap-3">
								<span className="font-medium text-foreground text-sm">{item.size}</span>
								<Button size="sm" variant="outline" className="gap-1.5 text-xs">
									<Folder className="size-3.5" />
									Open
								</Button>
							</div>
						</div>
					)
				})}
			</div>
		</div>
	)
}

StoragePage.displayName = "StoragePage"

const MemoizedStoragePage = memo(StoragePage)

export const Route = createFileRoute("/storage")({
	component: MemoizedStoragePage,
})
