import { Link } from "@tanstack/react-router"
import { ArrowUpRight, Code2 } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { appleToast } from "@/shared/ui/sonner"

const DeveloperSettingsCard = () => {
	const { t } = useTranslation()
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
		appleToast.info(
			next ? t("settings.developer.pagesEnabledToast") : t("settings.developer.pagesHiddenToast"),
			{
				description: next
					? t("settings.developer.pagesEnabledDescription")
					: t("settings.developer.pagesHiddenDescription"),
			},
		)
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
					<h2 className="text-[13.5px] font-semibold text-foreground">
						{t("settings.developer.title")}
					</h2>
				</div>

				{showDeveloperPages ? (
					<Link
						to="/ui-test"
						className="inline-flex items-center gap-1 rounded-lg border border-primary/30 bg-primary/12 px-2.5 py-1 text-[11.5px] font-medium text-primary transition-colors hover:bg-primary/20"
					>
						<span>{t("settings.developer.openUiLab")}</span>
						<ArrowUpRight className="size-3.5" />
					</Link>
				) : null}
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
				<button
					type="button"
					role="switch"
					aria-checked={showDeveloperPages}
					onClick={handleToggleDeveloperPages}
					className={cn(
						"shrink-0 cursor-pointer rounded-full px-3 py-1 text-[11px] font-semibold whitespace-nowrap transition-colors",
						showDeveloperPages
							? "bg-primary text-primary-foreground"
							: "bg-foreground/10 text-muted-foreground",
					)}
				>
					{showDeveloperPages ? t("common.enabled") : t("common.disabled")}
				</button>
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
				<button
					type="button"
					role="switch"
					aria-checked={enableDevContextMenu}
					onClick={handleToggleContextMenu}
					className={cn(
						"shrink-0 cursor-pointer rounded-full px-3 py-1 text-[11px] font-semibold whitespace-nowrap transition-colors",
						enableDevContextMenu
							? "bg-primary text-primary-foreground"
							: "bg-foreground/10 text-muted-foreground",
					)}
				>
					{enableDevContextMenu ? t("common.enabled") : t("common.disabled")}
				</button>
			</div>
		</section>
	)
}

export default DeveloperSettingsCard
