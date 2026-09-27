import { openUrl } from "@tauri-apps/plugin-opener"
import { ExternalLink, Scale } from "lucide-react"
import { useState } from "react"
import { FadeScroll } from "@/components/servers/shared/primitives"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"

interface Component {
	name: string
	/** What Ingot uses it for */
	role: string
	license: string
	source: string
}

interface Group {
	title: string
	note: string
	items: Component[]
}

/**
 * Third-party software Ingot ships or downloads and runs. Keep in sync with
 * src-tauri/android/jniLibs/README.md (bundled files) and the download code.
 */
const GROUPS: Group[] = [
	{
		title: "Included in the Android app",
		note: "Runs server software on Android inside a Linux sandbox. Builds from the Termux project.",
		items: [
			{
				name: "PRoot",
				role: "Linux sandbox for servers",
				license: "GPL-2.0",
				source: "https://github.com/termux/proot",
			},
			{
				name: "talloc",
				role: "Memory library used by PRoot",
				license: "LGPL-3.0-or-later",
				source: "https://talloc.samba.org",
			},
			{
				name: "libandroid-shmem",
				role: "Shared memory support for PRoot",
				license: "BSD-3-Clause",
				source: "https://github.com/termux/libandroid-shmem",
			},
		],
	},
	{
		title: "Downloaded when needed",
		note: "Fetched from their official sources the first time a feature uses them.",
		items: [
			{
				name: "Eclipse Temurin (OpenJDK)",
				role: "Java for servers and the game on desktop",
				license: "GPL-2.0 with Classpath Exception",
				source: "https://adoptium.net",
			},
			{
				name: "Alpine Linux",
				role: "Base system of the Android sandbox, including its OpenJDK",
				license: "Various open-source licenses",
				source: "https://alpinelinux.org",
			},
			{
				name: "Pumpkin",
				role: "Pumpkin server",
				license: "GPL-3.0",
				source: "https://github.com/Pumpkin-MC/Pumpkin",
			},
			{
				name: "playit.gg agent",
				role: "Public address for your server",
				license: "BSD-2-Clause",
				source: "https://github.com/playit-cloud/playit-agent",
			},
		],
	},
	{
		title: "Server software you can install",
		note: "Downloaded for each server you create, from the project that makes it.",
		items: [
			{
				name: "Paper",
				role: "Paper servers",
				license: "GPL-3.0",
				source: "https://github.com/PaperMC/Paper",
			},
			{
				name: "Folia",
				role: "Folia servers",
				license: "GPL-3.0",
				source: "https://github.com/PaperMC/Folia",
			},
			{
				name: "Purpur",
				role: "Purpur servers",
				license: "MIT",
				source: "https://github.com/PurpurMC/Purpur",
			},
			{
				name: "Fabric Loader",
				role: "Fabric servers",
				license: "Apache-2.0",
				source: "https://github.com/FabricMC/fabric-loader",
			},
			{
				name: "Minecraft server",
				role: "Vanilla servers (by Mojang, not open source)",
				license: "Minecraft EULA",
				source: "https://aka.ms/MinecraftEULA",
			},
		],
	},
]

/** Lists the third-party software Ingot ships or downloads, with licenses and sources */
export function LicensesDialog({
	open,
	onOpenChange,
}: {
	open: boolean
	onOpenChange: (open: boolean) => void
}) {
	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="flex max-h-[85vh] flex-col gap-3 p-0 sm:max-w-lg">
				<div className="px-5 pt-5">
					<DialogTitle className="text-base">Third-party licenses</DialogTitle>
					<p className="mt-1 text-xs text-zinc-500 leading-relaxed">
						Ingot uses this software from other projects. Each one is under its own license; tap a
						name for its source code and full license.
					</p>
				</div>
				<FadeScroll className="flex min-h-0 flex-col gap-5 px-5 pb-5">
					{GROUPS.map((group) => (
						<section key={group.title} className="flex flex-col gap-2">
							<div>
								<h3 className="font-semibold text-sm text-zinc-100">{group.title}</h3>
								<p className="text-[11px] text-zinc-500">{group.note}</p>
							</div>
							<ul className="flex flex-col divide-y divide-zinc-800/70 border border-zinc-800/70">
								{group.items.map((item) => (
									<li key={item.name}>
										<button
											type="button"
											onClick={() => openUrl(item.source).catch(console.error)}
											className="flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors hover:bg-zinc-900/60"
										>
											<div className="min-w-0 flex-1">
												<p className="truncate font-medium text-sm text-zinc-100">{item.name}</p>
												<p className="truncate text-[11px] text-zinc-500">{item.role}</p>
											</div>
											<span className="shrink-0 bg-zinc-900 px-1.5 py-0.5 font-mono text-[10px] text-zinc-300">
												{item.license}
											</span>
											<ExternalLink className="size-3.5 shrink-0 text-zinc-600" />
										</button>
									</li>
								))}
							</ul>
						</section>
					))}
					<p className="text-[11px] text-zinc-600 leading-relaxed">
						Minecraft is a trademark of Mojang AB. Ingot is not affiliated with Mojang or Microsoft.
					</p>
				</FadeScroll>
			</DialogContent>
		</Dialog>
	)
}

/** Settings card that opens the licenses list */
export function LicensesSettings() {
	const [open, setOpen] = useState(false)
	return (
		<div className="flex items-center justify-between gap-4 border border-border/30 bg-zinc-950/40 p-4">
			<div className="flex min-w-0 items-center gap-3">
				<Scale className="size-4 shrink-0 text-muted-foreground" />
				<div className="min-w-0">
					<p className="font-medium text-sm">Third-party licenses</p>
					<p className="text-muted-foreground text-xs">
						Open-source software Ingot includes or downloads.
					</p>
				</div>
			</div>
			<Button variant="outline" size="sm" onClick={() => setOpen(true)} className="shrink-0">
				View
			</Button>
			<LicensesDialog open={open} onOpenChange={setOpen} />
		</div>
	)
}
