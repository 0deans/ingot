import { Play } from "lucide-react"
import { memo } from "react"
import { Button } from "@/components/ui/button"

interface InstanceHeroProps {
	name: string
	version: string
	loader: string
	memory: string
	javaVersion: string
	onPlay?: () => void
}

const InstanceHero = ({
	name,
	version,
	loader,
	memory,
	javaVersion,
	onPlay,
}: InstanceHeroProps) => {
	return (
		<div className="mt-6 flex flex-1 flex-col justify-between rounded-2xl border border-border/40 bg-gradient-to-br from-zinc-900/80 via-zinc-900/40 to-zinc-950/90 p-8 shadow-2xl backdrop-blur-md">
			<div className="flex flex-col gap-2">
				<div className="flex items-center gap-2">
					<span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2.5 py-0.5 font-medium text-emerald-400 text-xs">
						<span className="size-1.5 animate-pulse rounded-full bg-emerald-400" />
						Ready to Play
					</span>
					<span className="text-muted-foreground text-xs">
						{loader} • {version}
					</span>
				</div>

				<h1 className="font-bold text-3xl text-white tracking-tight sm:text-4xl">{name}</h1>
				<p className="max-w-xl text-muted-foreground text-sm leading-relaxed">
					Fast, lightweight Minecraft launcher built with Tauri and React. High-performance gaming
					environment powered by Ingot.
				</p>
			</div>

			{/* Bottom Action Bar */}
			<div className="mt-8 flex items-center justify-between border-border/30 border-t pt-6">
				<div className="flex items-center gap-6">
					<div>
						<div className="text-muted-foreground text-xs uppercase tracking-wider">Memory</div>
						<div className="font-semibold text-foreground text-sm">{memory}</div>
					</div>
					<div className="h-8 w-px bg-border/40" />
					<div>
						<div className="text-muted-foreground text-xs uppercase tracking-wider">
							Java Runtime
						</div>
						<div className="font-semibold text-foreground text-sm">{javaVersion}</div>
					</div>
				</div>

				<Button
					size="lg"
					onClick={onPlay}
					className="gap-2.5 px-8 font-semibold text-base shadow-lg shadow-primary/20 transition-all hover:scale-[1.02] active:scale-[0.98]"
				>
					<Play className="size-5 fill-current" />
					Play
				</Button>
			</div>
		</div>
	)
}

InstanceHero.displayName = "InstanceHero"

export default memo(InstanceHero)
