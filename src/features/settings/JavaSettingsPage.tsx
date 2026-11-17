import { Cpu } from "lucide-react"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

const RAM_PRESETS = [4, 6, 8, 12, 16]

const JavaSettingsPage = () => {
	const { ramGb, setRamGb } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				ramGb: s.ramGb,
				setRamGb: s.setRamGb,
			}
		}),
	)

	return (
		<section className="flex flex-col gap-4 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="flex items-center gap-2">
				<Cpu className="size-4 text-primary" />
				<h2 className="text-[13.5px] font-semibold text-foreground">
					Default Memory Allocation ({ramGb} GB)
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
				aria-label="Allocated RAM in GB"
				className="w-full accent-primary"
			/>

			<div className="flex flex-wrap gap-2">
				{RAM_PRESETS.map((preset) => {
					const isSelected = ramGb === preset
					return (
						<button
							key={preset}
							type="button"
							onClick={() => {
								setRamGb(preset)
							}}
							className={cn(
								"cursor-pointer rounded-lg border px-3 py-1.5 text-[12px] font-medium transition-colors",
								isSelected
									? "border-primary/45 bg-primary/18 text-primary"
									: "border-border bg-foreground/5 text-foreground/80 hover:bg-foreground/10",
							)}
						>
							{preset} GB
						</button>
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

export default JavaSettingsPage
