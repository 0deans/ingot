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
import {
	Item,
	ItemActions,
	ItemContent,
	ItemDescription,
	ItemMedia,
	ItemTitle,
} from "@/components/ui/item"

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
					<Item
						variant="outline"
						render={
							<button
								type="button"
								disabled={isSubmitting}
								onClick={() => handleSelect("instance")}
							/>
						}
						className="items-start text-left hover:bg-muted/50 disabled:pointer-events-none disabled:opacity-50"
					>
						<ItemMedia variant="icon" className="text-primary">
							<ArrowUpFromLine />
						</ItemMedia>
						<ItemContent>
							<ItemTitle>{t("initialSync.fromInstance")}</ItemTitle>
							<ItemDescription>{t("initialSync.fromInstanceDesc")}</ItemDescription>
						</ItemContent>
						<ItemActions>
							<Badge variant="outline" className="border-primary/30 text-primary">
								{t("initialSync.uploadToShared")}
							</Badge>
						</ItemActions>
					</Item>

					{/* Option 2: From shared storage */}
					<Item
						variant="outline"
						render={
							<button
								type="button"
								disabled={isSubmitting}
								onClick={() => handleSelect("shared")}
							/>
						}
						className="items-start text-left hover:bg-muted/50 disabled:pointer-events-none disabled:opacity-50"
					>
						<ItemMedia variant="icon" className="text-info">
							<ArrowDownToLine />
						</ItemMedia>
						<ItemContent>
							<ItemTitle>{t("initialSync.fromShared")}</ItemTitle>
							<ItemDescription>{t("initialSync.fromSharedDesc")}</ItemDescription>
						</ItemContent>
						<ItemActions>
							<Badge variant="outline" className="border-info/30 text-info">
								{t("initialSync.downloadToInstance")}
							</Badge>
						</ItemActions>
					</Item>
				</div>
			</DialogContent>
		</Dialog>
	)
}

export default InitialSyncDialog
