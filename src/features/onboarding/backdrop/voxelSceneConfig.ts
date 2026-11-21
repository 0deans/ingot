export interface IsometricCubeSpec {
	id: string
	positionClass: string
	size: number
	driftY: number
	duration: number
	delay: number
	topFill: string
	leftFill: string
	rightFill: string
	strokeColor: string
	visibilityClass: string
}

export interface PixelEmberSpec {
	id: string
	leftPercent: string
	bottomPercent: string
	sizeClass: string
	colorClass: string
	riseY: number
	duration: number
	delay: number
}

export interface PixelStarSpec {
	id: string
	leftPercent: string
	topPercent: string
	sizeClass: string
	duration: number
	delay: number
}

export interface VoxelCloudSpec {
	id: string
	topClass: string
	startX: string
	driftX: number
	widthClass: string
	duration: number
	delay: number
	opacityClass: string
}

export const ISOMETRIC_CUBES: readonly IsometricCubeSpec[] = [
	{
		id: "cube-mid-left",
		positionClass: "top-[44%] left-[13%]",
		size: 32,
		driftY: 11,
		duration: 8.4,
		delay: 0.4,
		topFill: "color-mix(in oklab, var(--color-primary) 26%, transparent)",
		leftFill: "color-mix(in oklab, var(--color-primary) 15%, transparent)",
		rightFill: "color-mix(in oklab, var(--color-primary) 9%, transparent)",
		strokeColor: "color-mix(in oklab, var(--color-primary) 40%, transparent)",
		visibilityClass: "hidden md:block",
	},
	{
		id: "cube-mid-right",
		positionClass: "top-[46%] right-[13%]",
		size: 36,
		driftY: 12,
		duration: 9.2,
		delay: 0.2,
		topFill: "rgba(251, 191, 36, 0.22)",
		leftFill: "rgba(245, 158, 11, 0.13)",
		rightFill: "rgba(217, 119, 6, 0.09)",
		strokeColor: "rgba(251, 191, 36, 0.34)",
		visibilityClass: "hidden md:block",
	},
	{
		id: "cube-low-left",
		positionClass: "bottom-[14%] left-[24%]",
		size: 26,
		driftY: 8,
		duration: 7.8,
		delay: 1.1,
		topFill: "rgba(251, 191, 36, 0.20)",
		leftFill: "rgba(245, 158, 11, 0.12)",
		rightFill: "rgba(217, 119, 6, 0.08)",
		strokeColor: "rgba(251, 191, 36, 0.32)",
		visibilityClass: "hidden lg:block",
	},
	{
		id: "cube-low-right",
		positionClass: "bottom-[15%] right-[24%]",
		size: 28,
		driftY: 9,
		duration: 8.6,
		delay: 0.9,
		topFill: "color-mix(in oklab, var(--color-primary) 24%, transparent)",
		leftFill: "color-mix(in oklab, var(--color-primary) 14%, transparent)",
		rightFill: "color-mix(in oklab, var(--color-primary) 8%, transparent)",
		strokeColor: "color-mix(in oklab, var(--color-primary) 38%, transparent)",
		visibilityClass: "hidden lg:block",
	},
]

export const PIXEL_EMBERS: readonly PixelEmberSpec[] = [
	{
		id: "ember-1",
		leftPercent: "12%",
		bottomPercent: "10%",
		sizeClass: "size-1.5",
		colorClass: "bg-amber-400/75",
		riseY: 90,
		duration: 5.4,
		delay: 0.2,
	},
	{
		id: "ember-2",
		leftPercent: "24%",
		bottomPercent: "8%",
		sizeClass: "size-1",
		colorClass: "bg-primary/70",
		riseY: 110,
		duration: 6.2,
		delay: 1.4,
	},
	{
		id: "ember-3",
		leftPercent: "37%",
		bottomPercent: "12%",
		sizeClass: "size-1.5",
		colorClass: "bg-orange-400/65",
		riseY: 85,
		duration: 5.8,
		delay: 0.8,
	},
	{
		id: "ember-4",
		leftPercent: "53%",
		bottomPercent: "9%",
		sizeClass: "size-1",
		colorClass: "bg-amber-300/75",
		riseY: 100,
		duration: 6.0,
		delay: 2.1,
	},
	{
		id: "ember-5",
		leftPercent: "68%",
		bottomPercent: "11%",
		sizeClass: "size-1.5",
		colorClass: "bg-primary/70",
		riseY: 95,
		duration: 5.5,
		delay: 0.5,
	},
	{
		id: "ember-6",
		leftPercent: "82%",
		bottomPercent: "13%",
		sizeClass: "size-1",
		colorClass: "bg-amber-400/70",
		riseY: 105,
		duration: 6.4,
		delay: 1.7,
	},
	{
		id: "ember-7",
		leftPercent: "91%",
		bottomPercent: "9%",
		sizeClass: "size-1.5",
		colorClass: "bg-orange-400/60",
		riseY: 88,
		duration: 5.9,
		delay: 1.0,
	},
]

export const PIXEL_STARS: readonly PixelStarSpec[] = [
	{
		id: "star-1",
		leftPercent: "9%",
		topPercent: "9%",
		sizeClass: "size-1",
		duration: 3.8,
		delay: 0.2,
	},
	{
		id: "star-2",
		leftPercent: "16%",
		topPercent: "20%",
		sizeClass: "size-0.5",
		duration: 4.4,
		delay: 1.1,
	},
	{
		id: "star-3",
		leftPercent: "27%",
		topPercent: "12%",
		sizeClass: "size-1",
		duration: 3.4,
		delay: 0.6,
	},
	{
		id: "star-4",
		leftPercent: "36%",
		topPercent: "7%",
		sizeClass: "size-0.5",
		duration: 4.8,
		delay: 1.8,
	},
	{
		id: "star-5",
		leftPercent: "44%",
		topPercent: "15%",
		sizeClass: "size-1",
		duration: 4.1,
		delay: 0.9,
	},
	{
		id: "star-6",
		leftPercent: "57%",
		topPercent: "10%",
		sizeClass: "size-0.5",
		duration: 3.6,
		delay: 1.5,
	},
	{
		id: "star-7",
		leftPercent: "65%",
		topPercent: "18%",
		sizeClass: "size-1",
		duration: 4.5,
		delay: 0.4,
	},
	{
		id: "star-8",
		leftPercent: "74%",
		topPercent: "8%",
		sizeClass: "size-1",
		duration: 3.9,
		delay: 1.3,
	},
	{
		id: "star-9",
		leftPercent: "84%",
		topPercent: "22%",
		sizeClass: "size-0.5",
		duration: 4.2,
		delay: 0.7,
	},
	{
		id: "star-10",
		leftPercent: "92%",
		topPercent: "11%",
		sizeClass: "size-1",
		duration: 3.7,
		delay: 2.0,
	},
]

export const VOXEL_CLOUDS: readonly VoxelCloudSpec[] = [
	{
		id: "cloud-left",
		topClass: "top-[16%]",
		startX: "15%",
		driftX: 28,
		widthClass: "w-32 sm:w-44",
		duration: 18,
		delay: 0,
		opacityClass: "opacity-[0.09]",
	},
	{
		id: "cloud-center-right",
		topClass: "top-[17%]",
		startX: "61%",
		driftX: -24,
		widthClass: "w-28 sm:w-40",
		duration: 21,
		delay: 1.2,
		opacityClass: "opacity-[0.08]",
	},
	{
		id: "cloud-right",
		topClass: "top-[27%]",
		startX: "76%",
		driftX: 22,
		widthClass: "w-24 sm:w-36",
		duration: 19,
		delay: 0.6,
		opacityClass: "opacity-[0.07]",
	},
]
