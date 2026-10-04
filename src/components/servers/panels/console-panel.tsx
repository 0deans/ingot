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
import { useTranslation } from "react-i18next"
import type { ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
	InputGroup,
	InputGroupAddon,
	InputGroupButton,
	InputGroupInput,
	InputGroupText,
} from "@/components/ui/input-group"
import { Toggle } from "@/components/ui/toggle"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { useServerStatus } from "@/services/server-data"
import { useServerLogs } from "@/services/server-service"
import { EmptyState } from "../shared/primitives"

function lineClass(line: string): string {
	if (line.startsWith("[Ingot] Failed")) return "text-destructive"
	if (line.startsWith("[Ingot]")) return "text-info"
	if (/\bERROR\]|ERROR:|Exception|\bat [a-z]+\./.test(line)) return "text-destructive"
	if (/\bWARN\]|WARN:/.test(line)) return "text-warning"
	if (/joined the game|left the game|Player connected:|Player disconnected:/.test(line))
		return "text-primary"
	return "text-foreground/80"
}

/** Returns whether within 40px of bottom */
function isAtBottom(el: HTMLDivElement): boolean {
	return el.scrollHeight - el.scrollTop - el.clientHeight < 40
}

export function ConsolePanel({ server, className }: { server: ServerConfig; className?: string }) {
	const { t } = useTranslation()
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
		setAtBottom(isAtBottom(el))
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
			setAtBottom(isAtBottom(el))
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
		<div className={cn("relative flex min-h-0 flex-col overflow-hidden bg-background", className)}>
			<div className="flex shrink-0 items-center gap-1 px-2 pt-2">
				<Toggle variant="outline" size="sm" pressed={autoScroll} onPressedChange={setAutoScroll}>
					<ArrowDownToLine />
					{autoScroll ? t("console.autoScrollOn") : t("console.autoScrollOff")}
				</Toggle>
				<span className="ml-auto" />
				<IconButton
					label={t("console.copyAll")}
					onClick={async () => {
						await navigator.clipboard.writeText(logs.join("\n"))
						setCopied(true)
						setTimeout(() => setCopied(false), 1500)
					}}
				>
					{copied ? <Check className="size-3.5 text-primary" /> : <Copy className="size-3.5" />}
				</IconButton>
				<IconButton label={t("console.clear")} onClick={clearLogs}>
					<Eraser className="size-3.5" />
				</IconButton>
			</div>

			<div
				ref={scrollRef}
				className="scroll-fade-y min-h-0 flex-1 overflow-y-auto px-3 py-2 font-mono text-2xs leading-relaxed"
			>
				{logs.length === 0 ? (
					<div className="flex h-full items-center justify-center font-sans">
						<EmptyState
							icon={Terminal}
							title={isRunning ? t("console.waiting") : t("console.offline")}
							description={isRunning ? undefined : t("console.offlineHint")}
						/>
					</div>
				) : (
					logs.map((line, i) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: log lines are append-only
						<div key={i} className={cn("wrap-break-word whitespace-pre-wrap", lineClass(line))}>
							{line}
						</div>
					))
				)}
			</div>

			{!atBottom && (
				<Button
					variant="secondary"
					size="xs"
					onClick={jumpToLatest}
					className="absolute bottom-16 left-1/2 -translate-x-1/2 shadow-lg"
				>
					<ArrowDown />
					{t("console.latest")}
				</Button>
			)}

			<div className="shrink-0 px-2 pb-2">
				<InputGroup>
					<InputGroupAddon>
						<InputGroupText className="font-mono text-primary">/</InputGroupText>
					</InputGroupAddon>
					<InputGroupInput
						value={input}
						onChange={(e) => setInput(e.target.value)}
						onKeyDown={onKeyDown}
						disabled={!isRunning}
						placeholder={isRunning ? t("console.placeholder") : t("console.offline")}
						autoCapitalize="off"
						autoCorrect="off"
						spellCheck={false}
						enterKeyHint="send"
						className="font-mono"
					/>
					<InputGroupAddon align="inline-end">
						<InputGroupButton
							variant="default"
							size="icon-xs"
							onClick={send}
							disabled={!isRunning || !input.trim()}
							aria-label={t("console.send")}
						>
							<CornerDownLeft />
						</InputGroupButton>
					</InputGroupAddon>
				</InputGroup>
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
		<Tooltip>
			<TooltipTrigger
				render={
					<Button
						variant="ghost"
						size="icon-sm"
						aria-label={label}
						onClick={onClick}
						className="text-muted-foreground"
					/>
				}
			>
				{children}
			</TooltipTrigger>
			<TooltipContent>{label}</TooltipContent>
		</Tooltip>
	)
}
