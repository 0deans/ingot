import { Server } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import type { ServerEntry } from "@/features/servers/components/ServerRow"
import { Button } from "@/shared/ui/button"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/shared/ui/dialog"
import { Input } from "@/shared/ui/input"

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

	const handleOpenChange = (nextOpen: boolean) => {
		if (!nextOpen) {
			onClose()
		}
	}

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
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent
				aria-label={modalTitle}
				className="ingot-glass-elevated rounded-2xl p-4 shadow-2xl sm:max-w-md"
			>
				<DialogHeader className="border-b border-border pb-2.5">
					<DialogTitle className="flex items-center gap-2 text-sm font-semibold text-foreground">
						<Server className="size-4 text-primary" />
						<span>{modalTitle}</span>
					</DialogTitle>
				</DialogHeader>

				<form onSubmit={handleSubmit} className="space-y-3">
					<Input
						type="text"
						value={name}
						onChange={(event) => {
							setName(event.target.value)
						}}
						placeholder={t("servers.creatorModal.namePlaceholder")}
						className="h-9 rounded-xl border-border bg-background/80 px-3 text-xs md:text-xs"
					/>
					<div className="grid grid-cols-2 gap-2">
						<Input
							type="text"
							value={address}
							onChange={(event) => {
								setAddress(event.target.value)
							}}
							placeholder={t("servers.creatorModal.hostPlaceholder")}
							className="h-9 rounded-xl border-border bg-background/80 px-3 text-xs md:text-xs"
						/>
						<Input
							type="text"
							value={version}
							onChange={(event) => {
								setVersion(event.target.value)
							}}
							placeholder={t("servers.creatorModal.versionPlaceholder")}
							className="h-9 rounded-xl border-border bg-background/80 px-3 text-xs md:text-xs"
						/>
					</div>

					<div className="flex justify-end gap-2 pt-1">
						<Button
							type="button"
							variant="outline"
							onClick={onClose}
							className="cursor-pointer rounded-xl bg-foreground/5 px-3 text-xs hover:bg-foreground/10"
						>
							{t("common.cancel")}
						</Button>
						<Button type="submit" className="cursor-pointer rounded-xl px-4 text-xs font-semibold">
							{t("servers.creatorModal.saveServer")}
						</Button>
					</div>
				</form>
			</DialogContent>
		</Dialog>
	)
}

export default LocalServerCreatorModal
