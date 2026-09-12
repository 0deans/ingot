import { createFileRoute } from "@tanstack/react-router"
import { memo } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"

const SettingsPage = () => {
	return (
		<div className="flex max-w-3xl flex-1 flex-col gap-8">
			<div>
				<h1 className="font-bold text-2xl text-foreground tracking-tight">Launcher Settings</h1>
				<p className="text-muted-foreground text-sm">
					Configure global Java runtimes, RAM allocation, and launcher behavior.
				</p>
			</div>

			<div className="flex flex-col gap-6">
				{/* Java Runtime */}
				<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/40 p-5">
					<h3 className="font-semibold text-foreground text-sm">Default Java Runtime</h3>
					<p className="text-muted-foreground text-xs">
						Java 21 is required for Minecraft 1.20.5 and newer.
					</p>
					<div className="mt-2 flex gap-3">
						<Input
							defaultValue="C:\Program Files\Eclipse Adoptium\jdk-21.0.3.9-hotspot\bin\javaw.exe"
							className="font-mono text-xs"
						/>
						<Button variant="outline">Browse</Button>
					</div>
				</div>

				{/* Memory Allocation */}
				<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/40 p-5">
					<h3 className="font-semibold text-foreground text-sm">Memory Allocation (RAM)</h3>
					<p className="text-muted-foreground text-xs">
						Allocate maximum RAM for Minecraft instances.
					</p>
					<div className="mt-2 flex items-center gap-4">
						<Input type="number" defaultValue={4096} className="w-32 font-mono text-xs" />
						<span className="text-muted-foreground text-sm">
							MB (Recommended: 4096 MB – 8192 MB)
						</span>
					</div>
				</div>

				{/* Window & Launch Behavior */}
				<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/40 p-5">
					<h3 className="font-semibold text-foreground text-sm">Launcher Behavior</h3>
					<p className="text-muted-foreground text-xs">
						Choose what happens when an instance is launched.
					</p>
					<div className="mt-3 flex gap-2">
						<Button size="sm" variant="outline">
							Keep Launcher Open
						</Button>
						<Button size="sm" variant="outline">
							Hide Launcher to System Tray
						</Button>
					</div>
				</div>
			</div>
		</div>
	)
}

SettingsPage.displayName = "SettingsPage"

const MemoizedSettingsPage = memo(SettingsPage)

export const Route = createFileRoute("/settings")({
	component: MemoizedSettingsPage,
})
