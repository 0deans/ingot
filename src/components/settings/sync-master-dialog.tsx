import { Check, CheckCircle2, Database, FolderX, Loader2, UploadCloud } from "lucide-react"
import { useState } from "react"
import LoaderIcon from "@/components/instances/loader-icon"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import { useInstances } from "@/services/instance-service"
import { settingsService } from "@/services/settings-service"

export interface InitialSyncCategoryTarget {
	key: string
	categoryName: string
	title: string
	file: string
	hasSharedData: boolean
}

export type SyncSourceChoice =
	| { type: "instance"; instanceId: string }
	| { type: "shared" }
	| { type: "empty" }

interface SyncMasterDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	category?: InitialSyncCategoryTarget | null
	onSelectSource?: (choice: SyncSourceChoice) => Promise<void> | void
	onCancel?: () => void
	onSuccess?: () => void
}

export const SyncMasterDialog = ({
	open,
	onOpenChange,
	category,
	onSelectSource,
	onCancel,
	onSuccess,
}: SyncMasterDialogProps) => {
	const { instances } = useInstances()
	const [selectedId, setSelectedId] = useState<string | null>(null)
	const [isSubmitting, setIsSubmitting] = useState(false)
	const [statusMessage, setStatusMessage] = useState<string | null>(null)

	const handleChoice = async (choice: SyncSourceChoice) => {
		if (choice.type === "instance") {
			setSelectedId(choice.instanceId)
		}
		setIsSubmitting(true)
		setStatusMessage(null)

		try {
			if (onSelectSource) {
				await onSelectSource(choice)
			} else if (choice.type === "instance") {
				const inst = instances.find((i) => i.id === choice.instanceId)
				const report = await settingsService.pushInstanceSync(choice.instanceId)
				setStatusMessage(
					report.message || `Set "${inst?.name || "Instance"}" as master sync source.`,
				)
				if (onSuccess) onSuccess()
			}
			onOpenChange(false)
			setSelectedId(null)
			setStatusMessage(null)
		} catch (error) {
			console.error("Failed sync choice:", error)
			setStatusMessage(`Error: ${error}`)
		} finally {
			setIsSubmitting(false)
		}
	}

	const handleOpenChange = (nextOpen: boolean) => {
		if (!nextOpen && !isSubmitting) {
			if (onCancel) {
				onCancel()
			}
		}
		onOpenChange(nextOpen)
	}

	const title = category ? `Initial Sync: ${category.title}` : "Select Master Sync Instance"
	const description = category
		? `Choose where your shared storage should initially get ${category.file} from:`
		: "Choose an existing instance to populate your shared sync storage. Its settings and data will become the shared baseline."

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent className="max-h-[90vh] w-full gap-3 border-border/60 bg-zinc-950 p-4 shadow-2xl backdrop-blur-2xl sm:max-w-lg sm:p-5">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2 text-emerald-400">
						<UploadCloud className="size-5" />
						<DialogTitle className="font-semibold text-lg text-white">{title}</DialogTitle>
					</div>
					<DialogDescription className="text-xs text-zinc-400 leading-relaxed">
						{description}
					</DialogDescription>
				</DialogHeader>

				<div className="flex flex-col gap-3 py-1">
					{/* Option 1: Use Current Shared Storage if available */}
					{category?.hasSharedData && (
						<div className="flex items-center justify-between gap-3 rounded-xl border border-emerald-500/30 bg-emerald-950/20 p-3 transition-colors hover:border-emerald-500/50">
							<div className="flex items-center gap-3">
								<div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-emerald-500/20 text-emerald-400">
									<Database className="size-4" />
								</div>
								<div>
									<div className="font-medium text-white text-xs">Use Existing Shared Data</div>
									<div className="text-[11px] text-zinc-400">
										Keep current{" "}
										<code className="rounded bg-zinc-900 px-1 py-0.5 font-mono text-[10px] text-emerald-300">
											{category.file}
										</code>{" "}
										in shared storage
									</div>
								</div>
							</div>

							<Button
								size="sm"
								disabled={isSubmitting}
								onClick={() => handleChoice({ type: "shared" })}
								className="h-8 shrink-0 bg-emerald-600 text-white text-xs hover:bg-emerald-500"
							>
								Use Shared
							</Button>
						</div>
					)}

					{/* Instances selection */}
					<div className="flex flex-col gap-1.5">
						<span className="font-medium text-[11px] text-zinc-400">
							{category?.hasSharedData
								? "Or copy from an existing instance:"
								: "Copy from an existing instance:"}
						</span>

						<ScrollArea className="max-h-[36vh] pr-2">
							<div className="flex flex-col gap-2 py-1">
								{instances.length === 0 ? (
									<div className="py-4 text-center text-muted-foreground text-xs">
										No instances found.
									</div>
								) : (
									instances.map((inst) => {
										const isSelected = selectedId === inst.id
										return (
											<div
												key={inst.id}
												className={`flex items-center justify-between rounded-xl border p-2.5 transition-colors ${
													isSelected
														? "border-emerald-500/50 bg-emerald-500/10"
														: "border-zinc-800/80 bg-zinc-900/40 hover:border-zinc-700 hover:bg-zinc-900/70"
												}`}
											>
												<div className="flex items-center gap-2.5">
													<LoaderIcon loader={inst.loader} size={20} />
													<div>
														<div className="font-medium text-foreground text-xs">{inst.name}</div>
														<div className="text-[10px] text-muted-foreground">
															MC {inst.gameVersion} • {inst.loader}
														</div>
													</div>
												</div>

												<Button
													size="sm"
													variant={isSelected ? "default" : "outline"}
													disabled={isSubmitting}
													onClick={() => handleChoice({ type: "instance", instanceId: inst.id })}
													className="h-7 gap-1 text-xs"
												>
													{isSubmitting && isSelected ? (
														<Loader2 className="size-3 animate-spin" />
													) : isSelected ? (
														<Check className="size-3" />
													) : null}
													{category ? "Use This" : "Set as Master"}
												</Button>
											</div>
										)
									})
								)}
							</div>
						</ScrollArea>
					</div>

					{/* Option 3: Start Empty */}
					{category && (
						<div className="flex items-center justify-between border-border/30 border-t pt-2">
							<span className="text-[11px] text-zinc-400">Don't want to copy any data yet?</span>
							<Button
								type="button"
								variant="ghost"
								size="sm"
								disabled={isSubmitting}
								onClick={() => handleChoice({ type: "empty" })}
								className="h-7 gap-1.5 text-muted-foreground text-xs hover:text-foreground"
							>
								<FolderX className="size-3.5" />
								Start Fresh / Empty
							</Button>
						</div>
					)}
				</div>

				{statusMessage && (
					<div className="flex items-center gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-2.5 text-emerald-300 text-xs">
						<CheckCircle2 className="size-4 shrink-0 text-emerald-400" />
						<span>{statusMessage}</span>
					</div>
				)}
			</DialogContent>
		</Dialog>
	)
}

export default SyncMasterDialog
