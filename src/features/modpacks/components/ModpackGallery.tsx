import { Camera } from "lucide-react"
import { cn } from "@/shared/lib/utils"

interface ModpackGalleryProps {
	packName: string
	accentClass: string
}

const SLIDE_LABELS = ["Shader Panorama", "World Spawn Showcase", "Performance Telemetry"]

const ModpackGallery = ({ packName, accentClass }: ModpackGalleryProps) => {
	return (
		<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-3">
			{SLIDE_LABELS.map((label) => {
				return (
					<div
						key={label}
						className={cn(
							"flex h-28 flex-col justify-between rounded-xl border border-border bg-linear-to-br p-3",
							accentClass,
						)}
					>
						<Camera className="size-4 opacity-80" />
						<div>
							<p className="truncate text-[11.5px] font-semibold text-foreground">{packName}</p>
							<p className="text-[10px] text-muted-foreground">{label}</p>
						</div>
					</div>
				)
			})}
		</div>
	)
}

export default ModpackGallery
