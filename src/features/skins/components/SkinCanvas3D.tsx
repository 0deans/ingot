import { Shirt } from "lucide-react"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { cn } from "@/shared/lib/utils"

export type ArmModel = "classic" | "slim"

interface SkinCanvas3DProps {
	skinName: string
	gradient: string
	armModel: ArmModel
	selectedCape: string
	capes: readonly string[]
	onArmModelChange: (model: ArmModel) => void
	onSelectCape: (cape: string) => void
}

const MODEL_OPTIONS: SegmentedOption<ArmModel>[] = [
	{ value: "classic", label: "Classic (4px)" },
	{ value: "slim", label: "Slim (3px)" },
]

const SkinCanvas3D = ({
	skinName,
	gradient,
	armModel,
	selectedCape,
	capes,
	onArmModelChange,
	onSelectCape,
}: SkinCanvas3DProps) => {
	return (
		<section className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div
				className={cn(
					"flex h-52 flex-col items-center justify-center rounded-xl border border-white/10 bg-linear-to-b",
					gradient,
				)}
			>
				<div className="flex size-16 items-center justify-center rounded-2xl border border-white/20 bg-black/35 text-white shadow-lg">
					<Shirt className="size-8 text-emerald-300" />
				</div>
				<p className="mt-3 text-[13.5px] font-semibold text-white">{skinName}</p>
				<p className="text-[11px] text-zinc-200">
					{armModel === "classic" ? "Classic 4px" : "Slim 3px"} · Cape: {selectedCape}
				</p>
			</div>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-medium text-muted-foreground">Arm Geometry</span>
				<SegmentedControl
					layoutId="skins-arm-model"
					options={MODEL_OPTIONS}
					value={armModel}
					onChange={onArmModelChange}
					tone="emerald"
				/>
			</div>

			<div className="flex flex-col gap-1.5">
				<span className="text-[11px] font-medium text-muted-foreground">Equipped Cape</span>
				<div className="grid grid-cols-2 gap-1.5">
					{capes.map((cape) => {
						const active = cape === selectedCape
						return (
							<button
								key={cape}
								type="button"
								onClick={() => {
									onSelectCape(cape)
								}}
								className={cn(
									"cursor-pointer rounded-lg border px-2.5 py-1.5 text-[11px] font-medium transition-colors",
									active
										? "border-primary/45 bg-primary/18 text-primary"
										: "border-border bg-foreground/4 text-foreground/80 hover:bg-foreground/8",
								)}
							>
								{cape}
							</button>
						)
					})}
				</div>
			</div>
		</section>
	)
}

export default SkinCanvas3D
