import { Archive, Check } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import type { WorldSave } from "@/features/worlds/components/WorldCard"
import { Button } from "@/shared/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/shared/ui/dialog"
import { Input } from "@/shared/ui/input"

interface BackupSnapshotModalProps {
	world: WorldSave | null
	onClose: () => void
}

const BackupSnapshotModal = ({ world, onClose }: BackupSnapshotModalProps) => {
	const { t } = useTranslation()

	const [label, setLabel] = useState(() => {
		return t("worlds.backupModal.defaultLabel")
	})
	const [created, setCreated] = useState(false)

	const handleOpenChange = (nextOpen: boolean) => {
		if (!nextOpen) {
			onClose()
		}
	}

	const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault()
		setCreated(true)
		window.setTimeout(() => {
			setCreated(false)
			onClose()
		}, 900)
	}

	const modalTitle = t("worlds.backupModal.title")

	return (
		<Dialog open={world !== null} onOpenChange={handleOpenChange}>
			<DialogContent
				aria-label={modalTitle}
				className="ingot-glass-elevated rounded-2xl p-4 shadow-2xl sm:max-w-md"
			>
				<DialogHeader className="border-b border-border pb-2.5">
					<DialogTitle className="flex items-center gap-2 text-sm font-semibold text-foreground">
						<Archive className="size-4 text-primary" />
						<span>{modalTitle}</span>
					</DialogTitle>
					{world ? (
						<DialogDescription className="text-xs text-muted-foreground">
							{t("worlds.backupModal.descriptionPrefix")}{" "}
							<span className="font-medium text-foreground">{world.name}</span>{" "}
							{t("worlds.backupModal.descriptionSuffix", { sizeMb: world.sizeMb })}
						</DialogDescription>
					) : null}
				</DialogHeader>

				<form onSubmit={handleSubmit} className="space-y-3">
					<Input
						type="text"
						value={label}
						onChange={(event) => {
							setLabel(event.target.value)
						}}
						aria-label={t("worlds.backupModal.labelAria")}
						className="h-9 rounded-xl border-border bg-background/80 px-3 text-xs md:text-xs"
					/>

					<div className="flex justify-end gap-2 pt-1">
						<Button
							type="button"
							variant="outline"
							onClick={onClose}
							className="cursor-pointer rounded-xl bg-foreground/5 px-3 text-xs hover:bg-foreground/10"
						>
							{t("common.cancel")}
						</Button>
						<Button
							type="submit"
							className="cursor-pointer rounded-xl px-3.5 text-xs font-semibold"
						>
							{created ? <Check className="size-3.5" /> : <Archive className="size-3.5" />}
							<span>
								{created ? t("worlds.backupModal.saved") : t("worlds.backupModal.archiveNow")}
							</span>
						</Button>
					</div>
				</form>
			</DialogContent>
		</Dialog>
	)
}

export default BackupSnapshotModal
