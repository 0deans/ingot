import { Copy, ExternalLink, Palette, Play, Square } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect } from "react"
import { useTranslation } from "react-i18next"
import { ENGINE_LABEL_KEYS, type GameInstance } from "@/features/play/types"

export interface ContextMenuState {
	instance: GameInstance
	x: number
	y: number
}

interface InstanceContextMenuProps {
	state: ContextMenuState | null
	isRunning: boolean
	onClose: () => void
	onOpenDetails: (id: string) => void
	onLaunchOrStop: (instance: GameInstance) => void
	onCycleTag: (id: string) => void
	onClone: (instance: GameInstance) => void
}

const InstanceContextMenu = ({
	state,
	isRunning,
	onClose,
	onOpenDetails,
	onLaunchOrStop,
	onCycleTag,
	onClone,
}: InstanceContextMenuProps) => {
	const { t } = useTranslation()

	useEffect(() => {
		if (!state) {
			return
		}
		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				onClose()
			}
		}
		window.addEventListener("keydown", handleKeyDown)
		return () => {
			window.removeEventListener("keydown", handleKeyDown)
		}
	}, [onClose, state])

	const clampedX = state
		? Math.min(state.x, typeof window !== "undefined" ? window.innerWidth - 220 : state.x)
		: 0
	const clampedY = state
		? Math.min(state.y, typeof window !== "undefined" ? window.innerHeight - 210 : state.y)
		: 0

	return (
		<AnimatePresence>
			{state ? (
				<div className="fixed inset-0 z-50">
					<button
						type="button"
						aria-label={t("play.contextMenu.closeAria")}
						onClick={onClose}
						onContextMenu={(event) => {
							event.preventDefault()
							onClose()
						}}
						className="fixed inset-0 cursor-default bg-transparent"
					/>
					<motion.div
						role="menu"
						aria-label={t("play.contextMenu.actionsAria", { name: state.instance.name })}
						initial={{ opacity: 0, scale: 0.96, y: -4 }}
						animate={{ opacity: 1, scale: 1, y: 0 }}
						exit={{ opacity: 0, scale: 0.96, y: -4 }}
						transition={{ duration: 0.14, ease: [0.22, 1, 0.36, 1] }}
						style={{ left: clampedX, top: clampedY }}
						className="ingot-glass-elevated fixed z-50 flex w-52 flex-col gap-0.5 rounded-xl p-1.5 shadow-2xl"
					>
						<div className="border-b border-border px-2.5 py-1.5">
							<p className="truncate text-[11.5px] font-semibold text-foreground">
								{state.instance.name}
							</p>
							<p className="truncate text-[10px] text-muted-foreground">
								{t(ENGINE_LABEL_KEYS[state.instance.engine])} · {state.instance.version}
							</p>
						</div>

						<button
							type="button"
							role="menuitem"
							onClick={() => {
								onOpenDetails(state.instance.id)
								onClose()
							}}
							className="flex cursor-pointer items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-[12px] text-foreground transition-colors hover:bg-foreground/8"
						>
							<ExternalLink className="size-3.5 text-primary" />
							<span>{t("play.contextMenu.inspect")}</span>
						</button>

						<button
							type="button"
							role="menuitem"
							onClick={() => {
								onLaunchOrStop(state.instance)
								onClose()
							}}
							className="flex cursor-pointer items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-[12px] text-foreground transition-colors hover:bg-foreground/8"
						>
							{isRunning ? (
								<>
									<Square className="size-3.5 fill-current text-rose-500 dark:text-rose-400" />
									<span>{t("play.contextMenu.stopProcess")}</span>
								</>
							) : (
								<>
									<Play className="size-3.5 fill-current text-primary" />
									<span>{t("play.contextMenu.launchNow")}</span>
								</>
							)}
						</button>

						<button
							type="button"
							role="menuitem"
							onClick={() => {
								onCycleTag(state.instance.id)
								onClose()
							}}
							className="flex cursor-pointer items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-[12px] text-foreground transition-colors hover:bg-foreground/8"
						>
							<Palette className="size-3.5 text-amber-500 dark:text-amber-400" />
							<span>{t("play.contextMenu.cycleTag")}</span>
						</button>

						<button
							type="button"
							role="menuitem"
							onClick={() => {
								onClone(state.instance)
								onClose()
							}}
							className="flex cursor-pointer items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-[12px] text-foreground transition-colors hover:bg-foreground/8"
						>
							<Copy className="size-3.5 text-sky-500 dark:text-sky-400" />
							<span>{t("play.contextMenu.duplicate")}</span>
						</button>
					</motion.div>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default InstanceContextMenu
