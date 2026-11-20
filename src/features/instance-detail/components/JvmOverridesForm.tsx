import { Cpu, Save } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

interface JvmOverridesFormProps {
	defaultArgs: string
}

const JvmOverridesForm = ({ defaultArgs }: JvmOverridesFormProps) => {
	const { t } = useTranslation()
	const globalRamGb = useLauncherPrefsStore((s) => {
		return s.ramGb
	})

	const [overrideRam, setOverrideRam] = useState(globalRamGb)
	const [customArgs, setCustomArgs] = useState(defaultArgs)
	const [savedNotice, setSavedNotice] = useState(false)

	const handleSave = (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault()
		setSavedNotice(true)
		window.setTimeout(() => {
			setSavedNotice(false)
		}, 1600)
	}

	return (
		<form
			onSubmit={handleSave}
			className="flex flex-col gap-4 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md"
		>
			<div className="flex items-center gap-2">
				<Cpu className="size-4 text-primary" />
				<h2 className="text-[13.5px] font-semibold text-foreground">
					{t("instanceDetail.jvm.title", { ram: overrideRam })}
				</h2>
			</div>

			<input
				type="range"
				min={2}
				max={32}
				step={1}
				value={overrideRam}
				onChange={(event) => {
					setOverrideRam(Number(event.target.value))
				}}
				className="w-full accent-primary"
			/>

			<label className="flex flex-col gap-1.5">
				<span className="text-[11.5px] font-medium text-foreground/80">
					{t("instanceDetail.jvm.customArgs")}
				</span>
				<input
					type="text"
					value={customArgs}
					onChange={(event) => {
						setCustomArgs(event.target.value)
					}}
					className="h-9 rounded-xl border border-border bg-background/80 px-3 font-mono text-xs text-foreground focus:border-primary/40 focus:outline-none"
				/>
			</label>

			<div className="flex items-center justify-end gap-2">
				{savedNotice ? (
					<span className="text-[11.5px] font-medium text-primary">
						{t("instanceDetail.jvm.savedNotice")}
					</span>
				) : null}
				<button
					type="submit"
					className="flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3.5 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90"
				>
					<Save className="size-3.5" />
					<span>{t("instanceDetail.jvm.saveOverrides")}</span>
				</button>
			</div>
		</form>
	)
}

export default JvmOverridesForm
