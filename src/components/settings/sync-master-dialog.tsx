import { Check, CheckCircle2, Database, FolderX, UploadCloud } from "lucide-react"
import { useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import { alertTone } from "@/components/common/alert-tones"
import LoaderIcon from "@/components/instances/loader-icon"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
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
	const { t } = useTranslation()
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
				setStatusMessage(report.message || t("syncMaster.setAsSource", { name: inst?.name ?? "" }))
				if (onSuccess) onSuccess()
			}
			onOpenChange(false)
			setSelectedId(null)
			setStatusMessage(null)
		} catch (error) {
			console.error("Failed sync choice:", error)
			setStatusMessage(t("syncMaster.error", { error: String(error) }))
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

	const title = category
		? t("syncMaster.categoryTitle", { title: category.title })
		: t("syncMaster.title")
	const description = category
		? t("syncMaster.categoryDescription", { file: category.file })
		: t("syncMaster.description")

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent className="max-h-[90vh] w-full gap-3 p-4 sm:max-w-lg sm:p-5">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2 text-primary">
						<UploadCloud className="size-5" />
						<DialogTitle className="font-semibold text-foreground text-lg">{title}</DialogTitle>
					</div>
					<DialogDescription className="text-muted-foreground text-xs leading-relaxed">
						{description}
					</DialogDescription>
				</DialogHeader>

				<div className="flex flex-col gap-3 py-1">
					{/* Option 1: Use Current Shared Storage if available */}
					{category?.hasSharedData && (
						<div className="flex items-center justify-between gap-3 rounded-xl border border-primary/30 bg-primary/10 p-3 transition-colors hover:border-primary/50">
							<div className="flex items-center gap-3">
								<div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/20 text-primary">
									<Database className="size-4" />
								</div>
								<div>
									<div className="font-medium text-foreground text-xs">
										{t("syncMaster.useExisting")}
									</div>
									<div className="text-2xs text-muted-foreground">
										<Trans
											i18nKey="syncMaster.keepCurrent"
											values={{ file: category.file }}
											components={{
												code: (
													<code className="rounded bg-card px-1 py-0.5 font-mono text-3xs text-primary" />
												),
											}}
										/>
									</div>
								</div>
							</div>

							<Button
								size="sm"
								disabled={isSubmitting}
								onClick={() => handleChoice({ type: "shared" })}
								className="h-8 shrink-0 text-xs"
							>
								{t("syncMaster.useShared")}
							</Button>
						</div>
					)}

					{/* Instances selection */}
					<div className="flex flex-col gap-1.5">
						<span className="font-medium text-2xs text-muted-foreground">
							{category?.hasSharedData ? t("syncMaster.orCopyFrom") : t("syncMaster.copyFrom")}
						</span>

						<ScrollArea className="scroll-fade-y max-h-[36vh] pr-2">
							<div className="flex flex-col gap-2 py-1">
								{instances.length === 0 ? (
									<div className="py-4 text-center text-muted-foreground text-xs">
										{t("syncMaster.noInstances")}
									</div>
								) : (
									instances.map((inst) => {
										const isSelected = selectedId === inst.id
										return (
											<div
												key={inst.id}
												className={`flex items-center justify-between rounded-xl border p-2.5 transition-colors ${
													isSelected
														? "border-primary/50 bg-primary/10"
														: "border-border/80 bg-card/40 hover:border-input hover:bg-card/70"
												}`}
											>
												<div className="flex items-center gap-2.5">
													<LoaderIcon loader={inst.loader} size={20} />
													<div>
														<div className="font-medium text-foreground text-xs">{inst.name}</div>
														<div className="text-3xs text-muted-foreground">
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
														<Spinner className="size-3" />
													) : isSelected ? (
														<Check className="size-3" />
													) : null}
													{category ? t("syncMaster.useThis") : t("syncMaster.setAsMaster")}
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
							<span className="text-2xs text-muted-foreground">{t("syncMaster.noCopy")}</span>
							<Button
								type="button"
								variant="ghost"
								size="sm"
								disabled={isSubmitting}
								onClick={() => handleChoice({ type: "empty" })}
								className="h-7 gap-1.5 text-muted-foreground text-xs"
							>
								<FolderX className="size-3.5" />
								{t("syncMaster.startFresh")}
							</Button>
						</div>
					)}
				</div>

				{statusMessage && (
					<Alert className={alertTone.success}>
						<CheckCircle2 />
						<AlertDescription>{statusMessage}</AlertDescription>
					</Alert>
				)}
			</DialogContent>
		</Dialog>
	)
}

export default SyncMasterDialog
