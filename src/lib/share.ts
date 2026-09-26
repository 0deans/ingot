/** Sharing text and files: Android share sheet, then the Web Share API, then clipboard */

interface AndroidShareBridge {
	shareText?(title: string, text: string): void
	shareFile?(path: string, mimeType: string, title: string): void
}

function androidBridge(): AndroidShareBridge | undefined {
	return (window as Window & { IngotHost?: AndroidShareBridge }).IngotHost
}

/** Resolves to how the text was shared, so the UI can say "Copied" when relevant */
export async function shareText(title: string, text: string): Promise<"shared" | "copied"> {
	const android = androidBridge()
	if (android?.shareText) {
		android.shareText(title, text)
		return "shared"
	}
	if (navigator.share) {
		try {
			await navigator.share({ title, text })
			return "shared"
		} catch (e) {
			// The user closed the share sheet; don't fall through to copying
			if (e instanceof DOMException && e.name === "AbortError") return "shared"
		}
	}
	await navigator.clipboard.writeText(text)
	return "copied"
}

/** Whether files can be handed to other apps (Android share sheet) */
export function canShareFiles(): boolean {
	return Boolean(androidBridge()?.shareFile)
}

export function shareFile(path: string, mimeType: string, title: string): void {
	androidBridge()?.shareFile?.(path, mimeType, title)
}
