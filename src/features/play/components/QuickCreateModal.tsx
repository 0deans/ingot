import { useState, type ChangeEvent, type FormEvent } from "react"
import { useTranslation } from "react-i18next"
import type { CreateInstanceInput, InstanceEngine, TagColor } from "@/features/play/types"
import { Button } from "@/shared/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/shared/ui/dialog"
import { Input } from "@/shared/ui/input"

interface QuickCreateModalProps {
	open: boolean
	onClose: () => void
	onCreate: (input: CreateInstanceInput) => void
}

interface EngineDefaults {
	loader: string
	version: string
	colorTag: TagColor
}

const DEFAULTS_BY_ENGINE: Record<InstanceEngine, EngineDefaults> = {
	"Horizon (Rust)": { loader: "Horizon Native", version: "1.21.4-rs", colorTag: "rose" },
	Java: { loader: "Fabric", version: "1.21.4", colorTag: "emerald" },
	Bedrock: { loader: "UWP / GDK", version: "1.21.50", colorTag: "cyan" },
}

const isInstanceEngine = (value: string): value is InstanceEngine => {
	return value === "Java" || value === "Horizon (Rust)" || value === "Bedrock"
}

const QuickCreateModal = ({ open, onClose, onCreate }: QuickCreateModalProps) => {
	const { t } = useTranslation()

	const [name, setName] = useState("")
	const [engine, setEngine] = useState<InstanceEngine>("Horizon (Rust)")
	const [version, setVersion] = useState(DEFAULTS_BY_ENGINE["Horizon (Rust)"].version)

	const resetForm = () => {
		setName("")
		setEngine("Horizon (Rust)")
		setVersion(DEFAULTS_BY_ENGINE["Horizon (Rust)"].version)
	}

	const handleClose = () => {
		resetForm()
		onClose()
	}

	const handleOpenChange = (nextOpen: boolean) => {
		if (!nextOpen) {
			handleClose()
		}
	}

	const handleEngineChange = (event: ChangeEvent<HTMLSelectElement>) => {
		const next = event.target.value
		if (isInstanceEngine(next)) {
			setEngine(next)
			setVersion(DEFAULTS_BY_ENGINE[next].version)
		}
	}

	const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault()
		const defaults = DEFAULTS_BY_ENGINE[engine]
		onCreate({
			name,
			engine,
			loader: defaults.loader,
			version: version.trim() || defaults.version,
			colorTag: defaults.colorTag,
		})
		resetForm()
		onClose()
	}

	const modalTitle = t("play.quickCreate.title")
	const instanceNameLabel = t("play.quickCreate.instanceName")
	const engineLabel = t("play.quickCreate.engine")
	const gameVersionLabel = t("play.quickCreate.gameVersion")

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent
				aria-label={modalTitle}
				className="ingot-glass-elevated rounded-2xl p-5 shadow-2xl sm:max-w-lg"
			>
				<DialogHeader className="border-b border-border pb-3">
					<DialogTitle className="text-sm font-bold text-foreground">{modalTitle}</DialogTitle>
					<DialogDescription className="text-[11px] text-muted-foreground">
						{t("play.quickCreate.subtitle")}
					</DialogDescription>
				</DialogHeader>

				<form onSubmit={handleSubmit} className="space-y-4">
					<div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
						<label className="flex flex-col gap-1 sm:col-span-2">
							<span className="text-[11px] font-medium text-foreground/80">
								{instanceNameLabel}
							</span>
							<Input
								type="text"
								aria-label={instanceNameLabel}
								value={name}
								onChange={(event) => {
									setName(event.target.value)
								}}
								placeholder={t("play.quickCreate.instanceNamePlaceholder")}
								className="h-9 rounded-xl border-border bg-background/80 px-3 text-xs md:text-xs"
							/>
						</label>

						<label className="flex flex-col gap-1">
							<span className="text-[11px] font-medium text-foreground/80">{engineLabel}</span>
							<select
								aria-label={engineLabel}
								value={engine}
								onChange={handleEngineChange}
								className="h-9 rounded-xl border border-border bg-background px-2.5 text-xs text-foreground focus:border-primary/50 focus:outline-none"
							>
								<option value="Horizon (Rust)">{t("play.quickCreate.engineHorizon")}</option>
								<option value="Java">{t("play.quickCreate.engineJava")}</option>
								<option value="Bedrock">{t("play.quickCreate.engineBedrock")}</option>
							</select>
						</label>

						<label className="flex flex-col gap-1">
							<span className="text-[11px] font-medium text-foreground/80">{gameVersionLabel}</span>
							<Input
								type="text"
								aria-label={gameVersionLabel}
								value={version}
								onChange={(event) => {
									setVersion(event.target.value)
								}}
								placeholder={t("play.quickCreate.gameVersionPlaceholder")}
								className="h-9 rounded-xl border-border bg-background/80 px-3 text-xs md:text-xs"
							/>
						</label>
					</div>

					<div className="flex items-center justify-end gap-2 border-t border-border pt-3">
						<Button
							type="button"
							variant="outline"
							onClick={handleClose}
							className="cursor-pointer rounded-xl bg-foreground/5 px-3.5 text-xs hover:bg-foreground/10"
						>
							{t("common.cancel")}
						</Button>
						<Button type="submit" className="cursor-pointer rounded-xl px-4 text-xs font-bold">
							{t("play.quickCreate.createAndSelect")}
						</Button>
					</div>
				</form>
			</DialogContent>
		</Dialog>
	)
}

export default QuickCreateModal
