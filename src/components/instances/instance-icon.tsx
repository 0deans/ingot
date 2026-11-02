import { cn } from "cn"
import type React from "react"
import type { ModLoaderType, ServerCoreType } from "@/bindings"
import LoaderIcon from "@/components/instances/loader-icon"

export type PresetColorTagId =
	| "red"
	| "orange"
	| "amber"
	| "emerald"
	| "cyan"
	| "blue"
	| "violet"
	| "pink"
	| "rose"

export type InstanceColorTag = PresetColorTagId | string

export type PresetIconId =
	| "creeper"
	| "grass"
	| "sword"
	| "pickaxe"
	| "redstone"
	| "ender"
	| "nether"
	| "potion"
	| "command_block"
	| "chest"
	| "beacon"
	| "book"

export interface PresetIconDefinition {
	id: PresetIconId
	spec: `preset:${PresetIconId}`
	labelKey: string
	defaultLabel: string
	bgTop: string
	bgBottom: string
	border: string
	pixels: string[]
	palette: Record<string, string>
}

export const PRESET_ICONS: readonly PresetIconDefinition[] = [
	{
		id: "creeper",
		spec: "preset:creeper",
		labelKey: "instanceIcon.presets.creeper",
		defaultLabel: "Creeper",
		bgTop: "#48b848",
		bgBottom: "#246e24",
		border: "#5ce05c",
		pixels: [
			"........",
			".##..##.",
			".##..##.",
			"...##...",
			"..####..",
			"..####..",
			"..#..#..",
			"........",
		],
		palette: { "#": "#101810" },
	},
	{
		id: "grass",
		spec: "preset:grass",
		labelKey: "instanceIcon.presets.grass",
		defaultLabel: "Grass Block",
		bgTop: "#54be42",
		bgBottom: "#785232",
		border: "#6ed658",
		pixels: [
			"GGGGGGGG",
			"GGGGGGGG",
			"GggGGgGG",
			"ggDggDgg",
			"DDDDDDDD",
			"DDdDDDdd",
			"DDDDdDDD",
			"dDDDDDDd",
		],
		palette: {
			G: "#5cc846",
			g: "#449c32",
			D: "#865c38",
			d: "#6a4628",
		},
	},
	{
		id: "sword",
		spec: "preset:sword",
		labelKey: "instanceIcon.presets.sword",
		defaultLabel: "Diamond Sword",
		bgTop: "#1c283e",
		bgBottom: "#0e1422",
		border: "#3ef0e0",
		pixels: [
			"......DD",
			".....DDD",
			"....DDD.",
			".g.DDD..",
			"..gGD...",
			".s.g....",
			"G.......",
			"........",
		],
		palette: {
			D: "#4cf8e8",
			G: "#289e98",
			g: "#1e6e6e",
			s: "#865c34",
		},
	},
	{
		id: "pickaxe",
		spec: "preset:pickaxe",
		labelKey: "instanceIcon.presets.pickaxe",
		defaultLabel: "Diamond Pickaxe",
		bgTop: "#222434",
		bgBottom: "#12131c",
		border: "#3ce4f0",
		pixels: [
			"..DDDDD.",
			".DD..sDD",
			".D..s..D",
			"...s...D",
			"..s.....",
			".s......",
			"s.......",
			"........",
		],
		palette: {
			D: "#4cf8e8",
			s: "#8c6238",
		},
	},
	{
		id: "redstone",
		spec: "preset:redstone",
		labelKey: "instanceIcon.presets.redstone",
		defaultLabel: "Redstone",
		bgTop: "#361216",
		bgBottom: "#1c080a",
		border: "#f83434",
		pixels: [
			"...rr...",
			"..rRRr..",
			".rRRRRr.",
			"rRROORRr",
			"rRROORRr",
			".rRRRRr.",
			"..rRRr..",
			"...rr...",
		],
		palette: {
			R: "#f22222",
			r: "#981010",
			O: "#ff7858",
		},
	},
	{
		id: "ender",
		spec: "preset:ender",
		labelKey: "instanceIcon.presets.ender",
		defaultLabel: "Ender Pearl",
		bgTop: "#102a2a",
		bgBottom: "#081416",
		border: "#28d2b4",
		pixels: [
			"..eeee..",
			".eEEEEe.",
			"eEEccEEe",
			"eEcCCcEe",
			"eEcCCcEe",
			"eEEccEEe",
			".eEEEEe.",
			"..eeee..",
		],
		palette: {
			e: "#105c52",
			E: "#22a892",
			c: "#5ce8cc",
			C: "#bcfff0",
		},
	},
	{
		id: "nether",
		spec: "preset:nether",
		labelKey: "instanceIcon.presets.nether",
		defaultLabel: "Nether Portal",
		bgTop: "#2e1042",
		bgBottom: "#160622",
		border: "#b448f8",
		pixels: [
			"OOOOOOOO",
			"OppppppO",
			"OpPPPPpO",
			"OpPWWPpO",
			"OpPWWPpO",
			"OpPPPPpO",
			"OppppppO",
			"OOOOOOOO",
		],
		palette: {
			O: "#241c30",
			p: "#8226cc",
			P: "#bc56f8",
			W: "#e8b4ff",
		},
	},
	{
		id: "potion",
		spec: "preset:potion",
		labelKey: "instanceIcon.presets.potion",
		defaultLabel: "Potion",
		bgTop: "#261638",
		bgBottom: "#120a1e",
		border: "#ec4899",
		pixels: [
			"...cc...",
			"..gggg..",
			"...gg...",
			"..gMMg..",
			".gMMMMg.",
			".gMpMMg.",
			".gMMMMg.",
			"..gggg..",
		],
		palette: {
			c: "#b48248",
			g: "#c8dce8",
			M: "#ec3c92",
			p: "#ff9cd2",
		},
	},
	{
		id: "command_block",
		spec: "preset:command_block",
		labelKey: "instanceIcon.presets.commandBlock",
		defaultLabel: "Command Block",
		bgTop: "#422a1e",
		bgBottom: "#241610",
		border: "#de8854",
		pixels: [
			"cccccccc",
			"cCCCCCCc",
			"cCbbbbCc",
			"cCbggbCc",
			"cCbggbCc",
			"cCbbbbCc",
			"cCCCCCCc",
			"cccccccc",
		],
		palette: {
			c: "#8c5438",
			C: "#c87c52",
			b: "#483c38",
			g: "#86e2a8",
		},
	},
	{
		id: "chest",
		spec: "preset:chest",
		labelKey: "instanceIcon.presets.chest",
		defaultLabel: "Chest",
		bgTop: "#3c2814",
		bgBottom: "#20140a",
		border: "#d89238",
		pixels: [
			"bbbbbbbb",
			"bWWWWWWb",
			"bWWWWWWb",
			"bbbbLbbb",
			"bWWWLLWb",
			"bWWWWWWb",
			"bWWWWWWb",
			"bbbbbbbb",
		],
		palette: {
			b: "#32200e",
			W: "#b8782c",
			L: "#f0d458",
		},
	},
	{
		id: "beacon",
		spec: "preset:beacon",
		labelKey: "instanceIcon.presets.beacon",
		defaultLabel: "Beacon",
		bgTop: "#14343c",
		bgBottom: "#0a1a20",
		border: "#5cf0ff",
		pixels: [
			"gggggggg",
			"gbbbbbbg",
			"gbBBBBbg",
			"gbBWWBbg",
			"gbBWWBbg",
			"gbBBBBbg",
			"OOOOOOOO",
			"OOOOOOOO",
		],
		palette: {
			g: "#b4e8f4",
			b: "#3cd4e8",
			B: "#78f8ff",
			W: "#ebffff",
			O: "#282234",
		},
	},
	{
		id: "book",
		spec: "preset:book",
		labelKey: "instanceIcon.presets.book",
		defaultLabel: "Enchanted Book",
		bgTop: "#2c1838",
		bgBottom: "#160a1e",
		border: "#c864f8",
		pixels: [
			".rrrrrr.",
			"rRRRRRRp",
			"rRggggRp",
			"rRRRRRRp",
			"rRggggRp",
			"rRRRRRRp",
			"rppppppp",
			".wwwwww.",
		],
		palette: {
			r: "#681c2c",
			R: "#a82842",
			g: "#f4d254",
			p: "#c858f0",
			w: "#ece4d4",
		},
	},
]

export interface ColorTagDefinition {
	id: string
	labelKey: string
	defaultLabel: string
	dotClass: string
	glowClass: string
	borderClass: string
	badgeClass: string
	ringClass: string
	hex: string
	isCustom?: boolean
}

export const COLOR_TAGS: readonly ColorTagDefinition[] = [
	{
		id: "red",
		labelKey: "instances.colors.red",
		defaultLabel: "Red",
		dotClass: "bg-red-500",
		glowClass: "from-red-500/12 via-red-500/4 to-transparent",
		borderClass: "border-red-500/35 hover:border-red-500/60",
		badgeClass: "border-red-500/30 bg-red-500/10 text-red-400",
		ringClass: "ring-red-500",
		hex: "#ef4444",
	},
	{
		id: "orange",
		labelKey: "instances.colors.orange",
		defaultLabel: "Orange",
		dotClass: "bg-orange-500",
		glowClass: "from-orange-500/12 via-orange-500/4 to-transparent",
		borderClass: "border-orange-500/35 hover:border-orange-500/60",
		badgeClass: "border-orange-500/30 bg-orange-500/10 text-orange-400",
		ringClass: "ring-orange-500",
		hex: "#f97316",
	},
	{
		id: "amber",
		labelKey: "instances.colors.amber",
		defaultLabel: "Amber",
		dotClass: "bg-amber-500",
		glowClass: "from-amber-500/12 via-amber-500/4 to-transparent",
		borderClass: "border-amber-500/35 hover:border-amber-500/60",
		badgeClass: "border-amber-500/30 bg-amber-500/10 text-amber-400",
		ringClass: "ring-amber-500",
		hex: "#f59e0b",
	},
	{
		id: "emerald",
		labelKey: "instances.colors.emerald",
		defaultLabel: "Emerald",
		dotClass: "bg-emerald-500",
		glowClass: "from-emerald-500/12 via-emerald-500/4 to-transparent",
		borderClass: "border-emerald-500/35 hover:border-emerald-500/60",
		badgeClass: "border-emerald-500/30 bg-emerald-500/10 text-emerald-400",
		ringClass: "ring-emerald-500",
		hex: "#10b981",
	},
	{
		id: "cyan",
		labelKey: "instances.colors.cyan",
		defaultLabel: "Cyan",
		dotClass: "bg-cyan-500",
		glowClass: "from-cyan-500/12 via-cyan-500/4 to-transparent",
		borderClass: "border-cyan-500/35 hover:border-cyan-500/60",
		badgeClass: "border-cyan-500/30 bg-cyan-500/10 text-cyan-400",
		ringClass: "ring-cyan-500",
		hex: "#06b6d4",
	},
	{
		id: "blue",
		labelKey: "instances.colors.blue",
		defaultLabel: "Blue",
		dotClass: "bg-blue-500",
		glowClass: "from-blue-500/12 via-blue-500/4 to-transparent",
		borderClass: "border-blue-500/35 hover:border-blue-500/60",
		badgeClass: "border-blue-500/30 bg-blue-500/10 text-blue-400",
		ringClass: "ring-blue-500",
		hex: "#3b82f6",
	},
	{
		id: "violet",
		labelKey: "instances.colors.violet",
		defaultLabel: "Violet",
		dotClass: "bg-violet-500",
		glowClass: "from-violet-500/12 via-violet-500/4 to-transparent",
		borderClass: "border-violet-500/35 hover:border-violet-500/60",
		badgeClass: "border-violet-500/30 bg-violet-500/10 text-violet-400",
		ringClass: "ring-violet-500",
		hex: "#8b5cf6",
	},
	{
		id: "pink",
		labelKey: "instances.colors.pink",
		defaultLabel: "Pink",
		dotClass: "bg-pink-500",
		glowClass: "from-pink-500/12 via-pink-500/4 to-transparent",
		borderClass: "border-pink-500/35 hover:border-pink-500/60",
		badgeClass: "border-pink-500/30 bg-pink-500/10 text-pink-400",
		ringClass: "ring-pink-500",
		hex: "#ec4899",
	},
	{
		id: "rose",
		labelKey: "instances.colors.rose",
		defaultLabel: "Rose",
		dotClass: "bg-rose-500",
		glowClass: "from-rose-500/12 via-rose-500/4 to-transparent",
		borderClass: "border-rose-500/35 hover:border-rose-500/60",
		badgeClass: "border-rose-500/30 bg-rose-500/10 text-rose-400",
		ringClass: "ring-rose-500",
		hex: "#f43f5e",
	},
]

export const isValidHexColor = (value: string | null | undefined): boolean => {
	if (!value) {
		return false
	}
	return /^#[0-9a-fA-F]{6}$/.test(value.trim())
}

export const getColorTagDefinition = (
	colorTag: string | null | undefined,
): ColorTagDefinition | null => {
	if (!colorTag) {
		return null
	}
	const trimmed = colorTag.trim().toLowerCase()
	const preset = COLOR_TAGS.find((entry) => {
		return entry.id === trimmed || entry.hex.toLowerCase() === trimmed
	})
	if (preset) {
		return preset
	}
	if (isValidHexColor(trimmed)) {
		return {
			id: trimmed,
			labelKey: "instances.colors.custom",
			defaultLabel: trimmed.toUpperCase(),
			dotClass: "",
			glowClass: "",
			borderClass: "",
			badgeClass: "",
			ringClass: "",
			hex: trimmed,
			isCustom: true,
		}
	}
	return null
}

export const findPresetIcon = (
	iconSpec: string | null | undefined,
): PresetIconDefinition | null => {
	if (!iconSpec) {
		return null
	}
	const normalized = iconSpec.trim().toLowerCase()
	const id = normalized.startsWith("preset:") ? normalized.slice("preset:".length) : normalized
	return (
		PRESET_ICONS.find((preset) => {
			return preset.id === id
		}) ?? null
	)
}

interface PresetPixelSvgProps {
	preset: PresetIconDefinition
	size: number | string
	className?: string
}

const PresetPixelSvg = ({ preset, size, className }: PresetPixelSvgProps) => {
	const gradientId = `preset-grad-${preset.id}`
	const cells: React.ReactElement[] = []

	for (let rowIdx = 0; rowIdx < preset.pixels.length; rowIdx += 1) {
		const row = preset.pixels[rowIdx]
		for (let colIdx = 0; colIdx < row.length; colIdx += 1) {
			const symbol = row[colIdx]
			if (symbol === ".") {
				continue
			}
			const fill = preset.palette[symbol]
			if (!fill) {
				continue
			}
			cells.push(
				<rect
					key={`${rowIdx}-${colIdx}`}
					x={20 + colIdx * 11}
					y={20 + rowIdx * 11}
					width={11.2}
					height={11.2}
					fill={fill}
				/>,
			)
		}
	}

	return (
		<svg
			viewBox="0 0 128 128"
			className={cn("shrink-0 select-none", className)}
			style={{
				width: typeof size === "number" ? `${size}px` : size,
				height: typeof size === "number" ? `${size}px` : size,
			}}
			aria-label={preset.defaultLabel}
			role="img"
		>
			<defs>
				<linearGradient id={gradientId} x1="0%" y1="0%" x2="0%" y2="100%">
					<stop offset="0%" stopColor={preset.bgTop} />
					<stop offset="100%" stopColor={preset.bgBottom} />
				</linearGradient>
			</defs>
			<rect
				x="4"
				y="4"
				width="120"
				height="120"
				rx="26"
				fill={`url(#${gradientId})`}
				stroke={preset.border}
				strokeWidth="5"
			/>
			{cells}
		</svg>
	)
}

export interface InstanceIconProps {
	icon?: string | null
	loader: ModLoaderType | ServerCoreType | string
	size?: number | string
	className?: string
}

export const InstanceIcon = ({ icon, loader, size = 22, className }: InstanceIconProps) => {
	const trimmed = icon?.trim() ?? ""

	if (trimmed.startsWith("preset:")) {
		const preset = findPresetIcon(trimmed)
		if (preset) {
			return <PresetPixelSvg preset={preset} size={size} className={className} />
		}
	}

	if (trimmed.startsWith("loader:")) {
		const customLoader = trimmed.slice("loader:".length).trim()
		if (customLoader) {
			return <LoaderIcon loader={customLoader} size={size} className={className} />
		}
	}

	if (trimmed.startsWith("data:image/")) {
		return (
			<img
				src={trimmed}
				alt=""
				className={cn("shrink-0 rounded-md object-cover select-none", className)}
				style={{
					width: typeof size === "number" ? `${size}px` : size,
					height: typeof size === "number" ? `${size}px` : size,
				}}
				draggable={false}
			/>
		)
	}

	return <LoaderIcon loader={loader} size={size} className={className} />
}

const readFileAsDataUrl = (file: Blob): Promise<string> => {
	return new Promise((resolve, reject) => {
		const reader = new FileReader()
		reader.onload = () => {
			if (typeof reader.result === "string") {
				resolve(reader.result)
			} else {
				reject(new Error("Failed to read file as data URL"))
			}
		}
		reader.onerror = () => {
			reject(reader.error ?? new Error("Failed to read file"))
		}
		reader.readAsDataURL(file)
	})
}

const renderImageElementToSquarePng = (dataUrl: string, targetSize = 256): Promise<string> => {
	return new Promise((resolve, reject) => {
		const img = new Image()
		img.onload = () => {
			const canvas = document.createElement("canvas")
			canvas.width = targetSize
			canvas.height = targetSize
			const ctx = canvas.getContext("2d")
			if (!ctx) {
				reject(new Error("Failed to create 2D canvas context"))
				return
			}
			const minSide = Math.min(img.naturalWidth || targetSize, img.naturalHeight || targetSize)
			const sx = Math.max(0, Math.floor(((img.naturalWidth || targetSize) - minSide) / 2))
			const sy = Math.max(0, Math.floor(((img.naturalHeight || targetSize) - minSide) / 2))
			ctx.clearRect(0, 0, targetSize, targetSize)
			ctx.drawImage(img, sx, sy, minSide, minSide, 0, 0, targetSize, targetSize)
			resolve(canvas.toDataURL("image/png"))
		}
		img.onerror = () => {
			reject(new Error("Invalid image file"))
		}
		img.src = dataUrl
	})
}

/**
 * Converts an uploaded image or animated GIF into a self-contained data URL.
 * Animated GIFs preserve their animation in `<img src=...>` while embedding a 256x256 PNG
 * frame in the RFC 2397 `;frame=<b64>` parameter so OS `.ico` and Steam Grid generators
 * can extract a high-resolution static frame without external decoders.
 */
export const processUploadedIconFile = async (file: File): Promise<string> => {
	const rawDataUrl = await readFileAsDataUrl(file)
	const squarePngDataUrl = await renderImageElementToSquarePng(rawDataUrl, 256)

	const isGif =
		file.type.toLowerCase() === "image/gif" ||
		rawDataUrl.toLowerCase().startsWith("data:image/gif;")

	if (isGif && file.size <= 2 * 1024 * 1024) {
		const gifCommaIdx = rawDataUrl.indexOf(",")
		const pngCommaIdx = squarePngDataUrl.indexOf(",")
		if (gifCommaIdx !== -1 && pngCommaIdx !== -1) {
			const gifBase64 = rawDataUrl.slice(gifCommaIdx + 1)
			const pngBase64 = squarePngDataUrl.slice(pngCommaIdx + 1)
			return `data:image/gif;frame=${pngBase64};base64,${gifBase64}`
		}
	}

	return squarePngDataUrl
}

export default InstanceIcon
