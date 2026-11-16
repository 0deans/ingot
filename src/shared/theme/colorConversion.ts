export interface RgbColor {
	r: number
	g: number
	b: number
}

export interface HsvColor {
	h: number
	s: number
	v: number
}

const clampByte = (value: number): number => {
	return Math.min(255, Math.max(0, Math.round(value)))
}

const clampUnit = (value: number): number => {
	return Math.min(1, Math.max(0, value))
}

const toHexByte = (value: number): string => {
	return clampByte(value).toString(16).padStart(2, "0")
}

export const rgbToHex = ({ r, g, b }: RgbColor): string => {
	return `#${toHexByte(r)}${toHexByte(g)}${toHexByte(b)}`
}

export const rgbToHsv = ({ r, g, b }: RgbColor): HsvColor => {
	const rn = r / 255
	const gn = g / 255
	const bn = b / 255

	const max = Math.max(rn, gn, bn)
	const min = Math.min(rn, gn, bn)
	const delta = max - min

	let h = 0
	if (delta > 0) {
		if (max === rn) {
			h = ((gn - bn) / delta) % 6
		} else if (max === gn) {
			h = (bn - rn) / delta + 2
		} else {
			h = (rn - gn) / delta + 4
		}
		h *= 60
		if (h < 0) {
			h += 360
		}
	}

	const s = max === 0 ? 0 : (delta / max) * 100
	const v = max * 100

	return { h, s, v }
}

export const hsvToRgb = ({ h, s, v }: HsvColor): RgbColor => {
	const normalizedH = ((h % 360) + 360) % 360
	const normalizedS = Math.min(100, Math.max(0, s)) / 100
	const normalizedV = Math.min(100, Math.max(0, v)) / 100

	const c = normalizedV * normalizedS
	const x = c * (1 - Math.abs(((normalizedH / 60) % 2) - 1))
	const m = normalizedV - c

	let rPrime = 0
	let gPrime = 0
	let bPrime = 0

	if (normalizedH < 60) {
		rPrime = c
		gPrime = x
	} else if (normalizedH < 120) {
		rPrime = x
		gPrime = c
	} else if (normalizedH < 180) {
		gPrime = c
		bPrime = x
	} else if (normalizedH < 240) {
		gPrime = x
		bPrime = c
	} else if (normalizedH < 300) {
		rPrime = x
		bPrime = c
	} else {
		rPrime = c
		bPrime = x
	}

	return {
		r: clampByte((rPrime + m) * 255),
		g: clampByte((gPrime + m) * 255),
		b: clampByte((bPrime + m) * 255),
	}
}

const parseUnitOrPercent = (token: string, maxScale: number): number | null => {
	const lower = token.toLowerCase()
	if (lower === "none") {
		return 0
	}
	if (lower.endsWith("%")) {
		const num = Number(lower.slice(0, -1))
		return Number.isFinite(num) ? (num / 100) * maxScale : null
	}
	const num = Number(lower)
	return Number.isFinite(num) ? num : null
}

const parseAngleDegrees = (token: string): number | null => {
	const lower = token.toLowerCase()
	if (lower === "none") {
		return 0
	}
	if (lower.endsWith("deg")) {
		const val = Number(lower.slice(0, -3))
		return Number.isFinite(val) ? val : null
	}
	if (lower.endsWith("rad")) {
		const val = Number(lower.slice(0, -3))
		return Number.isFinite(val) ? (val * 180) / Math.PI : null
	}
	if (lower.endsWith("grad")) {
		const val = Number(lower.slice(0, -4))
		return Number.isFinite(val) ? val * 0.9 : null
	}
	if (lower.endsWith("turn")) {
		const val = Number(lower.slice(0, -4))
		return Number.isFinite(val) ? val * 360 : null
	}
	const val = Number(lower)
	return Number.isFinite(val) ? val : null
}

const hslToRgb = (hDeg: number, satPct: number, lightPct: number): RgbColor => {
	const h = ((hDeg % 360) + 360) % 360
	const s = clampUnit(satPct / 100)
	const l = clampUnit(lightPct / 100)

	const c = (1 - Math.abs(2 * l - 1)) * s
	const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
	const m = l - c / 2

	let rPrime = 0
	let gPrime = 0
	let bPrime = 0

	if (h < 60) {
		rPrime = c
		gPrime = x
	} else if (h < 120) {
		rPrime = x
		gPrime = c
	} else if (h < 180) {
		gPrime = c
		bPrime = x
	} else if (h < 240) {
		gPrime = x
		bPrime = c
	} else if (h < 300) {
		rPrime = x
		bPrime = c
	} else {
		rPrime = c
		bPrime = x
	}

	return {
		r: clampByte((rPrime + m) * 255),
		g: clampByte((gPrime + m) * 255),
		b: clampByte((bPrime + m) * 255),
	}
}

const linearToSrgb = (channel: number): number => {
	const clamped = clampUnit(channel)
	return clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * Math.pow(clamped, 1 / 2.4) - 0.055
}

const oklabToRgb = (l: number, a: number, b: number): RgbColor => {
	const lRoot = l + 0.3963377774 * a + 0.2158037573 * b
	const mRoot = l - 0.1055613458 * a - 0.0638541728 * b
	const sRoot = l - 0.0894841775 * a - 1.291485548 * b

	const lLin = lRoot * lRoot * lRoot
	const mLin = mRoot * mRoot * mRoot
	const sLin = sRoot * sRoot * sRoot

	const rLin = 4.0767416621 * lLin - 3.3077115913 * mLin + 0.2309699292 * sLin
	const gLin = -1.2684380046 * lLin + 2.6097574011 * mLin - 0.3413193965 * sLin
	const bLin = -0.0041960863 * lLin - 0.7034186147 * mLin + 1.707614701 * sLin

	return {
		r: clampByte(linearToSrgb(rLin) * 255),
		g: clampByte(linearToSrgb(gLin) * 255),
		b: clampByte(linearToSrgb(bLin) * 255),
	}
}

const extractFnChannels = (rawArgs: string): string[] | null => {
	const beforeAlpha = rawArgs.split("/")[0]?.trim()
	if (!beforeAlpha) {
		return null
	}
	const parts = beforeAlpha.includes(",") ? beforeAlpha.split(",") : beforeAlpha.split(/\s+/)
	const clean = parts
		.map((part) => {
			return part.trim()
		})
		.filter(Boolean)
	return clean.length >= 3 ? clean.slice(0, 3) : null
}

export const parseCssColorToRgb = (value: string): RgbColor | null => {
	const trimmed = value.trim()

	if (/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(trimmed)) {
		return {
			r: Number.parseInt(trimmed.slice(1, 3), 16),
			g: Number.parseInt(trimmed.slice(3, 5), 16),
			b: Number.parseInt(trimmed.slice(5, 7), 16),
		}
	}

	if (/^#[0-9a-fA-F]{3,4}$/.test(trimmed)) {
		return {
			r: Number.parseInt(`${trimmed[1]}${trimmed[1]}`, 16),
			g: Number.parseInt(`${trimmed[2]}${trimmed[2]}`, 16),
			b: Number.parseInt(`${trimmed[3]}${trimmed[3]}`, 16),
		}
	}

	const fnMatch = /^(rgb|rgba|hsl|hsla|oklch|oklab)\((.+)\)$/i.exec(trimmed)
	if (!fnMatch?.[1] || !fnMatch[2]) {
		return null
	}

	const fnName = fnMatch[1].toLowerCase()
	const channels = extractFnChannels(fnMatch[2])
	if (!channels) {
		return null
	}

	const [c1, c2, c3] = channels

	if (fnName === "rgb" || fnName === "rgba") {
		const r = parseUnitOrPercent(c1, 255)
		const g = parseUnitOrPercent(c2, 255)
		const b = parseUnitOrPercent(c3, 255)
		if (r === null || g === null || b === null) {
			return null
		}
		return { r: clampByte(r), g: clampByte(g), b: clampByte(b) }
	}

	if (fnName === "hsl" || fnName === "hsla") {
		const h = parseAngleDegrees(c1)
		const s = parseUnitOrPercent(c2, 100)
		const l = parseUnitOrPercent(c3, 100)
		if (h === null || s === null || l === null) {
			return null
		}
		return hslToRgb(h, s, l)
	}

	if (fnName === "oklch") {
		const l = parseUnitOrPercent(c1, 1)
		const c = parseUnitOrPercent(c2, 0.4)
		const h = parseAngleDegrees(c3)
		if (l === null || c === null || h === null) {
			return null
		}
		const hRad = (h * Math.PI) / 180
		return oklabToRgb(l, c * Math.cos(hRad), c * Math.sin(hRad))
	}

	if (fnName === "oklab") {
		const l = parseUnitOrPercent(c1, 1)
		const a = parseUnitOrPercent(c2, 0.4)
		const b = parseUnitOrPercent(c3, 0.4)
		if (l === null || a === null || b === null) {
			return null
		}
		return oklabToRgb(l, a, b)
	}

	return null
}

export const parseCssColorToHsv = (value: string, fallback = "#000000"): HsvColor => {
	const rgb = parseCssColorToRgb(value) ?? parseCssColorToRgb(fallback) ?? { r: 0, g: 0, b: 0 }
	return rgbToHsv(rgb)
}
