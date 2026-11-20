import { Cpu, Save } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { Button } from "@/shared/ui/button"
import { Input } from "@/shared/ui/input"

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
				<Input
					type="text"
					value={customArgs}
					onChange={(event) => {
						setCustomArgs(event.target.value)
					}}
					className="h-9 rounded-xl border-border bg-background/80 px-3 font-mono text-xs md:text-xs"
				/>
			</label>

			<div className="flex items-center justify-end gap-2">
				{savedNotice ? (
					<span className="text-[11.5px] font-medium text-primary">
						{t("instanceDetail.jvm.savedNotice")}
					</span>
				) : null}
				<Button type="submit" className="cursor-pointer rounded-xl px-3.5 text-xs font-semibold">
					<Save className="size-3.5" />
					<span>{t("instanceDetail.jvm.saveOverrides")}</span>
				</Button>
			</div>
		</form>
	)
}

export default JvmOverridesForm
