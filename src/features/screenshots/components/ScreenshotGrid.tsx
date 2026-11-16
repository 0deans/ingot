import ScreenshotCard, {
	type ScreenshotItem,
} from "@/features/screenshots/components/ScreenshotCard"

export type { ScreenshotItem }

interface ScreenshotGridProps {
	shots: readonly ScreenshotItem[]
	copiedId: string | null
	onOpenLightbox: (shot: ScreenshotItem) => void
	onCopyMeta: (shot: ScreenshotItem) => void
}

const ScreenshotGrid = ({ shots, copiedId, onOpenLightbox, onCopyMeta }: ScreenshotGridProps) => {
	return (
		<div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
			{shots.map((shot) => {
				return (
					<ScreenshotCard
						key={shot.id}
						shot={shot}
						isCopied={copiedId === shot.id}
						onOpenLightbox={onOpenLightbox}
						onCopyMeta={onCopyMeta}
					/>
				)
			})}
		</div>
	)
}

export default ScreenshotGrid
