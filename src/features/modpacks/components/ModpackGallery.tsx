import { Camera } from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"

interface ModpackGalleryProps {
	packName: string
	accentClass: string
}

type GalleryLabelKey =
	| "modpacks.gallery.shaderPanorama"
	| "modpacks.gallery.worldSpawn"
	| "modpacks.gallery.telemetry"

const SLIDE_LABEL_KEYS: readonly GalleryLabelKey[] = [
	"modpacks.gallery.shaderPanorama",
	"modpacks.gallery.worldSpawn",
	"modpacks.gallery.telemetry",
]

const ModpackGallery = ({ packName, accentClass }: ModpackGalleryProps) => {
	const { t } = useTranslation()

	return (
		<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-3">
			{SLIDE_LABEL_KEYS.map((labelKey) => {
				return (
					<div
						key={labelKey}
						className={cn(
							"flex h-28 flex-col justify-between rounded-xl border border-border bg-linear-to-br p-3",
							accentClass,
						)}
					>
						<Camera className="size-4 opacity-80" />
						<div>
							<p className="truncate text-[11.5px] font-semibold text-foreground">{packName}</p>
							<p className="text-[10px] text-muted-foreground">{t(labelKey)}</p>
						</div>
					</div>
				)
			})}
		</div>
	)
}

export default ModpackGallery
