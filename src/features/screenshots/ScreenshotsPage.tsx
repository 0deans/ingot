import { useCallback, useMemo, useState } from "react"
import LightboxModal from "@/features/screenshots/components/LightboxModal"
import ScreenshotGrid, {
	type ScreenshotItem,
} from "@/features/screenshots/components/ScreenshotGrid"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"

type InstanceFilter = "all" | ScreenshotItem["instance"]

const INSTANCE_FILTERS: SegmentedOption<InstanceFilter>[] = [
	{ value: "all", label: "All Shots" },
	{ value: "Fabulously Optimized", label: "Fabulously" },
	{ value: "Horizon Engine Nightly", label: "Horizon" },
	{ value: "Create: Astral", label: "Create" },
]

const SCREENSHOTS: ScreenshotItem[] = [
	{
		id: "shot-1",
		title: "Twilight Spire Over Aldervale",
		instance: "Fabulously Optimized",
		capturedAt: "Today · 19:42",
		resolution: "2560 × 1440",
		shaderPack: "Complementary Unbound",
		gradient: "from-emerald-600/40 via-sky-700/25 to-zinc-900",
	},
	{
		id: "shot-2",
		title: "Horizon Vulkan Volumetric Fog",
		instance: "Horizon Engine Nightly",
		capturedAt: "Yesterday · 22:15",
		resolution: "3840 × 2160",
		shaderPack: "Horizon Native PBR",
		gradient: "from-cyan-500/40 via-indigo-700/25 to-zinc-900",
	},
	{
		id: "shot-3",
		title: "Automatic Brass Train Junction",
		instance: "Create: Astral",
		capturedAt: "3 days ago",
		resolution: "2560 × 1440",
		shaderPack: "BSL v8.2",
		gradient: "from-amber-500/40 via-rose-700/25 to-zinc-900",
	},
	{
		id: "shot-4",
		title: "Deep Slate Cavern Geode",
		instance: "Fabulously Optimized",
		capturedAt: "Last week",
		resolution: "1920 × 1080",
		shaderPack: "Iris Fast",
		gradient: "from-violet-600/40 via-fuchsia-700/25 to-zinc-900",
	},
]

const ScreenshotsPage = () => {
	const [filter, setFilter] = useState<InstanceFilter>("all")
	const [copiedId, setCopiedId] = useState<string | null>(null)
	const [activeShot, setActiveShot] = useState<ScreenshotItem | null>(null)

	const visibleShots = useMemo(() => {
		if (filter === "all") {
			return SCREENSHOTS
		}
		return SCREENSHOTS.filter((shot) => {
			return shot.instance === filter
		})
	}, [filter])

	const handleCopyMeta = useCallback((shot: ScreenshotItem) => {
		setCopiedId(shot.id)
		if (typeof navigator !== "undefined" && navigator.clipboard) {
			void navigator.clipboard.writeText(`${shot.title} (${shot.instance})`).catch(() => {
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
	}, [])

	const handleCloseLightbox = useCallback(() => {
		setActiveShot(null)
	}, [])

	return (
		<div className="flex flex-col gap-4">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">Screenshots</h1>
					<p className="text-[12px] text-muted-foreground">
						Unified gallery indexed across all instances and shader profiles.
					</p>
				</div>
				<SegmentedControl
					layoutId="screenshots-instance-filter"
					options={INSTANCE_FILTERS}
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
