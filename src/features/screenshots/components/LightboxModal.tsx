import { Camera, X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect } from "react"
import { useTranslation } from "react-i18next"
import type { ScreenshotItem } from "@/features/screenshots/components/ScreenshotGrid"
import { cn } from "@/shared/lib/utils"

interface LightboxModalProps {
	shot: ScreenshotItem | null
	onClose: () => void
}

const LightboxModal = ({ shot, onClose }: LightboxModalProps) => {
	const { t } = useTranslation()

	useEffect(() => {
		if (!shot) {
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
	}, [onClose, shot])

	const shotTitle = shot ? t(shot.titleKey) : ""

	return (
		<AnimatePresence>
			{shot ? (
				<div className="fixed inset-0 z-50 flex items-center justify-center p-4">
					<button
						type="button"
						aria-label={t("screenshots.closeLightboxAria")}
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-black/75 backdrop-blur-sm"
					/>
					<motion.div
						role="dialog"
						aria-label={shotTitle}
						initial={{ opacity: 0, scale: 0.96 }}
						animate={{ opacity: 1, scale: 1 }}
						exit={{ opacity: 0, scale: 0.96 }}
						transition={{ duration: 0.17, ease: [0.22, 1, 0.36, 1] }}
						className="ingot-glass-elevated relative z-10 w-full max-w-2xl overflow-hidden rounded-2xl shadow-2xl"
					>
						<div className={cn("relative flex h-80 items-end bg-linear-to-br p-5", shot.gradient)}>
							<button
								type="button"
								onClick={onClose}
								aria-label={t("screenshots.dismissPreviewAria")}
								className="absolute top-3 right-3 flex size-8 cursor-pointer items-center justify-center rounded-xl border border-white/15 bg-black/50 text-zinc-200 hover:bg-black/75"
							>
								<X className="size-4" />
							</button>
							<div className="flex items-center gap-2 rounded-full border border-white/15 bg-black/50 px-3 py-1 text-xs text-zinc-200">
								<Camera className="size-3.5 text-emerald-400" />
								<span>{shot.resolution}</span>
							</div>
						</div>
						<div className="flex items-center justify-between p-4">
							<div>
								<h2 className="text-sm font-semibold text-foreground">{shotTitle}</h2>
								<p className="text-xs text-muted-foreground">
									{shot.instance} · {shot.shaderPack} · {t(shot.capturedAtKey)}
								</p>
							</div>
						</div>
					</motion.div>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default LightboxModal
