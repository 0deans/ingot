import { Archive, Check, X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import type { WorldSave } from "@/features/worlds/components/WorldCard"

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

	const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault()
		setCreated(true)
		window.setTimeout(() => {
			setCreated(false)
			onClose()
		}, 900)
	}

	return (
		<AnimatePresence>
			{world ? (
				<div className="fixed inset-0 z-50 flex items-center justify-center p-4">
					<button
						type="button"
						aria-label={t("worlds.backupModal.closeAria")}
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-black/60 backdrop-blur-xs"
					/>
					<motion.form
						role="dialog"
						aria-label={t("worlds.backupModal.title")}
						initial={{ opacity: 0, scale: 0.96 }}
						animate={{ opacity: 1, scale: 1 }}
						exit={{ opacity: 0, scale: 0.96 }}
						transition={{ duration: 0.16, ease: [0.22, 1, 0.36, 1] }}
						onSubmit={handleSubmit}
						className="ingot-glass-elevated relative z-10 w-full max-w-md space-y-3 rounded-2xl p-4 shadow-2xl"
					>
						<div className="flex items-center justify-between border-b border-border pb-2.5">
							<div className="flex items-center gap-2">
								<Archive className="size-4 text-primary" />
								<h2 className="text-sm font-semibold text-foreground">
									{t("worlds.backupModal.title")}
								</h2>
							</div>
							<button
								type="button"
								onClick={onClose}
								aria-label={t("worlds.backupModal.dismissAria")}
								className="cursor-pointer rounded-lg p-1 text-muted-foreground hover:bg-foreground/8 hover:text-foreground"
							>
								<X className="size-4" />
							</button>
						</div>

						<p className="text-xs text-muted-foreground">
							{t("worlds.backupModal.descriptionPrefix")}{" "}
							<span className="font-medium text-foreground">{world.name}</span>{" "}
							{t("worlds.backupModal.descriptionSuffix", { sizeMb: world.sizeMb })}
						</p>

						<input
							type="text"
							value={label}
							onChange={(event) => {
								setLabel(event.target.value)
							}}
							aria-label={t("worlds.backupModal.labelAria")}
							className="h-9 w-full rounded-xl border border-border bg-background/80 px-3 text-xs text-foreground focus:border-primary/40 focus:outline-none"
						/>

						<div className="flex justify-end gap-2 pt-1">
							<button
								type="button"
								onClick={onClose}
								className="h-8 cursor-pointer rounded-xl border border-border bg-foreground/5 px-3 text-xs font-medium text-foreground/80 hover:bg-foreground/10"
							>
								{t("common.cancel")}
							</button>
							<button
								type="submit"
								className="flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3.5 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90"
							>
								{created ? <Check className="size-3.5" /> : <Archive className="size-3.5" />}
								<span>
									{created ? t("worlds.backupModal.saved") : t("worlds.backupModal.archiveNow")}
								</span>
							</button>
						</div>
					</motion.form>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default BackupSnapshotModal
