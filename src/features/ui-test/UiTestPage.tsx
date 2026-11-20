import { Activity, FlaskConical, Layers, Trash2 } from "lucide-react"
import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { TAG_LABEL_KEYS } from "@/features/play/types"
import ToastLabSection from "@/features/ui-test/components/ToastLabSection"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { cn } from "@/shared/lib/utils"
import { COLOR_TAGS, TAG_ORDER } from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore, type NavVariant } from "@/shared/stores/useLauncherPrefsStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"
import { type AppThemeId } from "@/shared/theme/customTheme"

type DemoTone = "alpha" | "beta" | "gamma"

const UiTestPage = () => {
	const { t } = useTranslation()
	const { theme, setTheme, navVariant, setNavVariant } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				theme: s.theme,
				setTheme: s.setTheme,
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

	const demoOptions: SegmentedOption<DemoTone>[] = useMemo(() => {
		return [
			{ value: "alpha", label: t("uiLab.demoOptions.alpha"), dotClass: "bg-primary" },
			{ value: "beta", label: t("uiLab.demoOptions.beta"), dotClass: "bg-amber-400" },
			{ value: "gamma", label: t("uiLab.demoOptions.gamma"), dotClass: "bg-cyan-400" },
		]
	}, [t])

	const variantOptions: SegmentedOption<NavVariant>[] = useMemo(() => {
		return [
			{ value: "sonoma", label: t("uiLab.variants.sonoma") },
			{ value: "rail", label: t("uiLab.variants.rail") },
			{ value: "sequoia", label: t("uiLab.variants.sequoia") },
			{ value: "capsule", label: t("uiLab.variants.capsule") },
		]
	}, [t])

	const themeSegmentOptions: SegmentedOption<AppThemeId>[] = useMemo(() => {
		return [
			{ value: "light", label: t("uiLab.themes.light") },
			{ value: "dark", label: t("uiLab.themes.dark") },
			{ value: "purple", label: t("uiLab.themes.purple") },
			{ value: "custom", label: t("uiLab.themes.custom") },
		]
	}, [t])

	return (
		<div className="@container flex min-w-0 flex-col gap-5">
			<div className="flex items-center justify-between gap-3">
				<div className="min-w-0">
					<div className="flex flex-wrap items-center gap-2">
						<FlaskConical className="size-4.5 text-primary" />
						<h1 className="text-lg font-semibold tracking-tight text-foreground">
							{t("uiLab.title")}
						</h1>
						<span className="rounded-full border border-primary/30 bg-primary/15 px-2 py-0.5 text-[10px] font-semibold text-primary">
							{t("uiLab.devOnlyBadge")}
						</span>
					</div>
					<p className="mt-0.5 text-[12px] text-muted-foreground">{t("uiLab.subtitle")}</p>
				</div>
			</div>

			<div className="grid grid-cols-1 gap-4 @2xl:grid-cols-2">
				<section className="flex min-w-0 flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4">
					<div className="flex items-center gap-2">
						<Layers className="size-4 text-primary" />
						<h2 className="text-[13.5px] font-semibold text-foreground">
							{t("uiLab.liveShellControls")}
						</h2>
					</div>
					<div className="flex min-w-0 flex-col gap-2">
						<span className="text-[11px] text-muted-foreground">
							{t("uiLab.navigationVariant")}
						</span>
						<SegmentedControl
							layoutId="uitest-nav-variant"
							options={variantOptions}
							value={navVariant}
							onChange={setNavVariant}
							tone="emerald"
						/>
					</div>
					<div className="flex min-w-0 flex-col gap-2">
						<span className="text-[11px] text-muted-foreground">{t("uiLab.activeTheme")}</span>
						<SegmentedControl
							layoutId="uitest-theme"
							options={themeSegmentOptions}
							value={theme}
							onChange={setTheme}
							tone="emerald"
						/>
					</div>
				</section>

				<section className="flex min-w-0 flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4">
					<div className="flex items-center gap-2">
						<Activity className="size-4 text-primary" />
						<h2 className="text-[13.5px] font-semibold text-foreground">
							{t("uiLab.processSimulator", { count: runningProcesses.length })}
						</h2>
					</div>
					<div className="flex flex-wrap gap-2">
						<button
							type="button"
							onClick={() => {
								spawnMockProcess(t("uiLab.debugHorizonProcess"), "instance", "Horizon (Rust)")
							}}
							className="cursor-pointer rounded-xl bg-primary px-3 py-1.5 text-[12px] font-semibold text-primary-foreground transition-opacity hover:opacity-90"
						>
							{t("uiLab.spawnGame")}
						</button>
						<button
							type="button"
							onClick={() => {
								spawnMockProcess(t("uiLab.modrinthSyncServerProcess"), "server", "Java")
							}}
							className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
						>
							{t("uiLab.spawnServer")}
						</button>
						<button
							type="button"
							onClick={stopAllProcesses}
							className="flex cursor-pointer items-center gap-1 rounded-xl border border-rose-500/30 bg-rose-500/12 px-3 py-1.5 text-[12px] font-medium text-rose-500 transition-colors hover:bg-rose-500/22 dark:text-rose-300"
						>
							<Trash2 className="size-3.5" />
							<span>{t("uiLab.clearAll")}</span>
						</button>
					</div>
				</section>
			</div>

			<section className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4">
				<h2 className="text-[13.5px] font-semibold text-foreground">
					{t("uiLab.segmentedControlsHeading")}
				</h2>
				<div className="flex flex-wrap items-center gap-4">
					<SegmentedControl
						layoutId="uitest-neutral"
						options={demoOptions}
						value={neutralDemo}
						onChange={setNeutralDemo}
						tone="neutral"
					/>
					<SegmentedControl
						layoutId="uitest-emerald"
						options={demoOptions}
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
								<span>{t(TAG_LABEL_KEYS[tagId])}</span>
							</span>
						)
					})}
				</div>
			</section>

			<ToastLabSection />
		</div>
	)
}

export default UiTestPage
