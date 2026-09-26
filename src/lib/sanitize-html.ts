/**
 * Cleans HTML from third-party sources (plugin pages) before it's rendered. Inside the
 * app's WebView, script could reach the Tauri backend, so anything executable goes.
 */

const BLOCKED_TAGS =
	"script,style,iframe,frame,frameset,object,embed,form,input,button,textarea,select,meta,link,base,svg,math,template"

const URL_ATTRS = ["href", "src", "xlink:href", "action", "formaction", "poster"]

function isSafeUrl(value: string, attr: string, tag: string): boolean {
	// Browsers ignore whitespace and control characters inside the scheme (e.g. a tab in "javascript:")
	const url = [...value.toLowerCase()].filter((c) => c.charCodeAt(0) > 32).join("")
	if (url.startsWith("javascript:") || url.startsWith("vbscript:")) return false
	if (url.startsWith("data:"))
		return attr === "src" && tag === "img" && url.startsWith("data:image/")
	return true
}

export function sanitizeHtml(html: string): string {
	const doc = new DOMParser().parseFromString(html, "text/html")
	for (const el of doc.body.querySelectorAll(BLOCKED_TAGS)) el.remove()

	for (const el of doc.body.querySelectorAll("*")) {
		const tag = el.tagName.toLowerCase()
		for (const attr of [...el.attributes]) {
			const name = attr.name.toLowerCase()
			if (name.startsWith("on") || name === "style" || name === "srcdoc") {
				el.removeAttribute(attr.name)
			} else if (URL_ATTRS.includes(name) && !isSafeUrl(attr.value, name, tag)) {
				el.removeAttribute(attr.name)
			}
		}
		if (tag === "a") {
			el.setAttribute("target", "_blank")
			el.setAttribute("rel", "noopener noreferrer")
		}
		if (tag === "img") el.setAttribute("loading", "lazy")
	}
	return doc.body.innerHTML
}
