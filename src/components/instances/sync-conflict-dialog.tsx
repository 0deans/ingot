import { AlertTriangle, ArrowDownToLine, ArrowUpFromLine, ShieldAlert, XCircle } from "lucide-react"
import { useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import type { SyncConflictInfo } from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/common/dialog"
import { ScrollArea } from "@/components/common/scroll-area"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"

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
	const { t } = useTranslation()
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
			<DialogContent className="max-h-[90vh] w-full gap-3 border-border/60 bg-zinc-950 p-4 shadow-2xl backdrop-blur-2xl sm:max-w-lg sm:p-5 md:max-w-xl">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2.5 text-amber-400">
						<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-amber-500/10 text-amber-400">
							<ShieldAlert className="size-5" />
						</div>
						<DialogTitle className="font-semibold text-base text-zinc-50 sm:text-lg">
							{t("syncConflict.title")}
						</DialogTitle>
					</div>
					<DialogDescription className="text-xs text-zinc-300 leading-relaxed">
						<Trans
							i18nKey="syncConflict.description"
							values={{ name: conflict.instanceName }}
							components={{ b: <strong className="text-zinc-50" /> }}
						/>
					</DialogDescription>
				</DialogHeader>

				{/* Conflicting items pill list */}
				<Alert className={alertTone.warning}>
					<AlertTriangle />
					<AlertDescription className="flex flex-wrap items-center gap-1.5">
						<span className="text-[11px]">{t("syncConflict.conflictingFiles")}</span>
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
					</AlertDescription>
				</Alert>

				<div className="flex min-h-0 flex-1 flex-col gap-1.5">
					<div className="shrink-0 font-medium text-[11px] text-muted-foreground">
						{t("syncConflict.choose")}
					</div>

					<ScrollArea scrollFade className="-mr-2 max-h-[min(380px,50vh)] p-0.5 pr-2">
						<div className="flex flex-col gap-2">
							{/* Option 1: Use Shared Data */}
							<button
								type="button"
								disabled={isProcessing}
								onClick={() => handleChoice("use_shared")}
								className="group flex cursor-pointer flex-col items-start gap-1 rounded-xl border border-zinc-800/80 bg-zinc-900/40 p-3 text-left transition-all hover:border-sky-500/50 hover:bg-sky-500/5 focus:outline-none sm:p-3.5"
							>
								<div className="flex w-full items-center justify-between gap-2">
									<span className="flex items-center gap-2 font-semibold text-foreground text-xs group-hover:text-sky-400 sm:text-sm">
										<ArrowDownToLine className="size-4 shrink-0 text-sky-400" />
										<span>{t("syncConflict.useShared")}</span>
									</span>
									<Badge variant="secondary">{t("syncConflict.overwritesLocal")}</Badge>
								</div>
								<p className="pl-6 text-[11px] text-muted-foreground leading-relaxed">
									<Trans
										i18nKey="syncConflict.useSharedDesc"
										components={{ code: <code className="text-zinc-300" /> }}
									/>
								</p>
							</button>

							{/* Option 2: Use Instance as Master */}
							<button
								type="button"
								disabled={isProcessing}
								onClick={() => handleChoice("use_instance")}
								className="group flex cursor-pointer flex-col items-start gap-1 rounded-xl border border-zinc-800/80 bg-zinc-900/40 p-3 text-left transition-all hover:border-emerald-500/50 hover:bg-emerald-500/5 focus:outline-none sm:p-3.5"
							>
								<div className="flex w-full items-center justify-between gap-2">
									<span className="flex items-center gap-2 font-semibold text-foreground text-xs group-hover:text-emerald-400 sm:text-sm">
										<ArrowUpFromLine className="size-4 shrink-0 text-emerald-400" />
										<span>{t("syncConflict.useInstance")}</span>
									</span>
									<Badge variant="secondary">{t("syncConflict.overwritesShared")}</Badge>
								</div>
								<p className="pl-6 text-[11px] text-muted-foreground leading-relaxed">
									<Trans
										i18nKey="syncConflict.useInstanceDesc"
										components={{ code: <code className="text-zinc-300" /> }}
									/>
								</p>
							</button>

							{/* Option 3: Disable Sync for this instance */}
							<button
								type="button"
								disabled={isProcessing}
								onClick={() => handleChoice("disable_sync")}
								className="group flex cursor-pointer flex-col items-start gap-1 rounded-xl border border-zinc-800/80 bg-zinc-900/40 p-3 text-left transition-all hover:border-zinc-700 hover:bg-zinc-800/30 focus:outline-none sm:p-3.5"
							>
								<div className="flex w-full items-center justify-between gap-2">
									<span className="flex items-center gap-2 font-semibold text-foreground text-xs group-hover:text-zinc-200 sm:text-sm">
										<XCircle className="size-4 shrink-0 text-zinc-400" />
										<span>{t("syncConflict.disable")}</span>
									</span>
									<Badge variant="secondary">{t("syncConflict.noChanges")}</Badge>
								</div>
								<p className="pl-6 text-[11px] text-muted-foreground leading-relaxed">
									{t("syncConflict.disableDesc")}
								</p>
							</button>
						</div>
					</ScrollArea>
				</div>
			</DialogContent>
		</Dialog>
	)
}

export default SyncConflictDialog
