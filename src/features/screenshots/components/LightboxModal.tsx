import { Camera, X } from "lucide-react"
import { useTranslation } from "react-i18next"
import type { ScreenshotItem } from "@/features/screenshots/components/ScreenshotGrid"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/shared/ui/dialog"

interface LightboxModalProps {
	shot: ScreenshotItem | null
	onClose: () => void
}

const LightboxModal = ({ shot, onClose }: LightboxModalProps) => {
	const { t } = useTranslation()

	const handleOpenChange = (nextOpen: boolean) => {
		if (!nextOpen) {
			onClose()
		}
	}

	const shotTitle = shot ? t(shot.titleKey) : ""

	return (
		<Dialog open={shot !== null} onOpenChange={handleOpenChange}>
			<DialogContent
				showCloseButton={false}
				aria-label={shotTitle}
				className="ingot-glass-elevated gap-0 overflow-hidden rounded-2xl p-0 shadow-2xl sm:max-w-2xl"
			>
				{shot ? (
					<>
						<div className={cn("relative flex h-80 items-end bg-linear-to-br p-5", shot.gradient)}>
							<Button
								type="button"
								variant="ghost"
								size="icon"
								onClick={onClose}
								aria-label={t("screenshots.dismissPreviewAria")}
								className="absolute top-3 right-3 cursor-pointer rounded-xl border border-white/15 bg-black/50 text-zinc-200 hover:bg-black/75 hover:text-white"
							>
								<X className="size-4" />
							</Button>
							<div className="flex items-center gap-2 rounded-full border border-white/15 bg-black/50 px-3 py-1 text-xs text-zinc-200">
								<Camera className="size-3.5 text-emerald-400" />
								<span>{shot.resolution}</span>
							</div>
						</div>
						<div className="flex items-center justify-between p-4">
							<div>
								<DialogTitle className="text-sm font-semibold text-foreground">
									{shotTitle}
								</DialogTitle>
								<DialogDescription className="mt-0.5 text-xs text-muted-foreground">
									{shot.instance} · {shot.shaderPack} · {t(shot.capturedAtKey)}
								</DialogDescription>
							</div>
						</div>
					</>
				) : null}
			</DialogContent>
		</Dialog>
	)
}

export default LightboxModal
