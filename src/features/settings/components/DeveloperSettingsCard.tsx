import { Code2, RotateCcw } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { Button } from "@/shared/ui/button"
import { Switch } from "@/shared/ui/switch"

const DeveloperSettingsCard = () => {
	const { t } = useTranslation()
	const {
		showDeveloperPages,
		setShowDeveloperPages,
		enableDevContextMenu,
		setEnableDevContextMenu,
		setOnboardingCompleted,
	} = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				showDeveloperPages: s.showDeveloperPages,
				setShowDeveloperPages: s.setShowDeveloperPages,
				enableDevContextMenu: s.enableDevContextMenu,
				setEnableDevContextMenu: s.setEnableDevContextMenu,
				setOnboardingCompleted: s.setOnboardingCompleted,
			}
		}),
	)

	const handleToggleDeveloperPages = () => {
		setShowDeveloperPages(!showDeveloperPages)
	}

	const handleToggleContextMenu = () => {
		setEnableDevContextMenu(!enableDevContextMenu)
	}

	const handleReplayOnboarding = () => {
		setOnboardingCompleted(false)
	}

	return (
		<section className="@container flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-3.5 backdrop-blur-md sm:p-4">
			<div className="flex items-center gap-2.5">
				<div className="flex size-7 shrink-0 items-center justify-center rounded-md border border-border bg-foreground/5 text-primary">
					<Code2 className="size-4" />
				</div>
				<h2 className="text-[13.5px] font-semibold text-foreground">
					{t("settings.developer.title")}
				</h2>
			</div>

			<div className="flex items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0 flex-1">
					<p className="text-[12.5px] font-medium text-foreground">
						{t("settings.developer.showPagesTitle")}
					</p>
					<p className="text-[11px] leading-relaxed text-muted-foreground">
						{t("settings.developer.showPagesDescription")}
					</p>
				</div>
				<Switch
					checked={showDeveloperPages}
					onCheckedChange={handleToggleDeveloperPages}
					aria-label={t("settings.developer.showPagesTitle")}
				/>
			</div>

			<div className="flex items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0 flex-1">
					<p className="text-[12.5px] font-medium text-foreground">
						{t("settings.developer.contextMenuTitle")}
					</p>
					<p className="text-[11px] leading-relaxed text-muted-foreground">
						{t("settings.developer.contextMenuDescription")}
					</p>
				</div>
				<Switch
					checked={enableDevContextMenu}
					onCheckedChange={handleToggleContextMenu}
					aria-label={t("settings.developer.contextMenuTitle")}
				/>
			</div>

			<div className="flex items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0 flex-1">
					<p className="text-[12.5px] font-medium text-foreground">
						{t("settings.developer.replayOnboardingTitle")}
					</p>
					<p className="text-[11px] leading-relaxed text-muted-foreground">
						{t("settings.developer.replayOnboardingDescription")}
					</p>
				</div>
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={handleReplayOnboarding}
					className="shrink-0 cursor-pointer"
				>
					<RotateCcw className="size-3.5" />
					<span>{t("settings.developer.replayAction")}</span>
				</Button>
			</div>
		</section>
	)
}

export default DeveloperSettingsCard
