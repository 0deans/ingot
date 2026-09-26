import { Server, Signal } from "lucide-react"
import { cn } from "@/lib/utils"

/** Minecraft §-color codes */
export const MC_COLORS: Record<string, string> = {
	"0": "#000000",
	"1": "#0000AA",
	"2": "#00AA00",
	"3": "#00AAAA",
	"4": "#AA0000",
	"5": "#AA00AA",
	"6": "#FFAA00",
	"7": "#AAAAAA",
	"8": "#555555",
	"9": "#5555FF",
	a: "#55FF55",
	b: "#55FFFF",
	c: "#FF5555",
	d: "#FF55FF",
	e: "#FFFF55",
	f: "#FFFFFF",
}

interface MotdSpan {
	key: string
	text: string
	color?: string
	bold?: boolean
	italic?: boolean
	underline?: boolean
	strikethrough?: boolean
}

/** Splits a MOTD into styled spans per line. Supports § and & codes and "\n" */
export function parseMotd(motd: string): MotdSpan[][] {
	const lines: MotdSpan[][] = [[]]
	let counter = 0
	let style: Omit<MotdSpan, "key" | "text"> = {}
	let text = ""
	const flush = () => {
		if (text) lines[lines.length - 1].push({ key: `s${counter++}`, text, ...style })
		text = ""
	}
	for (let i = 0; i < motd.length; i++) {
		const ch = motd[i]
		if ((ch === "§" || ch === "&") && i + 1 < motd.length) {
			const code = motd[i + 1].toLowerCase()
			flush()
			if (code in MC_COLORS) style = { color: MC_COLORS[code] }
			else if (code === "l") style = { ...style, bold: true }
			else if (code === "o") style = { ...style, italic: true }
			else if (code === "n") style = { ...style, underline: true }
			else if (code === "m") style = { ...style, strikethrough: true }
			else if (code === "r") style = {}
			i++
		} else if (ch === "\n" || (ch === "\\" && motd[i + 1] === "n")) {
			flush()
			lines.push([])
			if (ch === "\\") i++
		} else {
			text += ch
		}
	}
	flush()
	return lines
}

/** Renders MOTD text with Minecraft colors (max two lines, like the game) */
export function MotdText({ motd, className }: { motd: string; className?: string }) {
	const lines = parseMotd(motd).slice(0, 2)
	return (
		<div className={cn("font-mono leading-snug [overflow-wrap:anywhere]", className)}>
			{lines.map((line, i) => (
				// biome-ignore lint/suspicious/noArrayIndexKey: lines have no identity besides order
				<div key={i} className={cn("truncate", line.length === 0 && "h-[1.2em]")}>
					{line.map((span) => (
						<span
							key={span.key}
							style={{
								color: span.color ?? "#AAAAAA",
								fontWeight: span.bold ? 700 : undefined,
								fontStyle: span.italic ? "italic" : undefined,
								textDecoration:
									[span.underline && "underline", span.strikethrough && "line-through"]
										.filter(Boolean)
										.join(" ") || undefined,
							}}
						>
							{span.text}
						</span>
					))}
				</div>
			))}
		</div>
	)
}

/** How the server appears in the Minecraft multiplayer list */
export function ServerListPreview({
	motd,
	icon,
	name,
	online = 0,
	max = 20,
	isRunning = false,
}: {
	motd: string
	icon?: string | null
	name: string
	online?: number
	max?: number
	isRunning?: boolean
}) {
	return (
		<div className="flex min-w-0 items-start gap-3 rounded-xl border border-zinc-800 bg-[#0e0e10] p-3 shadow-inner">
			<div className="flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-md bg-zinc-800">
				{icon ? (
					<img src={icon} alt="" className="size-full object-cover [image-rendering:pixelated]" />
				) : (
					<Server className="size-5 text-zinc-500" />
				)}
			</div>
			<div className="flex min-w-0 flex-1 flex-col gap-0.5">
				<div className="flex items-center justify-between gap-2">
					<span className="truncate font-medium text-sm text-white">{name}</span>
					<span className="flex shrink-0 items-center gap-1.5 font-mono text-[11px] text-zinc-400">
						{online}/{max}
						<Signal className={cn("size-3", isRunning ? "text-emerald-400" : "text-zinc-600")} />
					</span>
				</div>
				<MotdText motd={motd} className="text-xs" />
			</div>
		</div>
	)
}
