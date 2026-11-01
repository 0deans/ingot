import { openUrl } from "@tauri-apps/plugin-opener"
import { Calendar, ExternalLink, FileText, Plus } from "lucide-react"
import { motion } from "motion/react"
import { memo, useCallback } from "react"
import { useTranslation } from "react-i18next"
import type { PatchNoteSummary } from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { getMinecraftNetArticleUrl } from "@/services/news-service"

export interface PatchNoteCardProps {
	note: PatchNoteSummary
	onReadNotes: (note: PatchNoteSummary) => void
	onCreateInstance: (version: string) => void
}

const formatDate = (isoString: string): string => {
	try {
		const parsed = new Date(isoString)
		if (Number.isNaN(parsed.getTime())) {
			return isoString
		}
		return new Intl.DateTimeFormat(undefined, {
			year: "numeric",
			month: "short",
			day: "numeric",
		}).format(parsed)
	} catch {
		return isoString
	}
}

const PatchNoteCard = ({ note, onReadNotes, onCreateInstance }: PatchNoteCardProps) => {
	const { t } = useTranslation()
	const isRelease = note.patchType.toLowerCase() === "release"
	const formattedDate = formatDate(note.date)

	const handleRead = useCallback(() => {
		onReadNotes(note)
	}, [note, onReadNotes])

	const handleOpenExternal = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation()
			const url = getMinecraftNetArticleUrl(note.title, note.version)
			openUrl(url).catch((err) => {
				console.error("Failed to open Minecraft.net link:", err)
			})
		},
		[note.title, note.version],
	)

	const handleCreate = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation()
			onCreateInstance(note.version)
		},
		[note.version, onCreateInstance],
	)

	return (
		<motion.div
			layout
			initial={{ opacity: 0, y: 12 }}
			animate={{ opacity: 1, y: 0 }}
			transition={{ duration: 0.2 }}
			onClick={handleRead}
			className="group relative flex cursor-pointer flex-col overflow-hidden rounded-2xl border border-border/50 bg-card/60 transition-all duration-200 hover:border-primary/40 hover:bg-card/90 hover:shadow-lg"
		>
			{note.imageUrl && (
				<div className="relative aspect-video w-full overflow-hidden bg-muted/40">
					<img
						src={note.imageUrl}
						alt={note.title}
						loading="lazy"
						className="size-full object-cover transition-transform duration-300 group-hover:scale-105"
					/>
					<div className="absolute inset-0 bg-gradient-to-t from-background/90 via-background/20 to-transparent" />
				</div>
			)}

			<div className="flex flex-1 flex-col justify-between p-5">
				<div className="flex flex-col gap-2.5">
					<div className="flex items-center justify-between gap-2">
						<Badge
							variant="outline"
							className={
								isRelease
									? "border-emerald-500/30 bg-emerald-500/10 font-semibold text-emerald-500"
									: "border-amber-500/30 bg-amber-500/10 font-semibold text-amber-500"
							}
						>
							{isRelease ? t("news.badges.release") : t("news.badges.snapshot")}
						</Badge>
						<span className="flex items-center gap-1 text-xs text-muted-foreground">
							<Calendar className="size-3" />
							{formattedDate}
						</span>
					</div>

					<h2 className="line-clamp-2 text-base font-bold text-foreground transition-colors group-hover:text-primary">
						{note.title}
					</h2>

					{note.shortText && (
						<p className="line-clamp-3 text-xs leading-relaxed text-muted-foreground">
							{note.shortText}
						</p>
					)}
				</div>

				<div className="mt-4 flex items-center justify-between gap-1.5 border-t border-border/40 pt-3">
					<Button
						variant="ghost"
						size="sm"
						onClick={handleRead}
						className="h-8 max-w-[42%] min-w-0 gap-1.5 px-2 text-xs text-muted-foreground group-hover:text-foreground"
					>
						<FileText className="size-3.5 shrink-0" />
						<span className="truncate">{t("news.actions.readNotes")}</span>
					</Button>

					<div className="flex items-center gap-1.5">
						<Button
							variant="ghost"
							size="icon-sm"
							onClick={handleOpenExternal}
							title={t("news.actions.openExternal")}
							className="size-8 shrink-0 text-muted-foreground hover:text-foreground"
							aria-label={t("news.actions.openExternal")}
						>
							<ExternalLink className="size-3.5" />
						</Button>

						<Button
							variant="outline"
							size="sm"
							onClick={handleCreate}
							title={t("news.actions.createInstanceFull")}
							className="h-8 shrink-0 gap-1 px-2.5 text-xs font-medium hover:border-primary/50 hover:bg-primary/10 hover:text-primary"
						>
							<Plus className="size-3.5 shrink-0" />
							<span>{t("news.actions.createInstance")}</span>
						</Button>
					</div>
				</div>
			</div>
		</motion.div>
	)
}

PatchNoteCard.displayName = "PatchNoteCard"

export default memo(PatchNoteCard)
