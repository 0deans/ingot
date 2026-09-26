import {
	ArrowDown,
	ArrowDownToLine,
	Check,
	Copy,
	CornerDownLeft,
	Eraser,
	Terminal,
} from "lucide-react"
import { type KeyboardEvent, useEffect, useLayoutEffect, useRef, useState } from "react"
import type { ServerConfig } from "@/bindings"
import { cn } from "@/lib/utils"
import { useServerStatus } from "@/services/server-data"
import { useServerLogs } from "@/services/server-service"
import { EmptyState } from "../shared/primitives"

function lineClass(line: string): string {
	if (line.startsWith("[Ingot] Failed")) return "text-rose-400"
	if (line.startsWith("[Ingot]")) return "text-sky-300"
	if (/\bERROR\]|ERROR:|Exception|\bat [a-z]+\./.test(line)) return "text-rose-400"
	if (/\bWARN\]|WARN:/.test(line)) return "text-amber-300"
	if (/joined the game|left the game/.test(line)) return "text-emerald-300"
	return "text-zinc-300"
}

/** Edge fades only where there's more to scroll (same vars as .scroll-fade elsewhere); returns whether at the bottom */
function updateFades(el: HTMLDivElement): boolean {
	const below = Math.max(0, el.scrollHeight - el.scrollTop - el.clientHeight)
	el.style.setProperty("--scroll-area-overflow-y-start", `${el.scrollTop}px`)
	el.style.setProperty("--scroll-area-overflow-y-end", `${below}px`)
	return below < 40
}

export function ConsolePanel({ server, className }: { server: ServerConfig; className?: string }) {
	const { isRunning } = useServerStatus(server.id)
	const { logs, sendCommand, clearLogs } = useServerLogs(server.id)
	const [input, setInput] = useState("")
	const [history, setHistory] = useState<string[]>([])
	const [historyIndex, setHistoryIndex] = useState(-1)
	// Only the user turns auto-scroll back on; scrolling up turns it off
	const [autoScroll, setAutoScroll] = useState(true)
	const [atBottom, setAtBottom] = useState(true)
	const [copied, setCopied] = useState(false)
	const scrollRef = useRef<HTMLDivElement>(null)
	const lastScroll = useRef({ top: 0, height: 0 })

	useLayoutEffect(() => {
		const el = scrollRef.current
		if (!el || logs.length < 0) return
		if (autoScroll) el.scrollTop = el.scrollHeight
		lastScroll.current = { top: el.scrollTop, height: el.scrollHeight }
		setAtBottom(updateFades(el))
	}, [logs, autoScroll])

	useEffect(() => {
		const el = scrollRef.current
		if (!el) return
		const onScroll = () => {
			const prev = lastScroll.current
			// A move up while the content didn't shrink is the user reading back
			// (clearing or trimming logs also lowers scrollTop, but shrinks the content)
			if (el.scrollTop < prev.top - 2 && el.scrollHeight >= prev.height) setAutoScroll(false)
			lastScroll.current = { top: el.scrollTop, height: el.scrollHeight }
			setAtBottom(updateFades(el))
		}
		el.addEventListener("scroll", onScroll, { passive: true })
		return () => el.removeEventListener("scroll", onScroll)
	}, [])

	const jumpToLatest = () => {
		const el = scrollRef.current
		if (el) el.scrollTo({ top: el.scrollHeight, behavior: "smooth" })
	}

	const send = async () => {
		const cmd = input.trim().replace(/^\//, "")
		if (!cmd) return
		setHistory((h) => [cmd, ...h.filter((x) => x !== cmd)].slice(0, 50))
		setHistoryIndex(-1)
		setInput("")
		await sendCommand(cmd).catch(() => {})
	}

	const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "Enter") {
			e.preventDefault()
			send()
		} else if (e.key === "ArrowUp" && history.length > 0) {
			e.preventDefault()
			const i = Math.min(history.length - 1, historyIndex + 1)
			setHistoryIndex(i)
			setInput(history[i])
		} else if (e.key === "ArrowDown") {
			e.preventDefault()
			const i = historyIndex - 1
			setHistoryIndex(Math.max(-1, i))
			setInput(i >= 0 ? history[i] : "")
		}
	}

	return (
		<div className={cn("relative flex min-h-0 flex-col overflow-hidden bg-[#0a0a0c]", className)}>
			<div className="flex shrink-0 items-center gap-1 px-2 pt-2">
				<button
					type="button"
					role="switch"
					aria-checked={autoScroll}
					onClick={() => setAutoScroll((v) => !v)}
					className={cn(
						"flex h-7 items-center gap-1.5 rounded-full border px-2.5 font-medium text-[11px] transition-colors",
						autoScroll
							? "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
							: "border-zinc-800 text-zinc-500 hover:text-zinc-300",
					)}
				>
					<ArrowDownToLine className="size-3" />
					Auto-scroll {autoScroll ? "on" : "off"}
				</button>
				<span className="ml-auto" />
				<IconButton
					label="Copy all"
					onClick={async () => {
						await navigator.clipboard.writeText(logs.join("\n"))
						setCopied(true)
						setTimeout(() => setCopied(false), 1500)
					}}
				>
					{copied ? <Check className="size-3.5 text-emerald-400" /> : <Copy className="size-3.5" />}
				</IconButton>
				<IconButton label="Clear" onClick={clearLogs}>
					<Eraser className="size-3.5" />
				</IconButton>
			</div>

			<div
				ref={scrollRef}
				className="scroll-fade min-h-0 flex-1 overflow-y-auto px-3 py-2 font-mono text-[11px] leading-relaxed"
			>
				{logs.length === 0 ? (
					<div className="flex h-full items-center justify-center font-sans">
						<EmptyState
							icon={Terminal}
							title={isRunning ? "Waiting for output..." : "Server is offline"}
							description={isRunning ? undefined : "Start the server to see its console."}
						/>
					</div>
				) : (
					logs.map((line, i) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: log lines are append-only
						<div key={i} className={cn("whitespace-pre-wrap break-words", lineClass(line))}>
							{line}
						</div>
					))
				)}
			</div>

			{!atBottom && (
				<button
					type="button"
					onClick={jumpToLatest}
					className="absolute bottom-16 left-1/2 flex -translate-x-1/2 items-center gap-1 rounded-full border border-zinc-700 bg-zinc-900/95 px-3 py-1.5 text-[11px] text-zinc-200 shadow-lg"
				>
					<ArrowDown className="size-3" /> Latest
				</button>
			)}

			<div className="shrink-0 px-2 pb-2">
				<div className="flex items-center gap-2 rounded-2xl bg-zinc-900/70 p-1 pl-3 ring-1 ring-zinc-800/60 focus-within:ring-zinc-700">
					<span className="font-mono text-emerald-400 text-xs">/</span>
					<input
						value={input}
						onChange={(e) => setInput(e.target.value)}
						onKeyDown={onKeyDown}
						disabled={!isRunning}
						placeholder={isRunning ? "Type a command, e.g. time set day" : "Server is offline"}
						autoCapitalize="off"
						autoCorrect="off"
						spellCheck={false}
						enterKeyHint="send"
						className="h-9 min-w-0 flex-1 bg-transparent font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-600"
					/>
					<button
						type="button"
						onClick={send}
						disabled={!isRunning || !input.trim()}
						aria-label="Send command"
						className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-emerald-600 text-white transition-colors hover:bg-emerald-500 disabled:bg-transparent disabled:text-zinc-600"
					>
						<CornerDownLeft className="size-4" />
					</button>
				</div>
			</div>
		</div>
	)
}

function IconButton({
	label,
	onClick,
	children,
}: {
	label: string
	onClick: () => void
	children: React.ReactNode
}) {
	return (
		<button
			type="button"
			title={label}
			aria-label={label}
			onClick={onClick}
			className="flex size-7 items-center justify-center rounded-lg text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200"
		>
			{children}
		</button>
	)
}
