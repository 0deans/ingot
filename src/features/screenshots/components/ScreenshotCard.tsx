import { Camera, Check, Copy, Maximize2 } from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"

export type ScreenshotId = "shot-1" | "shot-2" | "shot-3" | "shot-4"

export interface ScreenshotItem {
	id: ScreenshotId
	titleKey: `screenshots.items.${ScreenshotId}.title`
	instance: "Fabulously Optimized" | "Horizon Engine Nightly" | "Create: Astral"
	capturedAtKey: `screenshots.items.${ScreenshotId}.capturedAt`
	resolution: string
	shaderPack: string
	gradient: string
}

interface ScreenshotCardProps {
	shot: ScreenshotItem
	isCopied: boolean
	onOpenLightbox: (shot: ScreenshotItem) => void
	onCopyMeta: (shot: ScreenshotItem) => void
}

const ScreenshotCard = ({ shot, isCopied, onOpenLightbox, onCopyMeta }: ScreenshotCardProps) => {
	const { t } = useTranslation()

	const shotTitle = t(shot.titleKey)
	const CopyIcon = isCopied ? Check : Copy
	const copyLabel = isCopied ? t("common.copied") : t("screenshots.copyName")

	return (
		<article className="overflow-hidden rounded-2xl border border-border bg-card/65 transition-colors hover:border-foreground/20">
			<div
				className={cn(
					"relative flex h-40 items-end justify-between bg-linear-to-br p-4",
					shot.gradient,
				)}
			>
				<div className="flex items-center gap-2 rounded-full border border-white/15 bg-black/45 px-2.5 py-1 text-[10.5px] font-medium text-zinc-200 backdrop-blur-md">
					<Camera className="size-3 text-emerald-400" />
					<span>{shot.resolution}</span>
				</div>
				<button
					type="button"
					onClick={() => {
						onOpenLightbox(shot)
					}}
					aria-label={t("screenshots.openLightboxAria", { title: shotTitle })}
					className="flex size-8 cursor-pointer items-center justify-center rounded-xl border border-white/15 bg-black/45 text-zinc-200 backdrop-blur-md transition-colors hover:bg-black/70"
				>
					<Maximize2 className="size-3.5" />
				</button>
			</div>

			<div className="flex items-center justify-between gap-3 p-3.5">
				<div className="min-w-0">
					<h2 className="truncate text-[13.5px] font-semibold text-foreground">{shotTitle}</h2>
					<p className="truncate text-[11px] text-muted-foreground">
						{shot.instance} · {shot.shaderPack} · {t(shot.capturedAtKey)}
					</p>
				</div>

				<button
					type="button"
					onClick={() => {
						onCopyMeta(shot)
					}}
					className="flex h-7 shrink-0 cursor-pointer items-center gap-1 rounded-lg border border-border bg-foreground/6 px-2.5 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/12"
				>
					<CopyIcon className={cn("size-3", isCopied && "text-primary")} />
					<span className={cn(isCopied && "text-primary")}>{copyLabel}</span>
				</button>
			</div>
		</article>
	)
}

export default ScreenshotCard
