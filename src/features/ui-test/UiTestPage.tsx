import { Activity, FlaskConical, Layers, Trash2 } from "lucide-react"
import { useTheme } from "next-themes"
import { useState } from "react"
import { useShallow } from "zustand/react/shallow"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { cn } from "@/shared/lib/utils"
import { COLOR_TAGS, TAG_ORDER } from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore, type NavVariant } from "@/shared/stores/useLauncherPrefsStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"
import { isAppThemeId, type AppThemeId } from "@/shared/theme/customTheme"

type DemoTone = "alpha" | "beta" | "gamma"

const DEMO_OPTIONS: SegmentedOption<DemoTone>[] = [
	{ value: "alpha", label: "Alpha", dotClass: "bg-primary" },
	{ value: "beta", label: "Beta", dotClass: "bg-amber-400" },
	{ value: "gamma", label: "Gamma", dotClass: "bg-cyan-400" },
]

const VARIANT_OPTIONS: SegmentedOption<NavVariant>[] = [
	{ value: "sonoma", label: "Sonoma" },
	{ value: "rail", label: "Rail" },
	{ value: "sequoia", label: "Sequoia" },
	{ value: "capsule", label: "Capsule" },
]

const THEME_SEGMENT_OPTIONS: SegmentedOption<AppThemeId>[] = [
	{ value: "light", label: "Light" },
	{ value: "dark", label: "Dark" },
	{ value: "purple", label: "Purple" },
	{ value: "custom", label: "Custom" },
]

const UiTestPage = () => {
	const { theme, setTheme } = useTheme()
	const { navVariant, setNavVariant } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				navVariant: s.navVariant,
				setNavVariant: s.setNavVariant,
			}
		}),
	)

	const { runningProcesses, spawnMockProcess, stopAllProcesses } = useProcessStore(
		useShallow((s) => {
			return {
				runningProcesses: s.runningProcesses,
				spawnMockProcess: s.spawnMockProcess,
				stopAllProcesses: s.stopAllProcesses,
			}
		}),
	)

	const [neutralDemo, setNeutralDemo] = useState<DemoTone>("alpha")
	const [emeraldDemo, setEmeraldDemo] = useState<DemoTone>("beta")

	const activeTheme: AppThemeId = isAppThemeId(theme) ? theme : "dark"

	return (
		<div className="flex flex-col gap-5">
			<div className="flex items-center justify-between gap-3">
				<div>
					<div className="flex items-center gap-2">
						<FlaskConical className="size-4.5 text-primary" />
						<h1 className="text-lg font-semibold tracking-tight text-foreground">UI Design Lab</h1>
						<span className="rounded-full border border-primary/30 bg-primary/15 px-2 py-0.5 text-[10px] font-semibold text-primary">
							DEV ONLY
						</span>
					</div>
					<p className="mt-0.5 text-[12px] text-muted-foreground">
						Interactive sandbox for themes, shell navigation variants, spring pills, and live
						process badges.
					</p>
				</div>
			</div>

			<div className="grid grid-cols-1 gap-4 md:grid-cols-2">
				<section className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4">
					<div className="flex items-center gap-2">
						<Layers className="size-4 text-primary" />
						<h2 className="text-[13.5px] font-semibold text-foreground">Live Shell Controls</h2>
					</div>
					<div className="flex flex-col gap-2">
						<span className="text-[11px] text-muted-foreground">Navigation Variant</span>
						<SegmentedControl
							layoutId="uitest-nav-variant"
							options={VARIANT_OPTIONS}
							value={navVariant}
							onChange={setNavVariant}
							tone="emerald"
						/>
					</div>
					<div className="flex flex-col gap-2">
						<span className="text-[11px] text-muted-foreground">Active Theme</span>
						<SegmentedControl
							layoutId="uitest-theme"
							options={THEME_SEGMENT_OPTIONS}
							value={activeTheme}
							onChange={(nextTheme) => {
								setTheme(nextTheme)
							}}
							tone="emerald"
						/>
					</div>
				</section>

				<section className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4">
					<div className="flex items-center gap-2">
						<Activity className="size-4 text-primary" />
						<h2 className="text-[13.5px] font-semibold text-foreground">
							Process Simulator ({runningProcesses.length} Active)
						</h2>
					</div>
					<div className="flex flex-wrap gap-2">
						<button
							type="button"
							onClick={() => {
								spawnMockProcess("Debug Horizon Session", "instance", "Horizon (Rust)")
							}}
							className="cursor-pointer rounded-xl bg-primary px-3 py-1.5 text-[12px] font-semibold text-primary-foreground transition-opacity hover:opacity-90"
						>
							+ Spawn Game
						</button>
						<button
							type="button"
							onClick={() => {
								spawnMockProcess("Modrinth Sync Server", "server", "Java")
							}}
							className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
						>
							+ Spawn Server
						</button>
						<button
							type="button"
							onClick={stopAllProcesses}
							className="flex cursor-pointer items-center gap-1 rounded-xl border border-rose-500/30 bg-rose-500/12 px-3 py-1.5 text-[12px] font-medium text-rose-500 transition-colors hover:bg-rose-500/22 dark:text-rose-300"
						>
							<Trash2 className="size-3.5" />
							<span>Clear All</span>
						</button>
					</div>
				</section>
			</div>

			<section className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4">
				<h2 className="text-[13.5px] font-semibold text-foreground">
					Segmented Controls & Color Tags
				</h2>
				<div className="flex flex-wrap items-center gap-4">
					<SegmentedControl
						layoutId="uitest-neutral"
						options={DEMO_OPTIONS}
						value={neutralDemo}
						onChange={setNeutralDemo}
						tone="neutral"
					/>
					<SegmentedControl
						layoutId="uitest-emerald"
						options={DEMO_OPTIONS}
						value={emeraldDemo}
						onChange={setEmeraldDemo}
						tone="emerald"
					/>
				</div>

				<div className="mt-2 flex flex-wrap gap-2">
					{TAG_ORDER.map((tagId) => {
						const meta = COLOR_TAGS[tagId]
						return (
							<span
								key={tagId}
								className={cn(
									"inline-flex items-center gap-1.5 rounded-full border border-border px-2.5 py-1 text-[11.5px] font-medium",
									meta.badge,
								)}
							>
								<span className={cn("size-2 rounded-full", meta.dot)} />
								<span>{meta.name}</span>
							</span>
						)
					})}
				</div>
			</section>
		</div>
	)
}

export default UiTestPage
