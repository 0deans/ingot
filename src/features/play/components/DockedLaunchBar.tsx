import { Package, Play, RefreshCw } from "lucide-react"
import { motion } from "motion/react"
import { useTranslation } from "react-i18next"
import { ENGINE_LABEL_KEYS } from "@/features/play/types"
import { cn } from "@/shared/lib/utils"
import { COLOR_TAGS, type GameInstance } from "@/shared/stores/useInstancesStore"

interface DockedLaunchBarProps {
	instance: GameInstance
	username: string
	ramGb: number
	isLaunching: boolean
	onLaunch: () => void
}

const DockedLaunchBar = ({
	instance,
	username,
	ramGb,
	isLaunching,
	onLaunch,
}: DockedLaunchBarProps) => {
	const { t } = useTranslation()

	const tagDotClass = COLOR_TAGS[instance.colorTag].dot
	const runtimeLabel = instance.jvmArgs || `JVM ${ramGb} GB`

	return (
		<motion.footer
			key="desktop-launch-bar"
			initial={{ opacity: 0, height: 0 }}
			animate={{ opacity: 1, height: 68 }}
			exit={{ opacity: 0, height: 0 }}
			transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
			className="z-10 flex h-17 shrink-0 items-center justify-between gap-4 overflow-hidden border-t border-border bg-sidebar/85 px-5 backdrop-blur-2xl"
		>
			<motion.div
				key={instance.id}
				initial={{ opacity: 0, x: -8 }}
				animate={{ opacity: 1, x: 0 }}
				transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
				className="flex min-w-0 items-center gap-3.5"
			>
				<div className="relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-primary/30 bg-primary/15 text-primary">
					<Package className="h-5 w-5" />
					<span
						className={cn(
							"absolute -top-0.5 -right-0.5 h-2.5 w-2.5 rounded-full ring-2 ring-sidebar",
							tagDotClass,
						)}
					/>
				</div>
				<div className="min-w-0 space-y-0.5">
					<div className="flex items-center gap-2">
						<span className="truncate text-xs font-bold text-foreground">{instance.name}</span>
						<span className="rounded-md border border-border bg-foreground/6 px-1.5 py-0.5 font-mono text-[10px] leading-none text-foreground/80">
							{t(ENGINE_LABEL_KEYS[instance.engine])} • {instance.version}
						</span>
					</div>
					<div className="truncate text-[11px] leading-tight text-muted-foreground">
						{t("play.launchBar.playingAs")}{" "}
						<span className="font-medium text-foreground">{username}</span> • {runtimeLabel}
					</div>
				</div>
			</motion.div>

			<motion.button
				type="button"
				whileHover={{ scale: 1.03 }}
				whileTap={{ scale: 0.95 }}
				onClick={onLaunch}
				className="flex h-10 shrink-0 cursor-pointer items-center gap-2 rounded-xl bg-primary px-5 text-xs font-bold text-primary-foreground shadow-lg shadow-primary/20 transition-opacity hover:opacity-90"
			>
				{isLaunching ? (
					<>
						<RefreshCw className="h-4 w-4 animate-spin" />
						<span>{t("play.launchBar.verifying")}</span>
					</>
				) : (
					<>
						<Play className="h-4 w-4 fill-current" />
						<span>{t("play.launchBar.launchInstance")}</span>
					</>
				)}
			</motion.button>
		</motion.footer>
	)
}

export default DockedLaunchBar
