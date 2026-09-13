import { ArrowDownToLine, ArrowUpFromLine, RefreshCw } from "lucide-react"
import { useState } from "react"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"

interface InitialSyncDialogProps {
	instanceName: string
	open: boolean
	onChoose: (source: "instance" | "shared") => Promise<void> | void
	onCancel: () => void
}

export const InitialSyncDialog = ({
	instanceName,
	open,
	onChoose,
	onCancel,
}: InitialSyncDialogProps) => {
	const [isSubmitting, setIsSubmitting] = useState(false)

	const handleSelect = async (source: "instance" | "shared") => {
		setIsSubmitting(true)
		try {
			await onChoose(source)
		} finally {
			setIsSubmitting(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={(isOpen) => !isOpen && onCancel()}>
			<DialogContent className="max-w-md border-border/60 bg-zinc-950 p-6 shadow-2xl backdrop-blur-2xl">
				<DialogHeader className="gap-2">
					<div className="flex items-center gap-2.5 text-emerald-400">
						<div className="flex size-9 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-400">
							<RefreshCw className="size-5" />
						</div>
						<DialogTitle className="font-semibold text-lg text-white">
							Initial Synchronization
						</DialogTitle>
					</div>
					<DialogDescription className="text-xs text-zinc-300 leading-relaxed">
						You are enabling synchronization for{" "}
						<strong className="text-white">{instanceName}</strong>. From where should we initially
						sync?
					</DialogDescription>
				</DialogHeader>

				<div className="flex flex-col gap-3 py-2">
					{/* Option 1: From this instance */}
					<button
						type="button"
						disabled={isSubmitting}
						onClick={() => handleSelect("instance")}
						className="group flex flex-col items-start gap-1 rounded-xl border border-zinc-800 bg-zinc-900/40 p-3.5 text-left transition-all hover:border-emerald-500/50 hover:bg-emerald-500/5 focus:outline-none"
					>
						<div className="flex w-full items-center justify-between">
							<span className="flex items-center gap-1.5 font-semibold text-foreground text-xs group-hover:text-emerald-400">
								<ArrowUpFromLine className="size-3.5 text-emerald-400" />
								From this instance
							</span>
							<span className="rounded bg-emerald-500/10 px-1.5 py-0.5 font-medium text-[10px] text-emerald-400">
								Upload to shared
							</span>
						</div>
						<p className="text-[11px] text-muted-foreground leading-snug">
							Copy this instance's options and servers to shared storage as the master copy.
						</p>
					</button>

					{/* Option 2: From shared storage */}
					<button
						type="button"
						disabled={isSubmitting}
						onClick={() => handleSelect("shared")}
						className="group flex flex-col items-start gap-1 rounded-xl border border-zinc-800 bg-zinc-900/40 p-3.5 text-left transition-all hover:border-sky-500/50 hover:bg-sky-500/5 focus:outline-none"
					>
						<div className="flex w-full items-center justify-between">
							<span className="flex items-center gap-1.5 font-semibold text-foreground text-xs group-hover:text-sky-400">
								<ArrowDownToLine className="size-3.5 text-sky-400" />
								From shared storage
							</span>
							<span className="rounded bg-sky-500/10 px-1.5 py-0.5 font-medium text-[10px] text-sky-400">
								Download to instance
							</span>
						</div>
						<p className="text-[11px] text-muted-foreground leading-snug">
							Download existing shared options and servers to this instance.
						</p>
					</button>
				</div>
			</DialogContent>
		</Dialog>
	)
}

export default InitialSyncDialog
