import { useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import LightboxModal from "@/features/screenshots/components/LightboxModal"
import ScreenshotGrid, {
	type ScreenshotItem,
} from "@/features/screenshots/components/ScreenshotGrid"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"

type InstanceFilter = "all" | ScreenshotItem["instance"]

const SCREENSHOTS: ScreenshotItem[] = [
	{
		id: "shot-1",
		titleKey: "screenshots.items.shot-1.title",
		instance: "Fabulously Optimized",
		capturedAtKey: "screenshots.items.shot-1.capturedAt",
		resolution: "2560 × 1440",
		shaderPack: "Complementary Unbound",
		gradient: "from-emerald-600/40 via-sky-700/25 to-zinc-900",
	},
	{
		id: "shot-2",
		titleKey: "screenshots.items.shot-2.title",
		instance: "Horizon Engine Nightly",
		capturedAtKey: "screenshots.items.shot-2.capturedAt",
		resolution: "3840 × 2160",
		shaderPack: "Horizon Native PBR",
		gradient: "from-cyan-500/40 via-indigo-700/25 to-zinc-900",
	},
	{
		id: "shot-3",
		titleKey: "screenshots.items.shot-3.title",
		instance: "Create: Astral",
		capturedAtKey: "screenshots.items.shot-3.capturedAt",
		resolution: "2560 × 1440",
		shaderPack: "BSL v8.2",
		gradient: "from-amber-500/40 via-rose-700/25 to-zinc-900",
	},
	{
		id: "shot-4",
		titleKey: "screenshots.items.shot-4.title",
		instance: "Fabulously Optimized",
		capturedAtKey: "screenshots.items.shot-4.capturedAt",
		resolution: "1920 × 1080",
		shaderPack: "Iris Fast",
		gradient: "from-violet-600/40 via-fuchsia-700/25 to-zinc-900",
	},
]

const ScreenshotsPage = () => {
	const { t } = useTranslation()

	const [filter, setFilter] = useState<InstanceFilter>("all")
	const [copiedId, setCopiedId] = useState<string | null>(null)
	const [activeShot, setActiveShot] = useState<ScreenshotItem | null>(null)

	const instanceFilters: SegmentedOption<InstanceFilter>[] = useMemo(() => {
		return [
			{ value: "all", label: t("screenshots.filters.all") },
			{ value: "Fabulously Optimized", label: t("screenshots.filters.fabulously") },
			{ value: "Horizon Engine Nightly", label: t("screenshots.filters.horizon") },
			{ value: "Create: Astral", label: t("screenshots.filters.create") },
		]
	}, [t])

	const visibleShots = useMemo(() => {
		if (filter === "all") {
			return SCREENSHOTS
		}
		return SCREENSHOTS.filter((shot) => {
			return shot.instance === filter
		})
	}, [filter])

	const handleCopyMeta = useCallback(
		(shot: ScreenshotItem) => {
			setCopiedId(shot.id)
			if (typeof navigator !== "undefined" && navigator.clipboard) {
				const shotTitle = t(shot.titleKey)
				void navigator.clipboard.writeText(`${shotTitle} (${shot.instance})`).catch(() => {
					return undefined
				})
			}
			window.setTimeout(() => {
				setCopiedId((current) => {
					if (current === shot.id) {
						return null
					}

					return current
				})
			}, 1500)
		},
		[t],
	)

	const handleCloseLightbox = useCallback(() => {
		setActiveShot(null)
	}, [])

	return (
		<div className="flex flex-col gap-4">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">
						{t("screenshots.title")}
					</h1>
					<p className="text-[12px] text-muted-foreground">{t("screenshots.subtitle")}</p>
				</div>
				<SegmentedControl
					layoutId="screenshots-instance-filter"
					options={instanceFilters}
					value={filter}
					onChange={setFilter}
					tone="emerald"
				/>
			</div>

			<ScreenshotGrid
				shots={visibleShots}
				copiedId={copiedId}
				onOpenLightbox={setActiveShot}
				onCopyMeta={handleCopyMeta}
			/>

			<LightboxModal shot={activeShot} onClose={handleCloseLightbox} />
		</div>
	)
}

export default ScreenshotsPage
