import { Cpu } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { Button } from "@/shared/ui/button"

const RAM_PRESETS = [4, 6, 8, 12, 16]

const JavaMemoryCard = () => {
	const { t } = useTranslation()
	const { ramGb, setRamGb } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				ramGb: s.ramGb,
				setRamGb: s.setRamGb,
			}
		}),
	)

	return (
		<section className="@container flex flex-col gap-4 rounded-2xl border border-border bg-card/65 p-3.5 backdrop-blur-md sm:p-4">
			<div className="flex items-center gap-2.5">
				<div className="flex size-7 shrink-0 items-center justify-center rounded-md border border-border bg-foreground/5 text-primary">
					<Cpu className="size-4" />
				</div>
				<h2 className="text-[13.5px] font-semibold text-foreground">
					{t("settings.java.title", { ramGb })}
				</h2>
			</div>

			<input
				type="range"
				min={2}
				max={32}
				step={1}
				value={ramGb}
				onChange={(event) => {
					setRamGb(Number(event.target.value))
				}}
				aria-label={t("settings.java.sliderAriaLabel")}
				className="w-full accent-primary"
			/>

			<div className="flex flex-wrap gap-2">
				{RAM_PRESETS.map((preset) => {
					const isSelected = ramGb === preset
					return (
						<Button
							key={preset}
							type="button"
							variant="outline"
							size="sm"
							onClick={() => {
								setRamGb(preset)
							}}
							className={cn(
								"cursor-pointer rounded-md text-[12px]",
								isSelected
									? "border-primary/45 bg-primary/18 text-primary hover:bg-primary/22 hover:text-primary"
									: "bg-foreground/5 text-foreground/80 hover:bg-foreground/10",
							)}
						>
							{preset} GB
						</Button>
					)
				})}
			</div>

			<div className="rounded-xl border border-border bg-foreground/4 p-3 font-mono text-[11px] leading-relaxed wrap-break-word text-muted-foreground">
				-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -Xms{Math.max(1, Math.floor(ramGb / 2))}G -Xmx
				{ramGb}G
			</div>
		</section>
	)
}

export default JavaMemoryCard
