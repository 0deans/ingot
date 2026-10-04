import { AlertTriangle, ArrowDownToLine, ArrowUpFromLine, ShieldAlert, XCircle } from "lucide-react"
import { useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import type { SyncConflictInfo } from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import {
	Item,
	ItemActions,
	ItemContent,
	ItemDescription,
	ItemMedia,
	ItemTitle,
} from "@/components/ui/item"
import { ScrollArea } from "@/components/ui/scroll-area"

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
			<DialogContent className="max-h-[90vh] w-full gap-3 p-4 sm:max-w-lg sm:p-5 md:max-w-xl">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2.5 text-warning">
						<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-warning/10 text-warning">
							<ShieldAlert className="size-5" />
						</div>
						<DialogTitle className="font-semibold text-base text-foreground sm:text-lg">
							{t("syncConflict.title")}
						</DialogTitle>
					</div>
					<DialogDescription className="text-foreground/80 text-xs leading-relaxed">
						<Trans
							i18nKey="syncConflict.description"
							values={{ name: conflict.instanceName }}
							components={{ b: <strong className="text-foreground" /> }}
						/>
					</DialogDescription>
				</DialogHeader>

				{/* Conflicting items pill list */}
				<Alert className={alertTone.warning}>
					<AlertTriangle />
					<AlertDescription className="flex flex-wrap items-center gap-1.5">
						<span className="text-2xs">{t("syncConflict.conflictingFiles")}</span>
						{conflict.hasInstanceOptions && conflict.hasSharedOptions && (
							<code className="rounded bg-warning/10 px-1.5 py-0.5 font-mono text-3xs text-warning">
								options.txt
							</code>
						)}
						{conflict.hasInstanceServers && conflict.hasSharedServers && (
							<code className="rounded bg-warning/10 px-1.5 py-0.5 font-mono text-3xs text-warning">
								servers.dat
							</code>
						)}
					</AlertDescription>
				</Alert>

				<div className="flex min-h-0 flex-1 flex-col gap-1.5">
					<div className="shrink-0 font-medium text-2xs text-muted-foreground">
						{t("syncConflict.choose")}
					</div>

					<ScrollArea className="scroll-fade-y -mr-2 max-h-[min(380px,50vh)] p-0.5 pr-2">
						<div className="flex flex-col gap-2">
							{/* Option 1: Use Shared Data */}
							<Item
								variant="outline"
								render={
									<button
										type="button"
										disabled={isProcessing}
										onClick={() => handleChoice("use_shared")}
									/>
								}
								className="items-start text-left hover:bg-muted/50 disabled:pointer-events-none disabled:opacity-50"
							>
								<ItemMedia variant="icon" className="text-info">
									<ArrowDownToLine />
								</ItemMedia>
								<ItemContent>
									<ItemTitle>{t("syncConflict.useShared")}</ItemTitle>
									<ItemDescription>
										<Trans
											i18nKey="syncConflict.useSharedDesc"
											components={{ code: <code className="text-foreground/80" /> }}
										/>
									</ItemDescription>
								</ItemContent>
								<ItemActions>
									<Badge variant="secondary">{t("syncConflict.overwritesLocal")}</Badge>
								</ItemActions>
							</Item>

							{/* Option 2: Use Instance as Master */}
							<Item
								variant="outline"
								render={
									<button
										type="button"
										disabled={isProcessing}
										onClick={() => handleChoice("use_instance")}
									/>
								}
								className="items-start text-left hover:bg-muted/50 disabled:pointer-events-none disabled:opacity-50"
							>
								<ItemMedia variant="icon" className="text-primary">
									<ArrowUpFromLine />
								</ItemMedia>
								<ItemContent>
									<ItemTitle>{t("syncConflict.useInstance")}</ItemTitle>
									<ItemDescription>
										<Trans
											i18nKey="syncConflict.useInstanceDesc"
											components={{ code: <code className="text-foreground/80" /> }}
										/>
									</ItemDescription>
								</ItemContent>
								<ItemActions>
									<Badge variant="secondary">{t("syncConflict.overwritesShared")}</Badge>
								</ItemActions>
							</Item>

							{/* Option 3: Disable Sync for this instance */}
							<Item
								variant="outline"
								render={
									<button
										type="button"
										disabled={isProcessing}
										onClick={() => handleChoice("disable_sync")}
									/>
								}
								className="items-start text-left hover:bg-muted/50 disabled:pointer-events-none disabled:opacity-50"
							>
								<ItemMedia variant="icon" className="text-muted-foreground">
									<XCircle />
								</ItemMedia>
								<ItemContent>
									<ItemTitle>{t("syncConflict.disable")}</ItemTitle>
									<ItemDescription>{t("syncConflict.disableDesc")}</ItemDescription>
								</ItemContent>
								<ItemActions>
									<Badge variant="secondary">{t("syncConflict.noChanges")}</Badge>
								</ItemActions>
							</Item>
						</div>
					</ScrollArea>
				</div>
			</DialogContent>
		</Dialog>
	)
}

export default SyncConflictDialog
