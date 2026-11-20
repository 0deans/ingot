import { X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect, useState, type ChangeEvent, type FormEvent } from "react"
import { useTranslation } from "react-i18next"
import type { CreateInstanceInput, InstanceEngine, TagColor } from "@/features/play/types"

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

	useEffect(() => {
		if (!open) {
			return
		}
		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				onClose()
			}
		}
		window.addEventListener("keydown", handleKeyDown)
		return () => {
			window.removeEventListener("keydown", handleKeyDown)
		}
	}, [onClose, open])

	const resetForm = () => {
		setName("")
		setEngine("Horizon (Rust)")
		setVersion(DEFAULTS_BY_ENGINE["Horizon (Rust)"].version)
	}

	const handleClose = () => {
		resetForm()
		onClose()
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
		<AnimatePresence>
			{open ? (
				<div className="fixed inset-0 z-50 flex items-center justify-center p-4">
					<motion.button
						type="button"
						aria-label={t("play.quickCreate.closeAria")}
						initial={{ opacity: 0 }}
						animate={{ opacity: 1 }}
						exit={{ opacity: 0 }}
						onClick={handleClose}
						className="fixed inset-0 cursor-default bg-black/60 backdrop-blur-xs"
					/>
					<motion.form
						role="dialog"
						aria-label={modalTitle}
						initial={{ opacity: 0, scale: 0.96, y: 8 }}
						animate={{ opacity: 1, scale: 1, y: 0 }}
						exit={{ opacity: 0, scale: 0.96, y: 8 }}
						transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
						onSubmit={handleSubmit}
						className="ingot-glass-elevated relative z-10 w-full max-w-lg space-y-4 rounded-2xl p-5 shadow-2xl"
					>
						<div className="flex items-center justify-between border-b border-border pb-3">
							<div>
								<h2 className="text-sm font-bold text-foreground">{modalTitle}</h2>
								<p className="text-[11px] text-muted-foreground">
									{t("play.quickCreate.subtitle")}
								</p>
							</div>
							<button
								type="button"
								aria-label={t("play.quickCreate.closeButtonAria")}
								onClick={handleClose}
								className="cursor-pointer rounded-lg p-1 text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
							>
								<X className="size-4" />
							</button>
						</div>

						<div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
							<label className="flex flex-col gap-1 sm:col-span-2">
								<span className="text-[11px] font-medium text-foreground/80">
									{instanceNameLabel}
								</span>
								<input
									type="text"
									aria-label={instanceNameLabel}
									value={name}
									onChange={(event) => {
										setName(event.target.value)
									}}
									placeholder={t("play.quickCreate.instanceNamePlaceholder")}
									className="h-9 rounded-xl border border-border bg-background/80 px-3 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary/50 focus:outline-none"
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
								<span className="text-[11px] font-medium text-foreground/80">
									{gameVersionLabel}
								</span>
								<input
									type="text"
									aria-label={gameVersionLabel}
									value={version}
									onChange={(event) => {
										setVersion(event.target.value)
									}}
									placeholder={t("play.quickCreate.gameVersionPlaceholder")}
									className="h-9 rounded-xl border border-border bg-background/80 px-3 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary/50 focus:outline-none"
								/>
							</label>
						</div>

						<div className="flex items-center justify-end gap-2 border-t border-border pt-3">
							<button
								type="button"
								onClick={handleClose}
								className="h-8 cursor-pointer rounded-xl border border-border bg-foreground/5 px-3.5 text-xs font-medium text-foreground/80 transition-colors hover:bg-foreground/10"
							>
								{t("common.cancel")}
							</button>
							<button
								type="submit"
								className="h-8 cursor-pointer rounded-xl bg-primary px-4 text-xs font-bold text-primary-foreground transition-opacity hover:opacity-90"
							>
								{t("play.quickCreate.createAndSelect")}
							</button>
						</div>
					</motion.form>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default QuickCreateModal
