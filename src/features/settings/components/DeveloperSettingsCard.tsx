import { Link } from "@tanstack/react-router"
import { ArrowUpRight, Code2 } from "lucide-react"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { appleToast } from "@/shared/ui/sonner"

const DeveloperSettingsCard = () => {
	const {
		showDeveloperPages,
		setShowDeveloperPages,
		enableDevContextMenu,
		setEnableDevContextMenu,
	} = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				showDeveloperPages: s.showDeveloperPages,
				setShowDeveloperPages: s.setShowDeveloperPages,
				enableDevContextMenu: s.enableDevContextMenu,
				setEnableDevContextMenu: s.setEnableDevContextMenu,
			}
		}),
	)

	const handleToggleDeveloperPages = () => {
		const next = !showDeveloperPages
		setShowDeveloperPages(next)
		appleToast.info(next ? "Developer pages enabled" : "Developer pages hidden", {
			description: next
				? "UI Lab is now visible in launcher navigation."
				: "UI Lab and developer routes have been hidden from navigation.",
		})
	}

	const handleToggleContextMenu = () => {
		const next = !enableDevContextMenu
		setEnableDevContextMenu(next)
	}

	return (
		<section className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="flex flex-wrap items-center justify-between gap-2">
				<div className="flex items-center gap-2">
					<Code2 className="size-4 text-primary" />
					<h2 className="text-[13.5px] font-semibold text-foreground">Developer Options</h2>
				</div>

				{showDeveloperPages ? (
					<Link
						to="/ui-test"
						className="inline-flex items-center gap-1 rounded-lg border border-primary/30 bg-primary/12 px-2.5 py-1 text-[11.5px] font-medium text-primary transition-colors hover:bg-primary/20"
					>
						<span>Open UI Lab</span>
						<ArrowUpRight className="size-3.5" />
					</Link>
				) : null}
			</div>

			<div className="flex items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0">
					<p className="text-[12.5px] font-medium text-foreground">Show Developer Pages</p>
					<p className="text-[11px] text-muted-foreground">
						Expose UI Design Lab and internal sandboxes in sidebar, capsule bar, and mobile menus.
					</p>
				</div>
				<button
					type="button"
					role="switch"
					aria-checked={showDeveloperPages}
					onClick={handleToggleDeveloperPages}
					className={cn(
						"shrink-0 cursor-pointer rounded-full px-3 py-1 text-[11px] font-semibold transition-colors",
						showDeveloperPages
							? "bg-primary text-primary-foreground"
							: "bg-foreground/10 text-muted-foreground",
					)}
				>
					{showDeveloperPages ? "Enabled" : "Disabled"}
				</button>
			</div>

			<div className="flex items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0">
					<p className="text-[12.5px] font-medium text-foreground">Allow Inspect Context Menu</p>
					<p className="text-[11px] text-muted-foreground">
						Keep the native right-click context menu enabled for element inspection inside desktop
						windows.
					</p>
				</div>
				<button
					type="button"
					role="switch"
					aria-checked={enableDevContextMenu}
					onClick={handleToggleContextMenu}
					className={cn(
						"shrink-0 cursor-pointer rounded-full px-3 py-1 text-[11px] font-semibold transition-colors",
						enableDevContextMenu
							? "bg-primary text-primary-foreground"
							: "bg-foreground/10 text-muted-foreground",
					)}
				>
					{enableDevContextMenu ? "Enabled" : "Disabled"}
				</button>
			</div>
		</section>
	)
}

export default DeveloperSettingsCard
