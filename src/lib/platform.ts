/**
 * Detects whether Ingot is running in a mobile/Android environment
 * Supports query param override (?mobile=true) for development/testing
 */
export const isMobileEnvironment = (): boolean => {
	if (typeof window === "undefined") return false
	const urlParams = new URLSearchParams(window.location.search)
	if (urlParams.get("mobile") === "true") return true
	if (urlParams.get("desktop") === "true") return false

	const ua = navigator.userAgent.toLowerCase()
	return (
		ua.includes("android") ||
		ua.includes("iphone") ||
		ua.includes("mobile") ||
		(window as unknown as { __TAURI_MOBILE__?: boolean }).__TAURI_MOBILE__ !== undefined
	)
}
