import { AlertTriangle, ArrowDownToLine, ArrowUpFromLine, ShieldAlert, XCircle } from "lucide-react"
import { useState } from "react"
import type { SyncConflictInfo } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"

interface SyncConflictDialogProps {
	conflict: SyncConflictInfo | null
	open: boolean
	onResolve: (resolution: "use_shared" | "use_instance" | "disable_sync") => Promise<void> | void
	onCancel: () => void
}

export const SyncConflictDialog = ({
	conflict,
	open,
	onResolve,
	onCancel,
}: SyncConflictDialogProps) => {
	const [isProcessing, setIsProcessing] = useState(false)

	if (!conflict) return null

	const handleChoice = async (resolution: "use_shared" | "use_instance" | "disable_sync") => {
		setIsProcessing(true)
		try {
			await onResolve(resolution)
		} finally {
			setIsProcessing(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={(isOpen) => !isOpen && onCancel()}>
			<DialogContent className="max-w-lg border-border/60 bg-zinc-950 p-6 shadow-2xl backdrop-blur-2xl">
				<DialogHeader className="gap-2">
					<div className="flex items-center gap-2.5 text-amber-400">
						<div className="flex size-9 items-center justify-center rounded-lg bg-amber-500/10 text-amber-400">
							<ShieldAlert className="size-5" />
						</div>
						<DialogTitle className="font-semibold text-lg text-white">
							Sync Conflict Detected
						</DialogTitle>
					</div>
					<DialogDescription className="text-xs text-zinc-300 leading-relaxed">
						<strong className="text-white">{conflict.instanceName}</strong> contains its own local
						configuration files that have never been synchronized with your shared storage.
					</DialogDescription>
				</DialogHeader>

				{/* Conflicting items pill list */}
				<div className="flex flex-wrap items-center gap-1.5 rounded-lg border border-amber-500/20 bg-amber-500/5 p-2.5 text-amber-300 text-xs">
					<AlertTriangle className="size-3.5 shrink-0" />
					<span className="text-[11px]">Conflicting files:</span>
					{conflict.hasInstanceOptions && conflict.hasSharedOptions && (
						<code className="rounded bg-amber-500/10 px-1.5 py-0.5 font-mono text-[10px] text-amber-200">
							options.txt
						</code>
					)}
					{conflict.hasInstanceServers && conflict.hasSharedServers && (
						<code className="rounded bg-amber-500/10 px-1.5 py-0.5 font-mono text-[10px] text-amber-200">
							servers.dat
						</code>
					)}
				</div>

				<div className="flex flex-col gap-2.5 py-1">
					<div className="font-medium text-[11px] text-muted-foreground">
						Choose how to resolve this initial synchronization:
					</div>

					{/* Option 1: Use Shared Data */}
					<button
						type="button"
						disabled={isProcessing}
						onClick={() => handleChoice("use_shared")}
						className="group flex flex-col items-start gap-1 rounded-xl border border-zinc-800 bg-zinc-900/40 p-3.5 text-left transition-all hover:border-sky-500/50 hover:bg-sky-500/5 focus:outline-none"
					>
						<div className="flex w-full items-center justify-between">
							<span className="flex items-center gap-1.5 font-semibold text-foreground text-xs group-hover:text-sky-400">
								<ArrowDownToLine className="size-3.5 text-sky-400" />
								Use Shared Settings
							</span>
							<span className="rounded bg-zinc-800/80 px-1.5 py-0.5 text-[10px] text-zinc-400">
								Overwrites local
							</span>
						</div>
						<p className="text-[11px] text-muted-foreground leading-snug">
							Replace this instance's options and servers with existing shared settings. An
							automatic <code className="text-zinc-300">.bak</code> backup of your instance files
							will be saved.
						</p>
					</button>

					{/* Option 2: Use Instance as Master */}
					<button
						type="button"
						disabled={isProcessing}
						onClick={() => handleChoice("use_instance")}
						className="group flex flex-col items-start gap-1 rounded-xl border border-zinc-800 bg-zinc-900/40 p-3.5 text-left transition-all hover:border-emerald-500/50 hover:bg-emerald-500/5 focus:outline-none"
					>
						<div className="flex w-full items-center justify-between">
							<span className="flex items-center gap-1.5 font-semibold text-foreground text-xs group-hover:text-emerald-400">
								<ArrowUpFromLine className="size-3.5 text-emerald-400" />
								Make This Instance the Shared Master
							</span>
							<span className="rounded bg-zinc-800/80 px-1.5 py-0.5 text-[10px] text-zinc-400">
								Overwrites shared
							</span>
						</div>
						<p className="text-[11px] text-muted-foreground leading-snug">
							Push this instance's options and servers into shared storage as the master copy.
							Existing shared files will be backed up as <code className="text-zinc-300">.bak</code>
							.
						</p>
					</button>

					{/* Option 3: Disable Sync for this instance */}
					<button
						type="button"
						disabled={isProcessing}
						onClick={() => handleChoice("disable_sync")}
						className="group flex flex-col items-start gap-1 rounded-xl border border-zinc-800 bg-zinc-900/40 p-3.5 text-left transition-all hover:border-zinc-700 hover:bg-zinc-800/30 focus:outline-none"
					>
						<div className="flex w-full items-center justify-between">
							<span className="flex items-center gap-1.5 font-semibold text-foreground text-xs group-hover:text-zinc-200">
								<XCircle className="size-3.5 text-zinc-400" />
								Keep Independent (Disable Sync)
							</span>
							<span className="rounded bg-zinc-800/80 px-1.5 py-0.5 text-[10px] text-zinc-400">
								No changes
							</span>
						</div>
						<p className="text-[11px] text-muted-foreground leading-snug">
							Disables synchronization on this instance so its servers and settings will never be
							altered or replaced by other instances.
						</p>
					</button>
				</div>

				<DialogFooter className="mt-2 flex items-center justify-end border-border/40 border-t pt-3">
					<Button
						type="button"
						variant="ghost"
						size="sm"
						onClick={onCancel}
						disabled={isProcessing}
						className="text-muted-foreground text-xs hover:text-foreground"
					>
						Cancel Launch
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

export default SyncConflictDialog
