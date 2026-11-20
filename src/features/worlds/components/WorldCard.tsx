import { Archive, Globe2, HardDrive, Play } from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"
import { formatRelativeLastPlayed, type InstanceEngine } from "@/shared/stores/useInstancesStore"

export interface WorldSave {
	id: string
	name: string
	instanceName: string
	version: string
	loader: string
	engine: InstanceEngine
	mode: "Survival" | "Creative" | "Hardcore"
	seed: string
	sizeMb: number
	lastPlayed: string
}

interface WorldCardProps {
	world: WorldSave
	onPlay: (world: WorldSave) => void
	onBackup: (world: WorldSave) => void
}

type WorldModeLabelKey = "worlds.modes.survival" | "worlds.modes.creative" | "worlds.modes.hardcore"

const MODE_LABEL_KEYS: Record<WorldSave["mode"], WorldModeLabelKey> = {
	Survival: "worlds.modes.survival",
	Creative: "worlds.modes.creative",
	Hardcore: "worlds.modes.hardcore",
}

const MODE_BADGE_CLASS: Record<WorldSave["mode"], string> = {
	Survival: "bg-primary/15 text-primary border-primary/30",
	Creative: "bg-sky-500/15 text-sky-700 dark:text-sky-300 border-sky-500/30",
	Hardcore: "bg-rose-500/15 text-rose-700 dark:text-rose-300 border-rose-500/30",
}

const WorldCard = ({ world, onPlay, onBackup }: WorldCardProps) => {
	const { t } = useTranslation()

	return (
		<article className="flex flex-col justify-between gap-3 rounded-2xl border border-border bg-card/65 p-4 transition-colors hover:border-foreground/20 hover:bg-card">
			<div className="flex items-start justify-between gap-3">
				<div className="flex min-w-0 items-center gap-3">
					<div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-primary/15 text-primary">
						<Globe2 className="size-5" />
					</div>
					<div className="min-w-0">
						<h2 className="truncate text-[14px] font-semibold text-foreground">{world.name}</h2>
						<p className="truncate text-[11.5px] text-muted-foreground">
							{world.instanceName} · {world.version}
						</p>
					</div>
				</div>

				<span
					className={cn(
						"shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-semibold",
						MODE_BADGE_CLASS[world.mode],
					)}
				>
					{t(MODE_LABEL_KEYS[world.mode])}
				</span>
			</div>

			<div className="flex flex-wrap items-center justify-between gap-2 border-t border-border pt-3 text-[11px] text-muted-foreground">
				<div className="flex flex-wrap items-center gap-3">
					<span className="font-mono text-muted-foreground">
						{t("worlds.seed", { seed: world.seed })}
					</span>
					<span className="flex items-center gap-1">
						<HardDrive className="size-3 text-muted-foreground" />
						<span>{world.sizeMb} MB</span>
					</span>
					<span>{formatRelativeLastPlayed(world.lastPlayed, t)}</span>
				</div>

				<div className="flex items-center gap-1.5">
					<button
						type="button"
						onClick={() => {
							onBackup(world)
						}}
						className="flex h-7 cursor-pointer items-center gap-1 rounded-lg border border-border bg-foreground/6 px-2.5 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						<Archive className="size-3" />
						<span>{t("worlds.snapshot")}</span>
					</button>
					<button
						type="button"
						onClick={() => {
							onPlay(world)
						}}
						className="flex h-7 cursor-pointer items-center gap-1.5 rounded-lg bg-primary px-3 text-[11.5px] font-semibold text-primary-foreground transition-opacity hover:opacity-90"
					>
						<Play className="size-3 fill-current" />
						<span>{t("worlds.playWorld")}</span>
					</button>
				</div>
			</div>
		</article>
	)
}

export default WorldCard
