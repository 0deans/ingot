import { ArrowDown, Send, Terminal } from "lucide-react"
import { memo, useCallback, useEffect, useRef, useState } from "react"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { serverService, useServerLogs } from "@/services/server-service"

interface MobileConsoleSheetProps {
	serverId: string
	serverName: string
	isOpen: boolean
	onClose: () => void
}

const QUICK_COMMANDS = [
	{ label: "Help", cmd: "help" },
	{ label: "List", cmd: "list" },
	{ label: "Day", cmd: "time set day" },
	{ label: "Weather Clear", cmd: "weather clear" },
	{ label: "Save", cmd: "save-all" },
]

export const MobileConsoleSheet = memo(
	({ serverId, serverName, isOpen, onClose }: MobileConsoleSheetProps) => {
		const { logs } = useServerLogs(serverId)
		const [command, setCommand] = useState("")
		const [isAutoScroll, setIsAutoScroll] = useState(true)
		const scrollBottomRef = useRef<HTMLDivElement>(null)

		const scrollToBottom = useCallback(() => {
			if (scrollBottomRef.current) {
				scrollBottomRef.current.scrollIntoView({ behavior: "smooth" })
			}
		}, [])

		// biome-ignore lint/correctness/useExhaustiveDependencies: auto-scroll when new log lines are added
		useEffect(() => {
			if (isAutoScroll && isOpen) {
				scrollToBottom()
			}
		}, [logs.length, isAutoScroll, isOpen, scrollToBottom])

		const handleSendCommand = useCallback(
			async (cmdToSend?: string) => {
				const cmd = (cmdToSend ?? command).trim()
				if (!cmd) return
				try {
					await serverService.sendCommand(serverId, cmd)
					setCommand("")
					setIsAutoScroll(true)
				} catch (e) {
					console.error("Failed to send command:", e)
				}
			},
			[command, serverId],
		)

		return (
			<Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
				<DialogContent className="flex h-[85vh] w-[95vw] max-w-md flex-col border-zinc-800 bg-zinc-950 p-4 text-zinc-100">
					<DialogHeader className="flex flex-row items-center justify-between border-zinc-800 border-b pb-3">
						<div className="flex items-center gap-2">
							<Terminal className="size-4 text-emerald-400" />
							<DialogTitle className="max-w-[200px] truncate font-semibold text-sm">
								{serverName} Console
							</DialogTitle>
						</div>
						<div className="flex items-center gap-1">
							<Button
								variant="ghost"
								size="sm"
								className="h-7 px-2 text-xs text-zinc-400 hover:text-zinc-200"
								onClick={() => setIsAutoScroll(!isAutoScroll)}
							>
								<ArrowDown className={`mr-1 size-3.5 ${isAutoScroll ? "text-emerald-400" : ""}`} />
								{isAutoScroll ? "Stick" : "Free"}
							</Button>
						</div>
					</DialogHeader>

					{/* Quick command chips */}
					<div className="scrollbar-none flex items-center gap-1.5 overflow-x-auto py-1">
						{QUICK_COMMANDS.map((qc) => (
							<button
								key={qc.label}
								type="button"
								onClick={() => handleSendCommand(qc.cmd)}
								className="shrink-0 rounded-full border border-zinc-800 bg-zinc-900/80 px-2.5 py-0.5 text-xs text-zinc-300 hover:bg-zinc-800 active:scale-95"
							>
								{qc.label}
							</button>
						))}
					</div>

					{/* Virtual terminal logs */}
					<div className="flex-1 select-text overflow-y-auto rounded-lg border border-zinc-900 bg-black/60 p-3 font-mono text-[11px] leading-relaxed">
						{logs.length === 0 ? (
							<div className="flex h-full items-center justify-center text-zinc-500">
								No logs available
							</div>
						) : (
							logs.map((line, idx) => {
								let colorClass = "text-zinc-300"
								if (line.includes("WARN") || line.includes("WARNING")) colorClass = "text-amber-400"
								if (line.includes("ERROR") || line.includes("Exception"))
									colorClass = "text-rose-400"
								if (line.includes("joined the game")) colorClass = "text-emerald-400 font-medium"
								if (line.includes("left the game")) colorClass = "text-amber-300"

								return (
									// biome-ignore lint/suspicious/noArrayIndexKey: log lines have no unique ids
									<div key={idx} className={`break-words ${colorClass}`}>
										{line}
									</div>
								)
							})
						)}
						<div ref={scrollBottomRef} />
					</div>

					{/* Input command box */}
					<form
						onSubmit={(e) => {
							e.preventDefault()
							handleSendCommand()
						}}
						className="flex items-center gap-2 pt-2"
					>
						<Input
							value={command}
							onChange={(e) => setCommand(e.target.value)}
							placeholder="Type Minecraft command (e.g. op steve)..."
							className="h-9 border-zinc-800 bg-zinc-900 font-mono text-xs"
						/>
						<Button
							type="submit"
							size="sm"
							className="h-9 bg-emerald-600 px-3 hover:bg-emerald-500"
						>
							<Send className="size-3.5" />
						</Button>
					</form>
				</DialogContent>
			</Dialog>
		)
	},
)

MobileConsoleSheet.displayName = "MobileConsoleSheet"
