import {
	ArrowDown,
	Check,
	Copy,
	CornerDownLeft,
	Loader2,
	Play,
	Send,
	Square,
	Terminal,
	Trash2,
} from "lucide-react"
import { useEffect, useRef, useState } from "react"
import type { ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"
import { serverService, useRunningServers, useServerLogs } from "@/services/server-service"

export interface ServerConsoleDialogProps {
	server: ServerConfig | null
	open: boolean
	onOpenChange: (open: boolean) => void
}

export default function ServerConsoleDialog({
	server,
	open,
	onOpenChange,
}: ServerConsoleDialogProps) {
	const serverId = server?.id || null
	const { logs, isLoadingHistory, sendCommand, clearLogs } = useServerLogs(open ? serverId : null)
	const { runningMap } = useRunningServers()

	const runningInfo = serverId ? runningMap.get(serverId) : undefined
	const isRunning = Boolean(runningInfo && runningInfo.status === "running")
	const isStarting = Boolean(runningInfo && runningInfo.status === "starting")

	const [commandInput, setCommandInput] = useState("")
	const [history, setHistory] = useState<string[]>([])
	const [historyIndex, setHistoryIndex] = useState(-1)
	const [autoScroll, setAutoScroll] = useState(true)
	const [copied, setCopied] = useState(false)
	const [isStopping, setIsStopping] = useState(false)

	const scrollBottomRef = useRef<HTMLDivElement | null>(null)
	const inputRef = useRef<HTMLInputElement | null>(null)

	// Auto-scroll when new logs arrive
	useEffect(() => {
		if (autoScroll && scrollBottomRef.current) {
			scrollBottomRef.current.scrollIntoView({ behavior: "smooth" })
		}
	}, [autoScroll])

	// Focus input on open
	useEffect(() => {
		if (open) {
			setTimeout(() => {
				inputRef.current?.focus()
			}, 100)
		}
	}, [open])

	const handleSendCommand = async () => {
		const cmd = commandInput.trim()
		if (!cmd || !serverId) return

		setHistory((prev) => [cmd, ...prev.filter((h) => h !== cmd)].slice(0, 50))
		setHistoryIndex(-1)
		setCommandInput("")

		try {
			await sendCommand(cmd)
		} catch (err) {
			console.error("Failed to send command:", err)
		}
	}

	const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "Enter") {
			e.preventDefault()
			handleSendCommand()
		} else if (e.key === "ArrowUp") {
			e.preventDefault()
			if (history.length > 0) {
				const nextIndex = Math.min(history.length - 1, historyIndex + 1)
				setHistoryIndex(nextIndex)
				setCommandInput(history[nextIndex])
			}
		} else if (e.key === "ArrowDown") {
			e.preventDefault()
			if (historyIndex > 0) {
				const nextIndex = historyIndex - 1
				setHistoryIndex(nextIndex)
				setCommandInput(history[nextIndex])
			} else if (historyIndex === 0) {
				setHistoryIndex(-1)
				setCommandInput("")
			}
		}
	}

	const handleCopyLogs = async () => {
		try {
			await navigator.clipboard.writeText(logs.join("\n"))
			setCopied(true)
			setTimeout(() => setCopied(false), 2000)
		} catch (err) {
			console.error("Failed to copy logs:", err)
		}
	}

	const handleStartServer = async () => {
		if (!serverId) return
		try {
			await serverService.startServer(serverId)
		} catch (err) {
			console.error("Failed to start server:", err)
		}
	}

	const handleStopServer = async () => {
		if (!serverId) return
		setIsStopping(true)
		try {
			await serverService.stopServer(serverId)
		} catch (err) {
			console.error("Failed to stop server:", err)
		} finally {
			setIsStopping(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="flex h-[680px] max-h-[92vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-4xl">
				{/* Dialog Header */}
				<DialogHeader className="flex shrink-0 flex-row items-center justify-between border-border/40 border-b bg-zinc-950/60 px-5 py-3.5">
					<div className="flex items-center gap-3">
						<div className="flex size-9 items-center justify-center rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-400">
							<Terminal className="size-4" />
						</div>
						<div>
							<div className="flex items-center gap-2">
								<DialogTitle className="font-semibold text-base text-foreground">
									{server?.name || "Server Console"}
								</DialogTitle>
								{isRunning ? (
									<span className="flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-0.5 font-medium text-[10px] text-emerald-400">
										<span className="size-1.5 animate-pulse rounded-full bg-emerald-400" />
										Online (Port {server?.port || 25565})
									</span>
								) : isStarting ? (
									<span className="flex items-center gap-1.5 rounded-full border border-amber-500/30 bg-amber-500/10 px-2.5 py-0.5 font-medium text-[10px] text-amber-400">
										<Loader2 className="size-3 animate-spin" />
										Starting...
									</span>
								) : (
									<span className="rounded-full border border-zinc-800 bg-zinc-900 px-2 py-0.5 font-medium text-[10px] text-zinc-400">
										Offline
									</span>
								)}
							</div>
							<DialogDescription className="font-mono text-[11px] text-muted-foreground">
								Core: {server?.core?.toUpperCase()} {server?.gameVersion} • Port:{" "}
								{server?.port || 25565}
								{runningInfo?.pid ? ` • PID: ${runningInfo.pid}` : ""}
							</DialogDescription>
						</div>
					</div>

					{/* Header Actions */}
					<div className="flex items-center gap-2">
						{isRunning ? (
							<Button
								variant="destructive"
								size="sm"
								onClick={handleStopServer}
								disabled={isStopping}
								className="h-8 gap-1.5 text-xs"
							>
								{isStopping ? (
									<Loader2 className="size-3.5 animate-spin" />
								) : (
									<Square className="size-3.5 fill-current" />
								)}
								<span>Stop Server</span>
							</Button>
						) : (
							<Button
								size="sm"
								onClick={handleStartServer}
								disabled={isStarting}
								className="h-8 gap-1.5 bg-emerald-600 text-white text-xs hover:bg-emerald-500"
							>
								{isStarting ? (
									<Loader2 className="size-3.5 animate-spin" />
								) : (
									<Play className="size-3.5 fill-current" />
								)}
								<span>Start Server</span>
							</Button>
						)}
					</div>
				</DialogHeader>

				{/* Toolbar Actions */}
				<div className="flex shrink-0 items-center justify-between border-border/30 border-b bg-zinc-950/30 px-4 py-1.5 text-xs">
					<div className="flex items-center gap-2 text-[11px] text-zinc-400">
						<span className="font-mono">
							{logs.length} line{logs.length === 1 ? "" : "s"}
						</span>
					</div>

					<div className="flex items-center gap-1.5">
						<Button
							variant="ghost"
							size="xs"
							onClick={() => setAutoScroll(!autoScroll)}
							className={cn(
								"h-7 gap-1 px-2 text-[11px]",
								autoScroll ? "text-emerald-400" : "text-muted-foreground",
							)}
						>
							<ArrowDown className="size-3" />
							<span>Auto-scroll: {autoScroll ? "On" : "Off"}</span>
						</Button>

						<Button
							variant="ghost"
							size="xs"
							onClick={handleCopyLogs}
							className="h-7 gap-1 px-2 text-[11px] text-muted-foreground hover:text-foreground"
						>
							{copied ? <Check className="size-3 text-emerald-400" /> : <Copy className="size-3" />}
							<span>{copied ? "Copied!" : "Copy"}</span>
						</Button>

						<Button
							variant="ghost"
							size="xs"
							onClick={clearLogs}
							className="h-7 gap-1 px-2 text-[11px] text-muted-foreground hover:text-foreground"
						>
							<Trash2 className="size-3" />
							<span>Clear</span>
						</Button>
					</div>
				</div>

				{/* Console Output Screen */}
				<ScrollArea className="flex-1 bg-black/90" viewportClassName="p-4">
					{isLoadingHistory ? (
						<div className="flex h-full min-h-[300px] flex-col items-center justify-center gap-2 text-zinc-500">
							<Loader2 className="size-6 animate-spin text-emerald-400" />
							<span className="text-xs">Loading console logs...</span>
						</div>
					) : logs.length === 0 ? (
						<div className="flex h-full min-h-[300px] flex-col items-center justify-center gap-2 text-center text-zinc-600">
							<Terminal className="size-8 opacity-40" />
							<p className="font-medium text-xs text-zinc-400">Console is idle</p>
							<p className="text-[11px] text-zinc-500">
								Start the server to view live Minecraft boot and gameplay logs.
							</p>
						</div>
					) : (
						<div className="flex flex-col gap-0.5 font-mono text-[12px] leading-relaxed">
							{logs.map((line, idx) => {
								const isWarn = line.includes("[WARN]") || line.includes("WARN:")
								const isError =
									line.includes("[ERROR]") || line.includes("ERROR:") || line.includes("Exception:")
								const isSuccess =
									line.includes("Done (") ||
									line.includes('For help, type "help"') ||
									line.includes("Preparing spawn area")
								const isJoin = line.includes("joined the game") || line.includes("logged in")
								const isLeave = line.includes("left the game") || line.includes("lost connection")

								return (
									<div
										// biome-ignore lint/suspicious/noArrayIndexKey: Log output lines
										key={idx}
										className={cn(
											"select-text whitespace-pre-wrap break-all selection:bg-zinc-800",
											isError
												? "text-rose-400"
												: isWarn
													? "text-amber-300"
													: isSuccess
														? "font-semibold text-emerald-400"
														: isJoin
															? "text-cyan-400"
															: isLeave
																? "text-orange-400"
																: "text-zinc-300",
										)}
									>
										{line}
									</div>
								)
							})}
							<div ref={scrollBottomRef} />
						</div>
					)}
				</ScrollArea>

				{/* Stdin Command Input Bar */}
				<div className="flex shrink-0 items-center gap-2 border-border/40 border-t bg-zinc-950 p-3">
					<div className="relative flex-1">
						<span className="absolute top-1/2 left-3 -translate-y-1/2 select-none font-bold font-mono text-emerald-500 text-xs">
							&gt;
						</span>
						<Input
							ref={inputRef}
							type="text"
							placeholder={
								isRunning
									? "Enter server command (e.g. op, gamemode, say, stop)..."
									: "Server is offline. Start the server to execute commands."
							}
							disabled={!isRunning}
							value={commandInput}
							onChange={(e) => setCommandInput(e.target.value)}
							onKeyDown={handleKeyDown}
							className="h-9 border-zinc-800 bg-zinc-900/90 pr-10 pl-7 font-mono text-xs text-zinc-100 placeholder:text-zinc-600 focus-visible:ring-emerald-500/50"
						/>
						<button
							type="button"
							onClick={handleSendCommand}
							disabled={!isRunning || !commandInput.trim()}
							className="absolute top-1/2 right-2 -translate-y-1/2 rounded p-1 text-zinc-400 transition-colors hover:text-zinc-100 disabled:opacity-40"
							title="Send command"
						>
							<CornerDownLeft className="size-3.5" />
						</button>
					</div>

					<Button
						size="sm"
						onClick={handleSendCommand}
						disabled={!isRunning || !commandInput.trim()}
						className="h-9 gap-1.5 bg-emerald-600 px-3 font-medium text-white text-xs hover:bg-emerald-500 disabled:opacity-40"
					>
						<Send className="size-3.5" />
						<span className="hidden sm:inline">Send</span>
					</Button>
				</div>
			</DialogContent>
		</Dialog>
	)
}
