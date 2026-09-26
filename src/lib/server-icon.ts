/**
 * Minecraft only accepts server-icon.png as a 64×64 PNG; anything else is ignored
 * (and logged as an error by the server).
 */

const PNG_SIGNATURE = "iVBORw0KGgo"

async function loadImage(src: string): Promise<HTMLImageElement> {
	const img = new Image()
	img.src = src
	await img.decode()
	return img
}

/** Center-crops to a square and scales to 64×64, returning a PNG data URL */
export async function toServerIcon(src: string | Blob): Promise<string> {
	const url = typeof src === "string" ? src : URL.createObjectURL(src)
	try {
		const img = await loadImage(url)
		const side = Math.min(img.width, img.height)
		const canvas = document.createElement("canvas")
		canvas.width = 64
		canvas.height = 64
		const ctx = canvas.getContext("2d")
		if (!ctx) throw new Error("Canvas unavailable")
		ctx.imageSmoothingEnabled = side > 64
		ctx.imageSmoothingQuality = "high"
		ctx.drawImage(img, (img.width - side) / 2, (img.height - side) / 2, side, side, 0, 0, 64, 64)
		return canvas.toDataURL("image/png")
	} finally {
		if (typeof src !== "string") URL.revokeObjectURL(url)
	}
}

/** Whether a server-icon data URL is something Minecraft will actually show */
export async function isValidServerIcon(dataUrl: string): Promise<boolean> {
	const base64 = dataUrl.slice(dataUrl.indexOf(",") + 1)
	if (!base64.startsWith(PNG_SIGNATURE)) return false
	try {
		const img = await loadImage(dataUrl)
		return img.naturalWidth === 64 && img.naturalHeight === 64
	} catch {
		return false
	}
}
