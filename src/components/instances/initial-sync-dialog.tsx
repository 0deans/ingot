import { ArrowDownToLine, ArrowUpFromLine, RefreshCw } from "lucide-react"
import { useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"
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
	const { t } = useTranslation()
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
			<DialogContent className="max-w-md p-6">
				<DialogHeader className="gap-2">
					<div className="flex items-center gap-2.5 text-primary">
						<div className="flex size-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
							<RefreshCw className="size-5" />
						</div>
						<DialogTitle className="font-semibold text-foreground text-lg">
							{t("initialSync.title")}
						</DialogTitle>
					</div>
					<DialogDescription className="text-foreground/80 text-xs leading-relaxed">
						<Trans
							i18nKey="initialSync.description"
							values={{ name: instanceName }}
							components={{ b: <strong className="text-foreground" /> }}
						/>
					</DialogDescription>
				</DialogHeader>

				<div className="flex flex-col gap-3 py-2">
					{/* Option 1: From this instance */}
					<button
						type="button"
						disabled={isSubmitting}
						onClick={() => handleSelect("instance")}
						className="group flex flex-col items-start gap-1 rounded-xl border border-border bg-card/40 p-3.5 text-left transition-all hover:border-primary/50 hover:bg-primary/5 focus:outline-none"
					>
						<div className="flex w-full items-center justify-between">
							<span className="flex items-center gap-1.5 font-semibold text-foreground text-xs group-hover:text-primary">
								<ArrowUpFromLine className="size-3.5 text-primary" />
								{t("initialSync.fromInstance")}
							</span>
							<Badge variant="outline" className="border-primary/30 text-primary">
								{t("initialSync.uploadToShared")}
							</Badge>
						</div>
						<p className="text-[11px] text-muted-foreground leading-snug">
							{t("initialSync.fromInstanceDesc")}
						</p>
					</button>

					{/* Option 2: From shared storage */}
					<button
						type="button"
						disabled={isSubmitting}
						onClick={() => handleSelect("shared")}
						className="group flex flex-col items-start gap-1 rounded-xl border border-border bg-card/40 p-3.5 text-left transition-all hover:border-info/50 hover:bg-info/5 focus:outline-none"
					>
						<div className="flex w-full items-center justify-between">
							<span className="flex items-center gap-1.5 font-semibold text-foreground text-xs group-hover:text-info">
								<ArrowDownToLine className="size-3.5 text-info" />
								{t("initialSync.fromShared")}
							</span>
							<Badge variant="outline" className="border-info/30 text-info">
								{t("initialSync.downloadToInstance")}
							</Badge>
						</div>
						<p className="text-[11px] text-muted-foreground leading-snug">
							{t("initialSync.fromSharedDesc")}
						</p>
					</button>
				</div>
			</DialogContent>
		</Dialog>
	)
}

export default InitialSyncDialog
