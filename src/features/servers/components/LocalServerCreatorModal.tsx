import { Server, X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import type { ServerEntry } from "@/features/servers/components/ServerRow"

interface LocalServerCreatorModalProps {
	open: boolean
	onClose: () => void
	onCreateServer: (server: ServerEntry) => void
}

const LocalServerCreatorModal = ({
	open,
	onClose,
	onCreateServer,
}: LocalServerCreatorModalProps) => {
	const { t } = useTranslation()

	const [name, setName] = useState("")
	const [address, setAddress] = useState("127.0.0.1:25565")
	const [version, setVersion] = useState("1.21.4")

	const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault()
		const cleanName = name.trim() || t("servers.creatorModal.defaultName")
		const cleanAddress = address.trim() || "127.0.0.1:25565"
		onCreateServer({
			id: `srv-${Date.now()}`,
			name: cleanName,
			address: cleanAddress,
			motd: t("servers.creatorModal.defaultMotd"),
			playersOnline: 0,
			playersMax: 20,
			pingMs: 1,
			version: version.trim() || "1.21.4",
			instanceName: "Fabulously Optimized",
			engine: "Java",
		})
		setName("")
		onClose()
	}

	const modalTitle = t("servers.creatorModal.title")

	return (
		<AnimatePresence>
			{open ? (
				<div className="fixed inset-0 z-50 flex items-center justify-center p-4">
					<button
						type="button"
						aria-label={t("servers.creatorModal.closeAria")}
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-black/60 backdrop-blur-xs"
					/>
					<motion.form
						role="dialog"
						aria-label={modalTitle}
						initial={{ opacity: 0, scale: 0.96 }}
						animate={{ opacity: 1, scale: 1 }}
						exit={{ opacity: 0, scale: 0.96 }}
						transition={{ duration: 0.16, ease: [0.22, 1, 0.36, 1] }}
						onSubmit={handleSubmit}
						className="ingot-glass-elevated relative z-10 w-full max-w-md space-y-3 rounded-2xl p-4 shadow-2xl"
					>
						<div className="flex items-center justify-between border-b border-border pb-2.5">
							<div className="flex items-center gap-2">
								<Server className="size-4 text-primary" />
								<h2 className="text-sm font-semibold text-foreground">{modalTitle}</h2>
							</div>
							<button
								type="button"
								onClick={onClose}
								aria-label={t("servers.creatorModal.dismissAria")}
								className="cursor-pointer rounded-lg p-1 text-muted-foreground hover:bg-foreground/8 hover:text-foreground"
							>
								<X className="size-4" />
							</button>
						</div>

						<input
							type="text"
							value={name}
							onChange={(event) => {
								setName(event.target.value)
							}}
							placeholder={t("servers.creatorModal.namePlaceholder")}
							className="h-9 w-full rounded-xl border border-border bg-background/80 px-3 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary/40 focus:outline-none"
						/>
						<div className="grid grid-cols-2 gap-2">
							<input
								type="text"
								value={address}
								onChange={(event) => {
									setAddress(event.target.value)
								}}
								placeholder={t("servers.creatorModal.hostPlaceholder")}
								className="h-9 rounded-xl border border-border bg-background/80 px-3 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary/40 focus:outline-none"
							/>
							<input
								type="text"
								value={version}
								onChange={(event) => {
									setVersion(event.target.value)
								}}
								placeholder={t("servers.creatorModal.versionPlaceholder")}
								className="h-9 rounded-xl border border-border bg-background/80 px-3 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary/40 focus:outline-none"
							/>
						</div>

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
								className="h-8 cursor-pointer rounded-xl bg-primary px-4 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90"
							>
								{t("servers.creatorModal.saveServer")}
							</button>
						</div>
					</motion.form>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default LocalServerCreatorModal
